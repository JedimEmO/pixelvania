//! Room-selected, independently lit scenery with pixel-snapped parallax.
//!
//! A room picks a set from `assetgen::backgrounds` with `background = cellars`,
//! and may tint its distance with a colour after the name: `background = ballroom 345a32`.

use assetgen::backgrounds::{self, DEFAULT_LIGHT, DEFAULT_SEED};
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::render::{BACKGROUND_LAYER, CameraFocus, FOREGROUND_LAYER};
use crate::rooms::ActiveRoom;

pub struct BackdropPlugin;

impl Plugin for BackdropPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (rebuild, place).chain().before(TransformSystems::Propagate),
        );
    }
}

#[derive(Component)]
struct Strip {
    parallax: f32,
    copy: i32,
}

fn rebuild(
    mut commands: Commands,
    active: Res<ActiveRoom>,
    strips: Query<Entity, With<Strip>>,
    mut images: ResMut<Assets<Image>>,
) {
    if !active.is_changed() {
        return;
    }
    for entity in &strips {
        commands.entity(entity).despawn();
    }
    let Some(room) = &active.room else { return };
    let Some(spec) = &room.background else { return };
    let Some((name, light)) = parse(spec) else {
        warn!("cannot read the room background `{spec}`");
        return;
    };
    let Some(layers) = backgrounds::set(name, DEFAULT_SEED, light) else {
        warn!("unknown room background `{name}`");
        return;
    };
    for (depth, layer) in layers.into_iter().enumerate() {
        let image = images.add(Image::new(
            Extent3d {
                width: layer.image.width(),
                height: layer.image.height(),
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            layer.image.into_raw(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        ));
        let render_layer = if layer.foreground {
            FOREGROUND_LAYER
        } else {
            BACKGROUND_LAYER
        };
        // Three copies cover the viewport and its margin even across the repeat boundary.
        for copy in -1..=1 {
            commands.spawn((
                Name::new(format!("{name} {} {copy}", layer.name)),
                Strip {
                    parallax: layer.parallax,
                    copy,
                },
                Sprite::from_image(image.clone()),
                Transform::from_xyz(0.0, 0.0, depth as f32),
                RenderLayers::layer(render_layer),
            ));
        }
    }
}

/// A room's `background`: a set's name, and the colour that tints its distance, if one follows,
/// as in `ballroom 345a32`.
fn parse(spec: &str) -> Option<(&str, [u8; 3])> {
    let mut words = spec.split_whitespace();
    let name = words.next()?;
    let light = match words.next() {
        None => DEFAULT_LIGHT,
        Some(hex) => {
            let v = u32::from_str_radix(hex.trim_start_matches('#'), 16).ok()?;
            if hex.trim_start_matches('#').len() != 6 {
                return None;
            }
            [(v >> 16) as u8, (v >> 8) as u8, v as u8]
        }
    };
    words.next().is_none().then_some((name, light))
}

fn place(focus: Res<CameraFocus>, mut strips: Query<(&Strip, &mut Transform)>) {
    let snapped = focus.0.round();
    let width = backgrounds::WIDTH as f32;
    for (strip, mut transform) in &mut strips {
        // At camera x=320 the first preview is centred at x=320 as well.
        let offset = backgrounds::scroll(focus.0.x - 320.0, strip.parallax)
            .rem_euclid(backgrounds::WIDTH as i32) as f32;
        transform.translation.x =
            snapped.x + width / 2.0 - 320.0 - offset + strip.copy as f32 * width;
        // The extra 40 rows allow a little vertical travel without exposing an empty edge.
        let vertical = ((focus.0.y + 200.0) * (1.0 - strip.parallax))
            .clamp(-18.0, 18.0)
            .round();
        transform.translation.y = snapped.y + vertical;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_background_names_a_set_and_may_tint_it() {
        assert_eq!(parse("ballroom"), Some(("ballroom", DEFAULT_LIGHT)));
        assert_eq!(
            parse("cellars 345a32"),
            Some(("cellars", [0x34, 0x5a, 0x32]))
        );
        assert_eq!(
            parse("cellars #345a32"),
            Some(("cellars", [0x34, 0x5a, 0x32]))
        );
        assert_eq!(parse("cellars 345a3"), None);
        assert_eq!(parse("cellars green"), None);
        assert_eq!(parse("cellars 345a32 more"), None);
    }
}
