//! The player: a body that runs and jumps on the fixed tick.
//!
//! This is the minimum the foundations need. Coyote time, jump buffering, apex hang and the
//! rest of the movement techniques in `docs/design/game-feel.md` arrive with the feel lab.

use bevy::prelude::*;
use bevy::sprite::Anchor;
use world::Body;

use crate::input::TickInput;
use crate::rooms::ActiveRoom;
use crate::tuning::Tuning;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, tick)
            .add_systems(Update, (spawn, place_sprite).chain());
    }
}

const SIZE: IVec2 = IVec2::new(12, 34);

#[derive(Component)]
pub struct Player {
    pub body: Body,
    pub vel: Vec2,
    /// Exact position at the start of the last tick, for interpolation.
    prev: Vec2,
    jump_cut_done: bool,
}

impl Player {
    pub fn exact_pos(&self) -> Vec2 {
        let (x, y) = self.body.exact_pos();
        Vec2::new(x, y)
    }

    /// Centre of the body in world space (y up), interpolated between ticks.
    pub fn render_centre(&self, overstep: f32) -> Vec2 {
        let p = self.prev.lerp(self.exact_pos(), overstep) + SIZE.as_vec2() / 2.0;
        Vec2::new(p.x, -p.y)
    }
}

/// Spawns the player at the room's `spawn` entity once a room has loaded.
fn spawn(mut commands: Commands, active: Res<ActiveRoom>, existing: Query<(), With<Player>>) {
    let Some(room) = &active.room else { return };
    if !existing.is_empty() {
        return;
    }
    let (x, y) = room
        .entities_of("spawn")
        .next()
        .map(|s| s.pixel_pos())
        .unwrap_or((32.0, 32.0));
    // The spawn point is where the feet go.
    let body = Body::new(x as i32 - SIZE.x / 2, y as i32 - SIZE.y, SIZE.x, SIZE.y);
    let pos = Vec2::new(body.x as f32, body.y as f32);
    commands.spawn((
        Name::new("player"),
        Player {
            body,
            vel: Vec2::ZERO,
            prev: pos,
            jump_cut_done: false,
        },
        Sprite::from_color(Color::srgb_u8(0x8c, 0x88, 0x9a), SIZE.as_vec2()),
        Anchor::TOP_LEFT,
        Transform::from_xyz(pos.x, -pos.y, 1.0),
    ));
}

fn tick(
    time: Res<Time>,
    mut input: ResMut<TickInput>,
    tuning: Res<Tuning>,
    active: Res<ActiveRoom>,
    mut players: Query<&mut Player>,
) {
    let Some(room) = &active.room else { return };
    let dt = time.delta_secs();
    let m = &tuning.movement;
    let jump_pressed = std::mem::take(&mut input.jump_pressed);

    for mut p in &mut players {
        p.prev = p.exact_pos();
        let grounded = p.body.on_ground(room);

        // Walking reads the stick as digital: past the threshold is full speed.
        let dir = if input.move_axis.x.abs() > m.walk_threshold {
            input.move_axis.x.signum()
        } else {
            0.0
        };
        let target = dir * m.run_speed;
        let accel = match (grounded, dir != 0.0) {
            (true, true) if p.vel.x * dir < 0.0 => m.turn_accel,
            (true, true) => m.run_accel,
            (true, false) => m.run_decel,
            (false, true) => m.air_accel,
            (false, false) => m.air_decel,
        };
        p.vel.x = move_towards(p.vel.x, target, accel * dt);

        if grounded && jump_pressed {
            p.vel.y = -m.jump_speed;
            p.jump_cut_done = false;
        }
        if p.vel.y < 0.0 && !input.jump_held && !p.jump_cut_done {
            p.vel.y *= m.jump_cut;
            p.jump_cut_done = true;
        }
        p.vel.y = (p.vel.y + m.gravity * dt).min(m.max_fall);

        let vx = p.vel.x * dt;
        let vy = p.vel.y * dt;
        if p.body.move_x(room, vx) {
            p.vel.x = 0.0;
        }
        if p.body.move_y(room, vy) {
            p.vel.y = 0.0;
        }
    }
}

fn move_towards(from: f32, to: f32, step: f32) -> f32 {
    if (to - from).abs() <= step {
        to
    } else {
        from + (to - from).signum() * step
    }
}

/// Puts the sprite at the interpolated position, on whole pixels.
fn place_sprite(fixed: Res<Time<Fixed>>, mut players: Query<(&Player, &mut Transform)>) {
    let t = fixed.overstep_fraction();
    for (p, mut transform) in &mut players {
        let pos = p.prev.lerp(p.exact_pos(), t).round();
        transform.translation.x = pos.x;
        transform.translation.y = -pos.y;
    }
}
