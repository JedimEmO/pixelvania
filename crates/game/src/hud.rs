//! The player's health, poise and Nectar, as thin bars in the top-left corner.
//!
//! The bars are drawn into the glow buffer, so the room's lighting never hides them,
//! and follow the view each frame. They are placeholders until the game has a real HUD.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::combat::Fighting;
use crate::player::Player;
use crate::render::{CameraFocus, GLOW_LAYER, HEIGHT, WIDTH};

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn)
            .add_systems(PostUpdate, follow.before(TransformSystems::Propagate));
    }
}

/// Bar length in pixels when full, and the gap from the screen's corner.
const LENGTH: f32 = 60.0;
const MARGIN: f32 = 10.0;

#[derive(Component)]
enum Bar {
    Health,
    Poise,
    Nectar,
}

fn spawn(mut commands: Commands) {
    let bars = [
        (Bar::Health, Color::srgb(0.75, 0.22, 0.32), 0.0, 2.0),
        (Bar::Poise, Color::srgb(0.55, 0.5, 0.62), 4.0, 1.0),
        (Bar::Nectar, Color::srgb(1.0, 0.78, 0.32), 7.0, 2.0),
    ];
    for (bar, color, dy, h) in bars {
        // A dim track under each bar, so an empty bar is still visible.
        commands.spawn((
            Sprite::from_color(Color::srgb(0.12, 0.1, 0.16), Vec2::new(LENGTH, h)),
            Anchor::TOP_LEFT,
            Transform::default(),
            RenderLayers::layer(GLOW_LAYER),
            Track(dy),
        ));
        commands.spawn((
            bar,
            Sprite::from_color(color, Vec2::new(LENGTH, h)),
            Anchor::TOP_LEFT,
            Transform::default(),
            RenderLayers::layer(GLOW_LAYER),
            Track(dy),
        ));
    }
}

/// Offset below the corner, in pixels.
#[derive(Component)]
struct Track(f32);

fn follow(
    focus: Res<CameraFocus>,
    players: Query<&Fighting, With<Player>>,
    indulgence: Res<crate::indulge::Indulgence>,
    progress: Res<crate::progress::PlayerProgress>,
    mut bars: Query<(&Track, &mut Transform, Option<&Bar>)>,
) {
    let corner = focus.0.round() + Vec2::new(-(WIDTH as f32) / 2.0, HEIGHT as f32 / 2.0);
    let fighting = players.single().ok();
    for (track, mut t, bar) in &mut bars {
        // Bars in front of their tracks.
        let z = if bar.is_some() { 5.1 } else { 5.0 };
        t.translation = (corner + Vec2::new(MARGIN, -MARGIN - track.0)).extend(z);
        let Some(bar) = bar else { continue };
        let k = fighting.map_or(0.0, |f| match bar {
            Bar::Health => f.combat.health / f.max_health,
            Bar::Poise => f.combat.poise / f.combat.max_poise,
            Bar::Nectar => indulgence.nectar / progress.0.nectar_capacity(),
        });
        // Whole pixels, so the bar's end does not shimmer.
        t.scale.x = (LENGTH * k.clamp(0.0, 1.0)).round() / LENGTH;
    }
}
