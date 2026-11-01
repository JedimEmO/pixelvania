//! Lounges: where the player rests, saves, and wakes after a blackout.
//!
//! A lounge is a room entity, `lounge 8 21 id=bench`, its position the middle of the chaise's feet.
//! It is a velvet chaise beside a standing lamp. Pressing up beside it rests:
//! health and poise come back, a hangover lifts, the game saves,
//! and this is where the player wakes after the next blackout.
//!
//! A save holds the progress and the last lounge, in `save.ron` in the platform's data folder
//! (`~/.local/share/pixelvania` on Linux), or in the browser's local storage on the web.
//! The game starts from it when it exists, unless a testing switch picks the start instead
//! (`PIXELVANIA_ROOM`, `PIXELVANIA_SPAWN` or `PIXELVANIA_PROGRESS`),
//! or `PIXELVANIA_FRESH=1` asks for a new game.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use serde::{Deserialize, Serialize};
use world::Progress;

use crate::combat::Fighting;
use crate::input::TickControls;
use crate::player::Player;
use crate::progress::PlayerProgress;
use crate::render::{GLOW_LAYER, Light2d};
use crate::rooms::{ActiveRoom, RoomPart};

pub struct LoungePlugin;

impl Plugin for LoungePlugin {
    fn build(&self, app: &mut App) {
        let save = startup_save();
        app.insert_resource(LastLounge(save.as_ref().map(|s| s.lounge.clone())))
            .add_systems(Update, (spawn, flicker, wake_at_start, show_prompt))
            .add_systems(
                FixedUpdate,
                rest.in_set(crate::input::TickSet::Simulate)
                    .after(crate::combat::SimulateBodies)
                    .after(crate::story::Talk),
            );
    }
}

/// A lounge in a room: the room's name and the lounge's id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoungeRef {
    pub room: String,
    pub id: String,
}

/// The lounge the player last rested at, where a blackout wakes them.
#[derive(Resource, Debug, Clone, Default)]
pub struct LastLounge(pub Option<LoungeRef>);

/// What a save holds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveFile {
    pub progress: Progress,
    pub lounge: LoungeRef,
    #[serde(default)]
    pub nectar: f32,
    #[serde(default)]
    pub tolerance: f32,
}

/// Where the save is kept: a file on the desktop, the browser's local storage on the web.
#[cfg(not(target_arch = "wasm32"))]
mod store {
    fn path() -> Option<std::path::PathBuf> {
        Some(dirs::data_dir()?.join("pixelvania").join("save.ron"))
    }

    pub fn place() -> String {
        path().map_or("nowhere".to_owned(), |p| p.display().to_string())
    }

    pub fn read() -> Option<String> {
        std::fs::read_to_string(path()?).ok()
    }

    pub fn write(text: &str) -> Result<(), String> {
        let path = path().ok_or("there is no data folder to save in")?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, text).map_err(|e| e.to_string())
    }

    pub fn delete() {
        if let Some(path) = path() {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod store {
    const KEY: &str = "pixelvania.save";

    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }

    pub fn place() -> String {
        format!("local storage `{KEY}`")
    }

    pub fn read() -> Option<String> {
        storage()?.get_item(KEY).ok()?
    }

    pub fn write(text: &str) -> Result<(), String> {
        storage()
            .ok_or("the browser has no local storage")?
            .set_item(KEY, text)
            .map_err(|e| format!("{e:?}"))
    }

    pub fn delete() {
        if let Some(s) = storage() {
            let _ = s.remove_item(KEY);
        }
    }
}

/// Deletes the save, so the next start is a new game.
pub fn delete_save() {
    store::delete();
}

/// Before the player has rested anywhere, a blackout wakes them where the game began:
/// on the start room's lounge, once they have been in that room.
fn wake_at_start(active: Res<ActiveRoom>, mut last: ResMut<LastLounge>) {
    if last.0.is_some() {
        return;
    }
    let Some(room) = &active.room else { return };
    if room.name != crate::rooms::world().start {
        return;
    }
    if let Some(id) = room
        .entities_of("lounge")
        .find_map(|d| d.props.get("id").cloned())
    {
        last.0 = Some(LoungeRef {
            room: room.name.clone(),
            id,
        });
    }
}

/// Whether a testing switch has picked how the game starts, so a save is not loaded.
fn testing_start() -> bool {
    [
        "PIXELVANIA_ROOM",
        "PIXELVANIA_SPAWN",
        "PIXELVANIA_PROGRESS",
        "PIXELVANIA_FRESH",
    ]
    .iter()
    .any(|v| std::env::var(v).is_ok())
}

/// The save to start from, if there is one and no testing switch says otherwise.
pub fn startup_save() -> Option<SaveFile> {
    if testing_start() {
        return None;
    }
    let text = store::read()?;
    match ron::from_str(&text) {
        Ok(save) => Some(save),
        Err(e) => {
            warn!("cannot read the save in {}: {e}", store::place());
            None
        }
    }
}

/// Saves the progress, the lounge and the Nectar. Returns false, with a warning, if it failed.
pub fn save(progress: &Progress, lounge: &LoungeRef, ind: &crate::indulge::Indulgence) -> bool {
    let file = SaveFile {
        progress: progress.clone(),
        lounge: lounge.clone(),
        nectar: ind.nectar,
        tolerance: ind.tolerance,
    };
    match write_save(&file) {
        Ok(()) => true,
        Err(e) => {
            warn!("saving failed: {e}");
            false
        }
    }
}

fn write_save(save: &SaveFile) -> Result<(), String> {
    let text = ron::ser::to_string_pretty(save, ron::ser::PrettyConfig::default())
        .map_err(|e| e.to_string())?;
    store::write(&text)
}

/// A lounge in the current room.
#[derive(Component)]
struct Lounge {
    id: String,
    /// The middle of the chaise's feet, in pixels with y down.
    feet: (f32, f32),
    lamp: Handle<Image>,
    /// Brightens the lamp for a moment after resting.
    flare: f32,
}

/// The room build the lounges were last spawned for.
#[derive(Default)]
struct SpawnedFor(Option<u64>);

/// Where a lounge with this id stands in a room, in pixels with y down.
pub fn lounge_feet(room: &world::Room, id: &str) -> Option<(f32, f32)> {
    room.entities_of("lounge")
        .find(|d| d.props.get("id").map(String::as_str) == Some(id))
        .map(|d| d.pixel_pos())
}

fn spawn(
    mut commands: Commands,
    active: Res<ActiveRoom>,
    mut spawned: Local<SpawnedFor>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(room) = &active.room else { return };
    if spawned.0 == Some(active.build) {
        return;
    }
    spawned.0 = Some(active.build);
    let image = |(w, h, data): (u32, u32, Vec<u8>), images: &mut Assets<Image>| {
        images.add(Image::new(
            bevy::render::render_resource::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            data,
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::MAIN_WORLD
                | bevy::asset::RenderAssetUsages::RENDER_WORLD,
        ))
    };
    let raw = |img: &assetgen::furniture::Piece, glow: bool| {
        let i = if glow { &img.glow } else { &img.color };
        (i.width(), i.height(), i.as_raw().clone())
    };
    for def in room.entities_of("lounge") {
        let Some(id) = def.props.get("id") else {
            error!(
                "room `{}`, line {}: a lounge needs an `id`",
                room.name, def.line
            );
            continue;
        };
        let feet = def.pixel_pos();
        let chaise = assetgen::furniture::chaise();
        let lamp = assetgen::furniture::lamp(0);
        let lamp_glow = image(raw(&lamp, true), &mut images);
        let at = Vec2::new(feet.0, -feet.1).round();
        commands.spawn((
            RoomPart,
            Name::new(format!("lounge {id}")),
            Lounge {
                id: id.clone(),
                feet,
                lamp: lamp_glow.clone(),
                flare: 0.0,
            },
            Transform::from_translation(at.extend(0.3)),
            Visibility::default(),
            children![
                (
                    Sprite::from_image(image(raw(&chaise, false), &mut images)),
                    Anchor::BOTTOM_CENTER,
                    Transform::default(),
                ),
                (
                    Sprite::from_image(image(raw(&lamp, false), &mut images)),
                    Anchor::BOTTOM_CENTER,
                    Transform::from_xyz(26.0, 0.0, 0.0),
                ),
                (
                    Sprite::from_image(lamp_glow),
                    Anchor::BOTTOM_CENTER,
                    Transform::from_xyz(26.0, 0.0, 0.0),
                    RenderLayers::layer(GLOW_LAYER),
                ),
                (
                    Light2d {
                        color: LinearRgba::rgb(1.0, 0.62, 0.36),
                        radius: 110.0,
                        strength: 1.2,
                        glow: 0.5,
                    },
                    Transform::from_xyz(26.0, 32.0, 0.0),
                ),
                crate::prompt::Prompt::bundle(
                    crate::prompt::Control::Up,
                    "Rest",
                    Vec2::new(0.0, 44.0)
                ),
            ],
        ));
    }
}

/// Flickers each lamp's flame and fades the flare after a rest.
fn flicker(
    time: Res<Time>,
    mut lounges: Query<(&mut Lounge, &Children)>,
    mut lights: Query<&mut Light2d>,
    mut images: ResMut<Assets<Image>>,
    mut last_step: Local<u32>,
) {
    let step = (time.elapsed_secs() * 12.0) as u32;
    let new_step = step != *last_step;
    *last_step = step;
    for (mut l, children) in &mut lounges {
        l.flare = (l.flare - time.delta_secs() * 1.5).max(0.0);
        for &c in children {
            if let Ok(mut light) = lights.get_mut(c) {
                light.strength = 1.2 * (1.0 + 0.6 * l.flare);
            }
        }
        if new_step && let Some(mut img) = images.get_mut(&l.lamp) {
            img.data = Some(assetgen::furniture::lamp(step).glow.into_raw());
        }
    }
}

/// Whether the player stands close enough to rest at this lounge.
fn near(p: &Player, l: &Lounge) -> bool {
    let b = &p.ctrl.body;
    let feet = ((b.x + b.w / 2) as f32, (b.y + b.h) as f32);
    p.ctrl.grounded && (feet.0 - l.feet.0).abs() < 28.0 && (feet.1 - l.feet.1).abs() < 4.0
}

/// Shows the Rest prompt when pressing up would rest.
fn show_prompt(
    players: Query<&Player>,
    lounges: Query<(&Lounge, &Children)>,
    mut prompts: Query<&mut Visibility, With<crate::prompt::Prompt>>,
    dialogue: Res<crate::story::Dialogue>,
    menu: Res<crate::levels::LoungeMenu>,
    near_talk: Res<crate::story::NearTalk>,
) {
    let player = players.single().ok();
    for (l, children) in &lounges {
        let show = player.is_some_and(|p| near(p, l))
            && !crate::story::talking(&dialogue)
            && !menu.wanted
            && !near_talk.0;
        for &c in children {
            if let Ok(mut vis) = prompts.get_mut(c) {
                vis.set_if_neq(if show {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
            }
        }
    }
}

/// Rests at a lounge when the player presses up beside it.
#[allow(clippy::too_many_arguments)]
fn rest(
    mut commands: Commands,
    input: Res<TickControls>,
    active: Res<ActiveRoom>,
    progress: Res<PlayerProgress>,
    mut last: ResMut<LastLounge>,
    mut indulgence: ResMut<crate::indulge::Indulgence>,
    mut menu: ResMut<crate::levels::LoungeMenu>,
    dialogue: Res<crate::story::Dialogue>,
    mut lounges: Query<&mut Lounge>,
    mut players: Query<(&Player, &mut Fighting)>,
    mut was_up: Local<bool>,
    mut sfx: MessageWriter<crate::audio::PlaySfx>,
) {
    let up = input.0.y > 0.6;
    let pressed = up && !*was_up;
    *was_up = up;
    if !pressed || crate::story::talking(&dialogue) {
        return;
    }
    for (p, mut f) in &mut players {
        for mut l in &mut lounges {
            if !near(p, &l) {
                continue;
            }
            f.combat.health = f.max_health;
            f.combat.poise = f.combat.max_poise;
            indulgence.rest();
            l.flare = 1.0;
            let lounge = LoungeRef {
                room: active.name.clone(),
                id: l.id.clone(),
            };
            last.0 = Some(lounge.clone());
            if !save(&progress.0, &lounge, &indulgence) {
                commands.spawn(crate::progress::Banner::bundle_text(
                    "RESTED",
                    "The night could not be saved",
                ));
            }
            menu.wanted = true;
            sfx.write(crate::audio::PlaySfx {
                pitch: 0.75,
                ..crate::audio::PlaySfx::new(crate::audio::Sfx::LureCatch)
            });
            info!("rested at `{}` in `{}`", l.id, active.name);
        }
    }
}
