//! The camera: leads the way the player faces, holds still through jumps, stays in the room.
//!
//! Horizontally, the camera looks ahead of the player in the direction they face,
//! and the look-ahead swings round at a fixed speed when they turn.
//! Vertically, it follows the height the player last stood at, not the player,
//! so jumps do not bob the view. Falling or rising far enough lets go of that height
//! until the player stands again.

use bevy::prelude::*;

use crate::player::Player;
use crate::render::{CameraFocus, HEIGHT, WIDTH};
use crate::rooms::ActiveRoom;
use crate::tuning::Tuning;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Follow>()
            .add_systems(Update, follow.after(crate::player::place));
    }
}

#[derive(Resource, Default)]
struct Follow {
    started: bool,
    lead: f32,
    /// The height the camera follows, world y up.
    anchor_y: f32,
    /// The player has fallen or risen far enough that the camera tracks them directly.
    tracking_y: bool,
}

fn follow(
    time: Res<Time>,
    fixed: Res<Time<Fixed>>,
    tuning: Res<Tuning>,
    active: Res<ActiveRoom>,
    player: Option<Single<&Player>>,
    mut state: ResMut<Follow>,
    mut focus: ResMut<CameraFocus>,
) {
    let (Some(room), Some(player)) = (&active.room, player) else {
        return;
    };
    let c = &tuning.camera;
    let dt = time.delta_secs();
    let centre = player.render_centre(fixed.overstep_fraction());
    let ctrl = &player.ctrl;

    let lead_target = ctrl.facing as f32 * c.lead;
    if !state.started {
        state.lead = lead_target;
        state.anchor_y = centre.y;
    }
    state.lead += (lead_target - state.lead).clamp(-c.lead_speed * dt, c.lead_speed * dt);

    if ctrl.grounded || ctrl.wall != 0 {
        state.anchor_y = centre.y;
        state.tracking_y = false;
    } else if centre.y < state.anchor_y - c.fall_release
        || centre.y > state.anchor_y + c.rise_release
    {
        state.tracking_y = true;
    }
    if state.tracking_y {
        state.anchor_y = centre.y;
    }

    let target = Vec2::new(centre.x + state.lead, state.anchor_y + c.look_up);
    let mut pos = if state.started {
        let kx = 1.0 - (-c.follow_rate_x * dt).exp();
        let ky = 1.0 - (-c.follow_rate_y * dt).exp();
        Vec2::new(
            focus.0.x + (target.x - focus.0.x) * kx,
            focus.0.y + (target.y - focus.0.y) * ky,
        )
    } else {
        state.started = true;
        target
    };

    // Keep the view inside the room; a room smaller than the screen is centred.
    let (rw, rh) = room.pixel_size();
    let half = Vec2::new(WIDTH as f32, HEIGHT as f32) / 2.0;
    let clamp_axis = |v: f32, lo: f32, hi: f32| {
        if lo > hi {
            (lo + hi) / 2.0
        } else {
            v.clamp(lo, hi)
        }
    };
    pos.x = clamp_axis(pos.x, half.x, rw as f32 - half.x);
    pos.y = clamp_axis(pos.y, -(rh as f32) + half.y, -half.y);
    focus.0 = pos;
}
