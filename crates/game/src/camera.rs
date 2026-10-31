//! The camera: follows the player smoothly, looks ahead, and stays inside the room.

use bevy::prelude::*;

use crate::player::Player;
use crate::render::{CameraFocus, HEIGHT, WIDTH};
use crate::rooms::ActiveRoom;
use crate::tuning::Tuning;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, follow);
    }
}

fn follow(
    time: Res<Time>,
    fixed: Res<Time<Fixed>>,
    tuning: Res<Tuning>,
    active: Res<ActiveRoom>,
    player: Option<Single<&Player>>,
    mut focus: ResMut<CameraFocus>,
    mut started: Local<bool>,
) {
    let (Some(room), Some(player)) = (&active.room, player) else {
        return;
    };
    let c = &tuning.camera;
    // Look ahead in proportion to running speed; the follow smoothing eases it in and out.
    let lead = (player.vel.x / tuning.movement.run_speed).clamp(-1.0, 1.0) * c.lead;
    let target = player.render_centre(fixed.overstep_fraction()) + Vec2::new(lead, 0.0);

    let k = if *started {
        1.0 - (-c.follow_rate * time.delta_secs()).exp()
    } else {
        *started = true;
        1.0
    };
    let mut pos = focus.0.lerp(target, k);

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
