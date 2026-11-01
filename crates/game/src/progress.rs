//! What the player has found, and the pickups in rooms that give it.
//!
//! A pickup is a room entity, `pickup 7 7.5 item=candelabrum`, drawn as a small glowing diamond
//! that bobs on the animation's steps: gold for an ability, pink for a weapon.
//! Touching it takes it for good, with a burst, a sound, and a banner naming what it gives.
//! Taken pickups are kept in [`PlayerProgress`] and never come back.
//!
//! A new game starts with nothing but the sabre; the slice gives the rest.
//! `PIXELVANIA_PROGRESS=slice` starts with what a player has on reaching the ballroom
//! in the full game (the Lunge, Cling, the Swoon, the Lure, the sabre and the Jeroboam),
//! and `PIXELVANIA_PROGRESS=all` with everything.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use pixelfx::effects::EffectId;
use world::{Item, Progress};

use crate::effects::SpawnFx;
use crate::player::Player;
use crate::render::{CameraFocus, GLOW_LAYER, HEIGHT, Light2d};
use crate::rooms::{ActiveRoom, RoomPart};

pub struct ProgressPlugin;

impl Plugin for ProgressPlugin {
    fn build(&self, app: &mut App) {
        let progress = match std::env::var("PIXELVANIA_PROGRESS").as_deref() {
            Ok("new") => Progress::new_game(),
            Ok("all") => Progress::everything(),
            Ok("slice") => Progress::slice_start(),
            _ => crate::lounge::startup_save()
                .map(|s| s.progress)
                .unwrap_or_else(Progress::new_game),
        };
        app.insert_resource(PlayerProgress(progress))
            .init_resource::<SpawnedFor>()
            .add_systems(
                Update,
                (give_abilities, spawn_pickups, collect, bob, show_banner),
            );
    }
}

/// Everything the player has found.
#[derive(Resource, Debug, Clone)]
pub struct PlayerProgress(pub Progress);

/// A pickup waiting in the room.
#[derive(Component)]
struct Pickup {
    item: Item,
    room: String,
    /// Where it rests, in world space; it bobs about this.
    at: Vec2,
    phase: f32,
}

/// The room build the pickups were last spawned for.
#[derive(Resource, Default)]
struct SpawnedFor(Option<u64>);

/// The movement abilities and the most health follow what has been found.
fn give_abilities(
    progress: Res<PlayerProgress>,
    mut players: Query<(&mut Player, &mut crate::combat::Fighting)>,
) {
    let want = progress.0.movement();
    let health = progress.0.max_health();
    let poise = progress.0.max_poise();
    let strength = progress.0.stats.strength_scale();
    for (mut p, mut f) in &mut players {
        f.combat.damage_scale = strength;
        f.combat.poise_scale = strength;
        if f.combat.max_poise != poise {
            f.combat.poise += poise - f.combat.max_poise;
            f.combat.max_poise = poise;
        }
        if p.ctrl.abilities != want {
            p.ctrl.abilities = want;
        }
        if f.max_health != health {
            // A Carnation fills the new health too.
            f.combat.health += health - f.max_health;
            f.max_health = health;
        }
    }
}

/// A small diamond with a bright core, drawn into the glow buffer.
fn glyph(core: [u8; 3], rim: [u8; 3]) -> Image {
    const S: i32 = 11;
    let mut data = vec![0u8; (S * S * 4) as usize];
    for y in 0..S {
        for x in 0..S {
            let d = (x - S / 2).abs() + (y - S / 2).abs();
            let c = match d {
                0..=1 => [255, 255, 255],
                2..=3 => core,
                4..=5 => rim,
                _ => continue,
            };
            let i = ((y * S + x) * 4) as usize;
            data[i..i + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    Image::new(
        Extent3d {
            width: S as u32,
            height: S as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

fn colours(item: Item) -> ([u8; 3], [u8; 3], LinearRgba) {
    match item {
        Item::Ability(_) => (
            [255, 214, 120],
            [170, 110, 40],
            LinearRgba::rgb(1.0, 0.8, 0.45),
        ),
        Item::Weapon(_) => (
            [255, 140, 190],
            [150, 50, 100],
            LinearRgba::rgb(1.0, 0.45, 0.7),
        ),
        Item::Upgrade(_) => (
            [255, 236, 236],
            [200, 60, 80],
            LinearRgba::rgb(1.0, 0.7, 0.7),
        ),
    }
}

fn spawn_pickups(
    mut commands: Commands,
    active: Res<ActiveRoom>,
    progress: Res<PlayerProgress>,
    mut spawned: ResMut<SpawnedFor>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(room) = &active.room else { return };
    if spawned.0 == Some(active.build) {
        return;
    }
    spawned.0 = Some(active.build);
    for (i, def) in room.entities_of("pickup").enumerate() {
        let name = def.props.get("item").map(String::as_str).unwrap_or("");
        let Some(item) = Item::parse(name) else {
            error!("room `{}`, line {}: no item `{name}`", room.name, def.line);
            continue;
        };
        if progress.0.taken(&room.name, item) {
            continue;
        }
        let (x, y) = def.pixel_pos();
        let at = Vec2::new(x, -y);
        let (core, rim, light) = colours(item);
        commands.spawn((
            RoomPart,
            Name::new(format!("pickup {name}")),
            Pickup {
                item,
                room: room.name.clone(),
                at,
                phase: i as f32 * 1.7,
            },
            Transform::from_translation(at.extend(2.0)),
            Visibility::default(),
            Light2d {
                color: light,
                radius: 56.0,
                strength: 1.2,
                glow: 0.8,
            },
            children![(
                Sprite::from_image(images.add(glyph(core, rim))),
                RenderLayers::layer(GLOW_LAYER),
                Transform::default(),
            )],
        ));
    }
}

/// Bobs each pickup a pixel or two, on twelve steps a second like the rest of the art.
fn bob(time: Res<Time>, mut pickups: Query<(&Pickup, &mut Transform)>) {
    let t = (time.elapsed_secs() * 12.0).floor() / 12.0;
    for (p, mut transform) in &mut pickups {
        let dy = (2.0 * (t * 2.2 + p.phase).sin()).round();
        transform.translation.y = p.at.y + dy;
    }
}

/// Takes a pickup the player touches.
#[allow(clippy::too_many_arguments)]
fn collect(
    mut commands: Commands,
    mut progress: ResMut<PlayerProgress>,
    weapons: Option<Res<crate::combat::Weapons>>,
    pickups: Query<(Entity, &Pickup)>,
    mut players: Query<(&mut Player, &mut crate::combat::Fighting)>,
    mut fx: MessageWriter<SpawnFx>,
    mut sfx: MessageWriter<crate::audio::PlaySfx>,
    mut shake: ResMut<crate::camera::Shake>,
    mut slow: ResMut<crate::combat::SlowMotion>,
    device: Res<crate::prompt::Device>,
) {
    for (mut p, mut f) in &mut players {
        let b = p.ctrl.body;
        for (e, pickup) in &pickups {
            let (x, y) = (pickup.at.x, -pickup.at.y);
            let reach = 8.0;
            let touches = (b.x as f32) < x + reach
                && x - reach < (b.x + b.w) as f32
                && (b.y as f32) < y + reach
                && y - reach < (b.y + b.h) as f32;
            if !touches || !progress.0.take(&pickup.room, pickup.item) {
                continue;
            }
            commands.entity(e).despawn();
            let mut burst = SpawnFx::new(EffectId::Poof, pickup.at);
            burst.options.light = 1.8;
            fx.write(burst);
            sfx.write(crate::audio::PlaySfx {
                pitch: 1.5,
                ..crate::audio::PlaySfx::new(crate::audio::Sfx::LureCatch)
            });
            shake.kick(Vec2::NEG_Y, 2.0);
            if let (world::Item::Weapon(w), Some(weapons)) = (pickup.item, &weapons) {
                p.carried = w.into();
                f.weapon = weapons.handle(p.carried);
            }
            // An ability found is an event: the world holds its breath for a moment.
            if matches!(pickup.item, Item::Ability(_)) {
                slow.left = 0.6;
                slow.speed = 0.2;
                shake.kick(Vec2::Y, 4.0);
                let mut ring = SpawnFx::new(EffectId::HammerSmash, pickup.at - Vec2::Y * 8.0);
                ring.options.light = 2.0;
                fx.write(ring);
            }
            commands.spawn(Banner::bundle(pickup.item, *device));
            info!("found the {}", pickup.item.name());
        }
    }
}

/// The name of what was just found, and how to use it, across the top of the screen.
#[derive(Component)]
pub(crate) struct Banner {
    age: f32,
}

/// Seconds a banner shows, including its fade.
const BANNER_TIME: f32 = 3.0;

impl Banner {
    pub(crate) fn bundle(item: Item, device: crate::prompt::Device) -> impl Bundle {
        let (title, hint) = words(item, device);
        Self::bundle_text(title, &hint)
    }

    /// A banner with a title and a line under it.
    pub(crate) fn bundle_text(title: &str, hint: &str) -> impl Bundle + use<> {
        (
            Banner { age: 0.0 },
            Text2d::new(format!("{title}\n{hint}")),
            TextFont {
                font_size: bevy::text::FontSize::Px(10.0),
                ..default()
            },
            TextLayout::justify(Justify::Center),
            TextColor(Color::srgb(1.0, 0.85, 0.6)),
            RenderLayers::layer(crate::render::OVERLAY_LAYER),
            Transform::from_xyz(0.0, 0.0, 6.0),
        )
    }
}

/// What a find is called on screen, and a line on how to use it,
/// naming the buttons on the device the player holds.
fn words(item: Item, device: crate::prompt::Device) -> (&'static str, String) {
    use crate::prompt::Control;
    use world::progress::{Ability, Upgrade, Weapon};
    let key = |c: Control| c.key(device);
    match item {
        Item::Ability(a) => match a {
            Ability::Lunge => (
                "THE LUNGE",
                format!("{} to dash, through guests too", key(Control::Lunge)),
            ),
            Ability::Cling => (
                "THE CLING",
                format!(
                    "Press into a wall to hold on, {} to leap off",
                    key(Control::Jump)
                ),
            ),
            Ability::Swoon => (
                "THE SWOON",
                format!("{} again in the air", key(Control::Jump)),
            ),
            Ability::Lure => (
                "THE LURE",
                format!("{} takes hold of the light", key(Control::Lure)),
            ),
            Ability::Plunge => (
                "THE PLUNGE",
                format!(
                    "{} in the air dives through what is cracked",
                    key(Control::Plunge)
                ),
            ),
            Ability::Submerge => ("SUBMERGE", "Sink into the nectar".into()),
            Ability::Drift => ("THE DRIFT", "Ride the warm air".into()),
            Ability::Numb => ("NUMB", "Feel nothing".into()),
        },
        Item::Upgrade(u) => match u {
            Upgrade::Carnation => ("A CARNATION", "Pinned to the lapel. More health".into()),
            Upgrade::Goblet => ("A GOBLET", "Room for more Nectar".into()),
        },
        Item::Weapon(w) => {
            let switch = format!("{} changes weapon", key(Control::SwitchWeapon));
            match w {
                Weapon::Sabre => ("THE SABRE", switch),
                Weapon::Jeroboam => ("THE JEROBOAM", format!("Swung like a maul. {switch}")),
                Weapon::Candelabrum => (
                    "THE CANDELABRUM",
                    format!("Its swings leave fire. {switch}"),
                ),
            }
        }
    }
}

/// Keeps banners at the top of the view, and fades them out.
fn show_banner(
    mut commands: Commands,
    time: Res<Time>,
    focus: Res<CameraFocus>,
    mut banners: Query<(Entity, &mut Banner, &mut Transform, &mut TextColor)>,
) {
    for (e, mut b, mut t, mut color) in &mut banners {
        b.age += time.delta_secs();
        if b.age >= BANNER_TIME {
            commands.entity(e).despawn();
            continue;
        }
        let top = focus.0.round() + Vec2::new(0.0, HEIGHT as f32 / 2.0 - 40.0);
        t.translation = top.extend(6.0);
        let fade = ((BANNER_TIME - b.age) / 0.6).min(1.0);
        color.0 = Color::srgba(1.0, 0.85, 0.6, fade);
    }
}
