//! Fire left burning on the floor by the candelabrum's grounded swings.
//!
//! A patch is a strip of flame on top of a floor tile. It burns guests standing in it,
//! a little at a time, without staggering them, and lights the room around it with a
//! flickering light. Its tongues are painted each frame into the glow buffer, so they burn
//! bright whatever the lighting, and they shrink as the patch dies down.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite::Anchor;
use world::{Room, TILE};

use crate::combat::{Fighting, GUEST_TEAM};
use crate::enemies::Guest;
use crate::input::TickSet;
use crate::render::{GLOW_LAYER, Light2d};
use crate::rooms::RoomPart;

pub struct FirePlugin;

impl Plugin for FirePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            burn.in_set(TickSet::Simulate)
                .after(crate::combat::SimulateBodies),
        )
        .add_systems(Update, flicker);
    }
}

/// A patch's width in pixels, how long it burns, and its damage per second.
const WIDTH: u32 = 40;
const HEIGHT: u32 = 18;
const LIFE: f32 = 2.5;
const DAMAGE_PER_SECOND: f32 = 6.0;

#[derive(Component)]
pub struct Fire {
    /// Left edge and floor height, in pixels with y down.
    x: f32,
    floor: f32,
    age: f32,
    image: Handle<Image>,
}

/// The top of the first floor at or below `y` in the column under `x`, in pixels with y down.
fn floor_below(room: &Room, x: f32, y: f32) -> Option<f32> {
    let tx = (x.floor() as i32).div_euclid(TILE);
    let from = (y.floor() as i32 - TILE).div_euclid(TILE);
    let (_, h) = room.pixel_size();
    (from.max(0)..h / TILE)
        .find(|&ty| room.is_solid(tx, ty) && !room.is_solid(tx, ty - 1))
        .map(|ty| (ty * TILE) as f32)
}

/// Lights a patch of fire centred on `x`, on the floor below `y` (pixels, y down),
/// if there is a floor within a few tiles.
pub fn ignite(commands: &mut Commands, images: &mut Assets<Image>, room: &Room, x: f32, y: f32) {
    let Some(floor) = floor_below(room, x, y).filter(|f| *f - y < 3.0 * TILE as f32) else {
        return;
    };
    let image = images.add(Image::new_fill(
        Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    ));
    commands.spawn((
        Name::new("fire"),
        RoomPart,
        Fire {
            x: x - WIDTH as f32 / 2.0,
            floor,
            age: 0.0,
            image: image.clone(),
        },
        Transform::from_xyz((x - WIDTH as f32 / 2.0).round(), -floor, 1.5),
        Visibility::default(),
        Light2d {
            color: LinearRgba::rgb(1.0, 0.55, 0.2),
            radius: 70.0,
            strength: 0.9,
            glow: 0.0,
        },
        children![(
            Sprite::from_image(image),
            Anchor::BOTTOM_LEFT,
            Transform::default(),
            RenderLayers::layer(GLOW_LAYER),
        )],
    ));
}

/// Burns guests standing in the fire, and puts fires out when they have burned down.
fn burn(
    mut commands: Commands,
    mut fires: Query<(Entity, &mut Fire)>,
    mut guests: Query<(&Guest, &mut Fighting)>,
) {
    let dt = world::combat::DT;
    for (e, mut fire) in &mut fires {
        fire.age += dt;
        if fire.age >= LIFE {
            commands.entity(e).despawn();
            continue;
        }
        for (g, mut f) in &mut guests {
            if f.team != GUEST_TEAM {
                continue;
            }
            let b = &g.ctrl.body;
            let feet = (b.y + b.h) as f32;
            let inside = (b.x + b.w) as f32 > fire.x
                && (b.x as f32) < fire.x + WIDTH as f32
                && (feet - fire.floor).abs() < 4.0;
            if inside {
                f.combat.burn(DAMAGE_PER_SECOND * dt);
            }
        }
    }
}

/// A small integer hash, for flame shapes that change every animation step.
fn hash(a: u32, b: u32) -> u32 {
    let mut h = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0xC2B2_AE3D);
    h ^ (h >> 13)
}

/// Paints the tongues of flame and flickers the light, on 12 fps steps like the rest of the art.
fn flicker(mut fires: Query<(Entity, &Fire, &mut Light2d)>, mut images: ResMut<Assets<Image>>) {
    let ramp: [[u8; 4]; 4] = [
        [0xff, 0xf6, 0xd0, 0xff],
        [0xff, 0xd3, 0x6a, 0xff],
        [0xff, 0x8a, 0x2a, 0xff],
        [0xc4, 0x3a, 0x1a, 0xff],
    ];
    for (e, fire, mut light) in &mut fires {
        let step = (fire.age * 12.0) as u32;
        let life = 1.0 - fire.age / LIFE;
        let seed = e.index_u32();
        let mut data = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
        for x in 0..WIDTH {
            // Taller in the middle of the patch, lower at its ends and as it dies down.
            let edge = 1.0 - ((x as f32 + 0.5) / WIDTH as f32 * 2.0 - 1.0).abs();
            let noise = (hash(seed ^ x, step) % 100) as f32 / 100.0;
            let height = ((3.0 + noise * 10.0) * edge.sqrt() * life).round() as u32;
            for y in 0..height.min(HEIGHT) {
                // Hottest at the base, cooling to red at the tip.
                let u = y as f32 / height.max(1) as f32;
                let c = ramp[((u * 3.99) as usize).min(3)];
                let row = HEIGHT - 1 - y;
                let i = ((row * WIDTH + x) * 4) as usize;
                data[i..i + 4].copy_from_slice(&c);
            }
        }
        if let Some(mut image) = images.get_mut(&fire.image) {
            image.data = Some(data);
        }
        let flick = 0.8 + (hash(seed, step) % 40) as f32 / 100.0;
        light.strength = 0.9 * life * flick;
    }
}
