//! Lounges: where the player rests, saves, and wakes after a blackout.
//!
//! A lounge is a room entity, `lounge 8 21 id=bench`, its position the middle of the chaise's feet.
//! It is a velvet chaise beside a standing lamp. Pressing up beside it rests:
//! health and poise come back, a hangover lifts, the game saves,
//! and this is where the player wakes after the next blackout.
//!
//! A save holds the progress and the last lounge, in `save.ron` in the platform's data folder
//! (`~/.local/share/pixelvania` on Linux). The game starts from it when it exists,
//! unless a testing switch picks the start instead (`PIXELVANIA_ROOM`, `PIXELVANIA_SPAWN`
//! or `PIXELVANIA_PROGRESS`), or `PIXELVANIA_FRESH=1` asks for a new game.
//! The web build does not save yet.

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
            .add_systems(Update, (spawn, flicker))
            .add_systems(
                FixedUpdate,
                rest.in_set(crate::input::TickSet::Simulate)
                    .after(crate::combat::SimulateBodies),
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

#[cfg(not(target_arch = "wasm32"))]
fn save_path() -> Option<std::path::PathBuf> {
    Some(dirs::data_dir()?.join("pixelvania").join("save.ron"))
}

#[cfg(target_arch = "wasm32")]
fn save_path() -> Option<std::path::PathBuf> {
    None
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
    let path = save_path()?;
    let text = std::fs::read_to_string(&path).ok()?;
    match ron::from_str(&text) {
        Ok(save) => Some(save),
        Err(e) => {
            warn!("cannot read the save at {}: {e}", path.display());
            None
        }
    }
}

fn write_save(save: &SaveFile) -> Result<(), String> {
    let path = save_path().ok_or("saving is not supported here")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = ron::ser::to_string_pretty(save, ron::ser::PrettyConfig::default())
        .map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| e.to_string())
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

/// Rests at a lounge when the player presses up beside it.
#[allow(clippy::too_many_arguments)]
fn rest(
    mut commands: Commands,
    input: Res<TickControls>,
    active: Res<ActiveRoom>,
    progress: Res<PlayerProgress>,
    mut last: ResMut<LastLounge>,
    mut indulgence: ResMut<crate::indulge::Indulgence>,
    mut lounges: Query<&mut Lounge>,
    mut players: Query<(&Player, &mut Fighting)>,
    mut was_up: Local<bool>,
    mut sfx: MessageWriter<crate::audio::PlaySfx>,
) {
    let up = input.0.y > 0.6;
    let pressed = up && !*was_up;
    *was_up = up;
    if !pressed {
        return;
    }
    for (p, mut f) in &mut players {
        let b = &p.ctrl.body;
        let feet = ((b.x + b.w / 2) as f32, (b.y + b.h) as f32);
        for mut l in &mut lounges {
            let near = (feet.0 - l.feet.0).abs() < 28.0 && (feet.1 - l.feet.1).abs() < 4.0;
            if !near || !p.ctrl.grounded {
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
            let save = SaveFile {
                progress: progress.0.clone(),
                lounge,
                nectar: indulgence.nectar,
                tolerance: indulgence.tolerance,
            };
            let hint = match write_save(&save) {
                Ok(()) => "The night is saved here",
                Err(e) => {
                    warn!("saving failed: {e}");
                    "The night could not be saved"
                }
            };
            commands.spawn(crate::progress::Banner::bundle_text("RESTED", hint));
            sfx.write(crate::audio::PlaySfx {
                pitch: 0.75,
                ..crate::audio::PlaySfx::new(crate::audio::Sfx::LureCatch)
            });
            info!("rested at `{}` in `{}`", l.id, active.name);
        }
    }
}
