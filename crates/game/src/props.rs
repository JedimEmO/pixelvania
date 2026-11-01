//! Props: the painted things that say what a room is for.
//!
//! A prop is a room entity, `prop 12 21 kind=coat_rack`, its position the middle of its feet.
//! `flip=1` mirrors it, and `z=front` draws it in front of the player (the default, `z=back`,
//! draws it behind everyone). Props are decoration: nothing collides with them.
//!
//! Each is painted by `assetgen::props` into a surface image, lit by the room like the tiles,
//! and a glow image for what shines by itself. Some cast a small light of their own,
//! and some animate their glow (flames, bubbles, smoke), which is repainted on the animation's steps.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite::Anchor;

use crate::render::{GLOW_LAYER, Light2d};
use crate::rooms::{ActiveRoom, RoomPart};

pub struct PropsPlugin;

impl Plugin for PropsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (spawn, animate));
    }
}

/// Behind the tiles' front, the lounges and every body.
const BACK_Z: f32 = 0.2;
/// In front of the player and the guests, behind effects.
const FRONT_Z: f32 = 1.5;

/// A prop whose glow is repainted on the animation's steps.
#[derive(Component)]
struct Animated {
    kind: String,
    flip: bool,
    glow: Handle<Image>,
}

/// RGBA pixels, mirrored left to right when `flip` is set.
fn pixels(w: u32, raw: &[u8], flip: bool) -> Vec<u8> {
    if !flip {
        return raw.to_vec();
    }
    raw.chunks_exact(w as usize * 4)
        .flat_map(|row| row.as_chunks::<4>().0.iter().rev().flatten().copied())
        .collect()
}

fn upload(w: u32, h: u32, raw: &[u8], flip: bool, images: &mut Assets<Image>) -> Handle<Image> {
    images.add(Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels(w, raw, flip),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    ))
}

/// Spawns the room's props, once per room build.
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
    for def in room.entities_of("prop") {
        let Some(kind) = def.props.get("kind") else {
            error!(
                "room `{}`, line {}: a prop needs a `kind`",
                room.name, def.line
            );
            continue;
        };
        let Some(piece) = assetgen::props::paint(kind, 0) else {
            error!(
                "room `{}`, line {}: no prop kind `{kind}`",
                room.name, def.line
            );
            continue;
        };
        let flip = def.props.get("flip").is_some_and(|v| v == "1");
        let z = match def.props.get("z").map(String::as_str) {
            None | Some("back") => BACK_Z,
            Some("front") => FRONT_Z,
            Some(other) => {
                error!(
                    "room `{}`, line {}: `z={other}` is not `back` or `front`",
                    room.name, def.line
                );
                BACK_Z
            }
        };
        let (w, h) = piece.color.dimensions();
        let (x, y) = def.pixel_pos();
        // Odd widths would put the middle between pixels; keep the sprite on the grid.
        let half = if w % 2 == 1 { 0.5 } else { 0.0 };
        let at = Vec2::new(x.round() + half, -y.round());
        let glow = upload(w, h, piece.glow.as_raw(), flip, &mut images);
        let mut e = commands.spawn((
            RoomPart,
            Name::new(format!("prop {kind}")),
            Transform::from_translation(at.extend(z)),
            Visibility::default(),
            children![
                (
                    Sprite::from_image(upload(w, h, piece.color.as_raw(), flip, &mut images)),
                    Anchor::BOTTOM_CENTER,
                    Transform::default(),
                ),
                (
                    Sprite::from_image(glow.clone()),
                    Anchor::BOTTOM_CENTER,
                    Transform::default(),
                    RenderLayers::layer(GLOW_LAYER),
                ),
            ],
        ));
        if assetgen::props::animated(kind) {
            e.insert(Animated {
                kind: kind.clone(),
                flip,
                glow,
            });
        }
        if let Some(l) = assetgen::props::light(kind) {
            let [r, g, b] = l.color;
            let lx = if flip { -l.x } else { l.x };
            commands.spawn((
                RoomPart,
                Name::new(format!("prop {kind} light")),
                Light2d {
                    color: Color::srgb_u8(r, g, b).to_linear(),
                    radius: l.radius,
                    strength: l.strength,
                    glow: 0.0,
                },
                Transform::from_xyz(at.x + lx, at.y + l.y, 0.0),
            ));
        }
    }
}

/// Repaints animated glows on the animation's steps, 12 a second.
fn animate(
    time: Res<Time>,
    props: Query<&Animated>,
    mut images: ResMut<Assets<Image>>,
    mut last_step: Local<u32>,
) {
    let step = (time.elapsed_secs() * 12.0) as u32;
    if step == *last_step {
        return;
    }
    *last_step = step;
    for p in &props {
        let Some(piece) = assetgen::props::paint(&p.kind, step) else {
            continue;
        };
        let glow = pixels(piece.glow.width(), piece.glow.as_raw(), p.flip);
        if let Some(mut img) = images.get_mut(&p.glow) {
            img.data = Some(glow);
        }
    }
}
