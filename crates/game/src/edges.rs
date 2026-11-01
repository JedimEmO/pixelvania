//! The room's edges: black beyond its bounds, and darkness in its doorways.
//!
//! A door is an opening in the room's outer wall, so without this the player would see
//! the backdrop carry on past it, as if the door opened onto air.
//! Each doorway gets a band of darkness instead, thickest at the room's edge and dithered
//! away over a few tiles inwards, so a door reads as a passage leading somewhere unlit.
//! Both are drawn in the foreground, over the player, who walks into the dark as they leave.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite::Anchor;
use world::doors::Side;

use crate::render::FOREGROUND_LAYER;
use crate::rooms::{ActiveRoom, RoomPart};

pub struct EdgesPlugin;

impl Plugin for EdgesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn);
    }
}

/// How far a doorway's darkness reaches into the room past the opening, in pixels.
const REACH: u32 = 40;
/// How far the black beyond the room's bounds extends, in pixels.
const BEYOND: f32 = 4096.0;
/// Over the foreground scenery.
const Z: f32 = 50.0;

/// Spawns the edges of the room, once per room build.
fn spawn(
    mut commands: Commands,
    active: Res<ActiveRoom>,
    mut spawned: Local<Option<u64>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(room) = &active.room else { return };
    if *spawned == Some(active.build) {
        return;
    }
    *spawned = Some(active.build);
    let (w, h) = room.pixel_size();
    let (w, h) = (w as f32, h as f32);

    // Black on all four sides, overlapping at the corners. Top-left corners, y down.
    let beyond = [
        (-BEYOND, -BEYOND, BEYOND, h + 2.0 * BEYOND),
        (w, -BEYOND, BEYOND, h + 2.0 * BEYOND),
        (0.0, -BEYOND, w, BEYOND),
        (0.0, h, w, BEYOND),
    ];
    for (x, y, bw, bh) in beyond {
        commands.spawn((
            RoomPart,
            Name::new("beyond the room"),
            Sprite::from_color(Color::BLACK, Vec2::new(bw, bh)),
            Anchor::TOP_LEFT,
            Transform::from_xyz(x, -y, Z),
            RenderLayers::layer(FOREGROUND_LAYER),
        ));
    }

    let Ok(doors) = room.doors() else { return };
    for door in doors {
        let Some(side) = door.side(room) else {
            continue;
        };
        let across = match side {
            Side::Left | Side::Right => door.w,
            Side::Top | Side::Bottom => door.h,
        };
        let depth = across.round() as u32 + REACH;
        let (x, y, iw, ih) = match side {
            Side::Left => (0.0, door.y, depth, door.h as u32),
            Side::Right => (w - depth as f32, door.y, depth, door.h as u32),
            Side::Top => (door.x, 0.0, door.w as u32, depth),
            Side::Bottom => (door.x, h - depth as f32, door.w as u32, depth),
        };
        let image = shade(iw, ih, side, across.round() as u32);
        commands.spawn((
            RoomPart,
            Name::new(format!("doorway {}", door.id)),
            Sprite::from_image(images.add(image)),
            Anchor::TOP_LEFT,
            Transform::from_xyz(x.round(), -y.round(), Z),
            RenderLayers::layer(FOREGROUND_LAYER),
        ));
    }
}

/// A doorway's darkness: solid over the opening, then dithered away to nothing over [`REACH`].
fn shade(w: u32, h: u32, side: Side, solid: u32) -> Image {
    const BAYER: [[f32; 4]; 4] = [
        [0.0, 8.0, 2.0, 10.0],
        [12.0, 4.0, 14.0, 6.0],
        [3.0, 11.0, 1.0, 9.0],
        [15.0, 7.0, 13.0, 5.0],
    ];
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            // Distance from the room's edge, in pixels.
            let d = match side {
                Side::Left => x,
                Side::Right => w - 1 - x,
                Side::Top => y,
                Side::Bottom => h - 1 - y,
            };
            let k = if d < solid {
                1.0
            } else {
                1.0 - (d - solid) as f32 / REACH as f32
            };
            // Four steps of darkness, the step between them chosen by an ordered dither.
            let threshold = (BAYER[(y % 4) as usize][(x % 4) as usize] + 0.5) / 16.0;
            let level = ((k * k * 4.0 + threshold).floor() / 4.0).clamp(0.0, 1.0);
            data.extend_from_slice(&[0, 0, 0, (level * 255.0).round() as u8]);
        }
    }
    Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}
