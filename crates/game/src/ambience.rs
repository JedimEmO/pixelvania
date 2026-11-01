//! The air of the party: motes of dust and glitter drifting through every room.
//!
//! A few dozen single pixels drift slowly across the view, each with its own speed, sway and
//! twinkle, drawn into the glow buffer so the room's light does not hide them. They live around
//! the camera and wrap at its edges, so they cost the same in every room.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use crate::render::{CameraFocus, GLOW_LAYER, HEIGHT, WIDTH};

pub struct AmbiencePlugin;

impl Plugin for AmbiencePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn).add_systems(Update, drift);
    }
}

const MOTES: u32 = 48;

#[derive(Component)]
struct Mote {
    /// Position relative to the view's corner, in pixels.
    at: Vec2,
    velocity: Vec2,
    phase: f32,
    /// Glitter catches the light brighter than dust.
    glitter: bool,
}

fn hash(n: u32) -> f32 {
    let mut h = n.wrapping_mul(0x9E37_79B1);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85EB_CA77);
    h ^= h >> 13;
    (h % 10_000) as f32 / 10_000.0
}

fn spawn(mut commands: Commands) {
    for i in 0..MOTES {
        let glitter = i % 5 == 0;
        commands.spawn((
            Mote {
                at: Vec2::new(hash(i * 3) * WIDTH as f32, hash(i * 3 + 1) * HEIGHT as f32),
                velocity: Vec2::new(
                    (hash(i * 7) - 0.5) * 8.0,
                    if glitter {
                        -4.0 - hash(i * 5) * 4.0
                    } else {
                        2.0 + hash(i * 5) * 4.0
                    },
                ),
                phase: hash(i * 11) * std::f32::consts::TAU,
                glitter,
            },
            Sprite::from_color(Color::BLACK, Vec2::ONE),
            RenderLayers::layer(GLOW_LAYER),
            Transform::from_xyz(0.0, 0.0, 4.0),
        ));
    }
}

fn drift(
    time: Res<Time>,
    focus: Res<CameraFocus>,
    mut motes: Query<(&mut Mote, &mut Transform, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    let size = Vec2::new(WIDTH as f32, HEIGHT as f32);
    let corner = focus.0.round() - size / 2.0;
    for (mut m, mut tr, mut sprite) in &mut motes {
        let sway = (t * 0.7 + m.phase).sin() * 3.0;
        let v = m.velocity;
        m.at += (v + Vec2::new(sway, 0.0)) * dt;
        m.at = m.at.rem_euclid(size);
        tr.translation = (corner + m.at).round().extend(4.0);
        // Twinkle on the animation's steps, like everything else drawn.
        let step = ((t * 12.0).floor() / 12.0 * 1.3 + m.phase).sin() * 0.5 + 0.5;
        let k = if m.glitter {
            0.35 + 0.65 * step * step
        } else {
            0.08 + 0.1 * step
        };
        sprite.color = if m.glitter {
            Color::srgb(k, 0.8 * k, 0.5 * k)
        } else {
            Color::srgb(k, 0.9 * k, 0.85 * k)
        };
    }
}
