//! What the Lure looks like: the rope, and the light it would take hold of.
//!
//! The movement is in `world::controller`. Here the rope is drawn as a thread of light
//! from the hands to the chandelier, dim at the hands and bright at the flame,
//! and while the player is free, the light the stick is aiming at is marked twice over,
//! so a catch is never a guess: pink brackets snap in around the fixture,
//! and the fixture itself answers, its flames rising and its light swelling (see `hanging`).

use bevy::prelude::*;

use crate::hanging::Hanging;
use crate::input::TickInput;
use crate::player::{Look, Player};
use crate::rooms::ActiveRoom;
use crate::tuning::Tuning;

pub struct LurePlugin;

impl Plugin for LurePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LureTarget>().add_systems(
            PostUpdate,
            draw.after(bevy::transform::TransformSystems::Propagate),
        );
    }
}

/// The light the Lure would take hold of now, if any, and when it became the target.
#[derive(Resource, Default)]
pub struct LureTarget {
    pub anchor: Option<(f32, f32)>,
    since: f32,
}

/// Half the width and height of the brackets round each kind of fixture, and how far
/// below the grab point their centre sits, in pixels.
fn frame(kind: assetgen::hanging::Kind) -> (Vec2, f32) {
    match kind {
        assetgen::hanging::Kind::Chandelier => (Vec2::new(27.0, 19.0), 4.0),
        assetgen::hanging::Kind::Lantern => (Vec2::new(9.0, 13.0), 0.0),
    }
}

/// From y-down room pixels to world space, on the centre of the pixel.
fn to_world(p: (f32, f32)) -> Vec2 {
    Vec2::new(p.0.floor() + 0.5, -p.1.floor() - 0.5)
}

fn draw(
    time: Res<Time>,
    beat: Res<crate::music::Beat>,
    mut target: ResMut<LureTarget>,
    tuning: Res<Tuning>,
    active: Res<ActiveRoom>,
    input: Res<TickInput>,
    players: Query<(&Player, &Look, &GlobalTransform)>,
    fixtures: Query<&Hanging>,
    mut gizmos: Gizmos,
) {
    let Some(room) = &active.room else { return };
    let mut aimed = None;
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
        let Some(anchor) = c.lure_target(room, &tuning.movement, controls) else {
            continue;
        };
        aimed = Some(anchor);
        if target.anchor != Some(anchor) {
            target.since = time.elapsed_secs();
        }
        let Some(fixture) = fixtures.iter().find(|h| h.anchor == anchor) else {
            continue;
        };
        // The brackets snap in from wide over three animation steps,
        // then breathe out a pixel on each beat of the music.
        let step = ((time.elapsed_secs() - target.since) * 12.0) as usize;
        let snap = [1.7, 1.35, 1.12].get(step).copied().unwrap_or(1.0);
        let on_beat = if beat.phase() < 0.15 { 1.0 } else { 0.0 };
        let (half, drop) = frame(fixture.kind());
        let half = (half * snap).round() + Vec2::splat(on_beat);
        let centre = (fixture.grab_point() - Vec2::Y * drop).floor() + Vec2::splat(0.5);
        let arm = 6.0;
        let pink = Color::srgb(1.0, 0.45, 0.75);
        for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            // Two pixels thick: the outer line, and one just inside it.
            for inset in [0.0, 1.0] {
                let corner = centre + Vec2::new(sx * (half.x - inset), sy * (half.y - inset));
                gizmos.line_2d(corner, corner - Vec2::new(sx * arm, 0.0), pink);
                gizmos.line_2d(corner, corner - Vec2::new(0.0, sy * arm), pink);
            }
        }
    }
    target.anchor = aimed;
}
