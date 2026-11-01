//! What the Lure looks like: the rope, and the light it would take hold of.
//!
//! The movement is in `world::controller`. Here the rope is drawn as a thread of light
//! from the hands to the chandelier, dim at the hands and bright at the flame,
//! and while the player is free, a small diamond marks the light the stick is aiming at,
//! so a catch is never a guess.

use bevy::prelude::*;

use crate::hanging::Hanging;
use crate::input::TickInput;
use crate::player::{Look, Player};
use crate::rooms::ActiveRoom;
use crate::tuning::Tuning;

pub struct LurePlugin;

impl Plugin for LurePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            draw.after(bevy::transform::TransformSystems::Propagate),
        );
    }
}

/// From y-down room pixels to world space, on the centre of the pixel.
fn to_world(p: (f32, f32)) -> Vec2 {
    Vec2::new(p.0.floor() + 0.5, -p.1.floor() - 0.5)
}

fn draw(
    time: Res<Time>,
    tuning: Res<Tuning>,
    active: Res<ActiveRoom>,
    input: Res<TickInput>,
    players: Query<(&Player, &Look, &GlobalTransform)>,
    fixtures: Query<&Hanging>,
    mut gizmos: Gizmos,
) {
    let Some(room) = &active.room else { return };
    for (p, look, at) in &players {
        let c = &p.ctrl;
        if let Some(rope) = c.rope {
            // From the drawn hands, so the rope meets the rig whatever the physics rounds to.
            let pose = look.animator.pose();
            let feet = at.translation().truncate();
            let hands =
                feet + Vec2::new((pose.fx * pose.face as f64) as f32, -(pose.fy as f32) + 1.0);
            let hands = hands.floor() + Vec2::splat(0.5);
            // To the fixture where it has swayed to.
            let anchor = fixtures
                .iter()
                .find(|h| h.anchor == rope.anchor)
                .map_or(to_world(rope.anchor), |h| {
                    h.grab_point().floor() + Vec2::splat(0.5)
                });
            gizmos.line_gradient_2d(
                hands,
                anchor,
                Color::srgb(0.35, 0.22, 0.12),
                Color::srgb(1.0, 0.82, 0.5),
            );
            continue;
        }
        if !c.abilities.lure {
            continue;
        }
        let controls = world::Controls {
            x: input.move_axis.x,
            y: input.move_axis.y,
            ..world::Controls::default()
        };
        let Some(target) = c.lure_target(room, &tuning.movement, controls) else {
            continue;
        };
        // A slow pulse, stepped on the animation rate like everything else drawn.
        let step = (time.elapsed_secs() * 12.0).floor() / 12.0;
        let r = 6.0 + (step * 5.0).sin().round();
        let centre = to_world(target);
        let glow = Color::srgb(0.55, 0.42, 0.25);
        let corners = [Vec2::X, Vec2::Y, Vec2::NEG_X, Vec2::NEG_Y].map(|d| centre + d * r);
        for i in 0..4 {
            gizmos.line_2d(corners[i], corners[(i + 1) % 4], glow);
        }
    }
}
