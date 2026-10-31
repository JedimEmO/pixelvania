//! Room-selected, independently lit scenery with pixel-snapped parallax.

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
    match room.background.as_deref() {
        None => return,
        Some("ballroom") => {}
        Some(name) => {
            warn!("unknown room background `{name}`");
            return;
        }
    }
    for (depth, layer) in backgrounds::ballroom(DEFAULT_SEED, DEFAULT_LIGHT)
        .into_iter()
        .enumerate()
    {
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
                Name::new(format!("ballroom {} {copy}", layer.name)),
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
