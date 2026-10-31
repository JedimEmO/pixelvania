//! The player's movement: running, jumping, the Lunge, the Swoon and the Cling.
//!
//! [`Controller::tick`] advances one fixed tick. It takes the input as the tick sees it
//! and returns what happened, so the game can squash the sprite, kick dust or rumble.
//! Nothing here knows about Bevy, which is why the movement metrics can be tested headless.
//!
//! The techniques and the reasons for them are in `docs/design/game-feel.md`.
//! Units are pixels and seconds, with y pointing down.

use serde::{Deserialize, Serialize};

use crate::body::Body;
use crate::room::Room;

/// Every movement number. Speeds are pixels per second, accelerations pixels per second squared,
/// times are seconds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MoveTuning {
    pub run_speed: f32,
    pub run_accel: f32,
    pub run_decel: f32,
    /// Acceleration while reversing direction on the ground.
    pub turn_accel: f32,
    pub air_accel: f32,
    pub air_decel: f32,
    /// Deceleration in the air while faster than `run_speed` and still holding forward,
    /// so a jump out of a Lunge keeps its momentum.
    pub air_overspeed_decel: f32,
    pub gravity: f32,
    pub max_fall: f32,
    pub jump_speed: f32,
    /// Upward speed is multiplied by this when jump is released early.
    pub jump_cut: f32,
    /// Near the top of a jump, while jump is held, gravity is multiplied by `apex_gravity`.
    pub apex_threshold: f32,
    pub apex_gravity: f32,
    /// Holding down while falling multiplies gravity and raises the speed limit.
    pub fast_fall_gravity: f32,
    pub fast_fall_max: f32,
    pub coyote_time: f32,
    pub jump_buffer: f32,
    /// How far a jump that clips a ceiling corner is nudged sideways around it.
    pub corner_correction: i32,
    /// How far a Lunge that clips a ledge top is popped up onto it.
    pub ledge_pop: i32,
    /// Stick deflection needed to walk.
    pub walk_threshold: f32,
    /// Larger deflection needed for down, so a tilted stick never fast-falls by accident.
    pub down_threshold: f32,

    pub lunge_speed: f32,
    pub lunge_time: f32,
    pub lunge_cooldown: f32,

    pub swoon_speed: f32,
    /// The Swoon's apex is slower and floatier than a jump's.
    pub swoon_apex_gravity: f32,
    pub swoon_apex_threshold: f32,

    pub wall_slide_max: f32,
    pub wall_jump_x: f32,
    pub wall_jump_y: f32,
    /// After a wall jump, input towards the wall is ignored for this long.
    pub wall_jump_lock: f32,
    /// A wall jump still works this long after leaving the wall.
    pub wall_coyote: f32,

    /// A landing faster than this counts as hard: the sprite squashes and the pad rumbles.
    pub hard_landing_speed: f32,
}

impl Default for MoveTuning {
    fn default() -> Self {
        Self {
            run_speed: 150.0,
            run_accel: 1900.0,
            run_decel: 2400.0,
            turn_accel: 3200.0,
            air_accel: 1400.0,
            air_decel: 900.0,
            air_overspeed_decel: 300.0,
            gravity: 1450.0,
            max_fall: 420.0,
            jump_speed: 480.0,
            jump_cut: 0.4,
            apex_threshold: 40.0,
            apex_gravity: 0.5,
            fast_fall_gravity: 1.6,
            fast_fall_max: 560.0,
            coyote_time: 0.1,
            jump_buffer: 0.12,
            corner_correction: 4,
            ledge_pop: 3,
            walk_threshold: 0.3,
            down_threshold: 0.6,
            lunge_speed: 400.0,
            lunge_time: 0.14,
            lunge_cooldown: 0.25,
            swoon_speed: 370.0,
            swoon_apex_gravity: 0.3,
            swoon_apex_threshold: 60.0,
            wall_slide_max: 70.0,
            wall_jump_x: 190.0,
            wall_jump_y: 380.0,
            wall_jump_lock: 0.12,
            wall_coyote: 0.08,
            hard_landing_speed: 370.0,
        }
    }
}

/// Which movement abilities the player has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Abilities {
    pub lunge: bool,
    pub swoon: bool,
    pub cling: bool,
}

impl Abilities {
    pub const ALL: Abilities = Abilities {
        lunge: true,
        swoon: true,
        cling: true,
    };
}

/// Input as one tick sees it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Controls {
    /// Stick or d-pad, x right, y up, each -1..1.
    pub x: f32,
    pub y: f32,
    pub jump_held: bool,
    /// Jump was pressed since the previous tick.
    pub jump_pressed: bool,
    pub lunge_pressed: bool,
}

impl Controls {
    /// One line of an input recording: `x y flags`, where flags are `h` (jump held),
    /// `j` (jump pressed) and `l` (Lunge pressed), or `-` for none.
    pub fn encode(&self) -> String {
        let mut flags = String::new();
        for (on, c) in [
            (self.jump_held, 'h'),
            (self.jump_pressed, 'j'),
            (self.lunge_pressed, 'l'),
        ] {
            if on {
                flags.push(c);
            }
        }
        if flags.is_empty() {
            flags.push('-');
        }
        format!("{:.3} {:.3} {flags}", self.x, self.y)
    }

    /// Reads a line written by [`Controls::encode`].
    pub fn decode(line: &str) -> Option<Controls> {
        let mut parts = line.split_whitespace();
        let x = parts.next()?.parse().ok()?;
        let y = parts.next()?.parse().ok()?;
        let flags = parts.next()?;
        Some(Controls {
            x,
            y,
            jump_held: flags.contains('h'),
            jump_pressed: flags.contains('j'),
            lunge_pressed: flags.contains('l'),
        })
    }
}

/// What happened during a tick.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Events {
    pub jumped: bool,
    pub swooned: bool,
    /// The direction pushed away from the wall, -1 or 1.
    pub wall_jumped: Option<i32>,
    pub lunged: bool,
    /// Landed, with the downward speed just before landing.
    pub landed: Option<f32>,
}

impl Events {
    /// Combines the events of several ticks.
    pub fn merge(&mut self, other: Events) {
        self.jumped |= other.jumped;
        self.swooned |= other.swooned;
        self.wall_jumped = self.wall_jumped.or(other.wall_jumped);
        self.lunged |= other.lunged;
        self.landed = match (self.landed, other.landed) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        };
    }
}

/// The player's movement state.
#[derive(Debug, Clone, PartialEq)]
pub struct Controller {
    pub body: Body,
    pub vx: f32,
    pub vy: f32,
    /// -1 or 1.
    pub facing: i32,
    pub abilities: Abilities,
    pub grounded: bool,
    /// The wall being slid down, -1 or 1, or 0.
    pub wall: i32,
    /// Seconds left of the current Lunge, or 0.
    pub lunging: f32,
    coyote: f32,
    jump_buffer: f32,
    jump_cut_done: bool,
    swoon_ready: bool,
    swooning: bool,
    lunge_ready: bool,
    lunge_cooldown: f32,
    wall_coyote: f32,
    wall_coyote_dir: i32,
    wall_lock: f32,
    wall_lock_dir: i32,
}

impl Controller {
    pub fn new(body: Body, abilities: Abilities) -> Self {
        Self {
            body,
            vx: 0.0,
            vy: 0.0,
            facing: 1,
            abilities,
            grounded: false,
            wall: 0,
            lunging: 0.0,
            coyote: 0.0,
            jump_buffer: 0.0,
            jump_cut_done: true,
            swoon_ready: abilities.swoon,
            swooning: false,
            lunge_ready: abilities.lunge,
            lunge_cooldown: 0.0,
            wall_coyote: 0.0,
            wall_coyote_dir: 0,
            wall_lock: 0.0,
            wall_lock_dir: 0,
        }
    }

    /// Places a controller with its feet centred on `(x, y)`.
    pub fn standing_at(x: f32, y: f32, w: i32, h: i32, abilities: Abilities) -> Self {
        Self::new(
            Body::new(x.round() as i32 - w / 2, y.round() as i32 - h, w, h),
            abilities,
        )
    }

    /// Advances one tick of `dt` seconds.
    pub fn tick(&mut self, room: &Room, t: &MoveTuning, input: Controls, dt: f32) -> Events {
        let mut ev = Events::default();

        self.grounded = self.body.on_ground(room);
        if self.grounded {
            self.coyote = t.coyote_time;
            self.swoon_ready = self.abilities.swoon;
            self.swooning = false;
            if self.lunging <= 0.0 {
                self.lunge_ready = self.abilities.lunge;
            }
        } else {
            self.coyote = (self.coyote - dt).max(0.0);
        }
        if input.jump_pressed {
            self.jump_buffer = t.jump_buffer;
        } else {
            self.jump_buffer = (self.jump_buffer - dt).max(0.0);
        }
        self.lunge_cooldown = (self.lunge_cooldown - dt).max(0.0);
        self.wall_lock = (self.wall_lock - dt).max(0.0);

        // Walking reads the stick as digital: past the threshold is full speed.
        let mut dir = if input.x.abs() > t.walk_threshold {
            input.x.signum() as i32
        } else {
            0
        };
        if self.wall_lock > 0.0 && dir == self.wall_lock_dir {
            dir = 0;
        }
        let down = input.y < -t.down_threshold;

        // The Cling: falling while pressing into a wall slides down it slowly.
        self.wall = 0;
        if self.abilities.cling && !self.grounded {
            for side in [-1, 1] {
                if self.body.collides_at(room, side, 0) && dir == side && self.vy >= 0.0 {
                    self.wall = side;
                }
            }
        }
        if self.wall != 0 {
            self.wall_coyote = t.wall_coyote;
            self.wall_coyote_dir = self.wall;
            self.lunge_ready = self.abilities.lunge;
            self.swooning = false;
        } else {
            self.wall_coyote = (self.wall_coyote - dt).max(0.0);
        }

        if self.lunging <= 0.0 && dir != 0 {
            self.facing = dir;
        }

        // The Lunge.
        if input.lunge_pressed && self.lunge_ready && self.lunge_cooldown <= 0.0 {
            let lunge_dir = if self.wall != 0 {
                -self.wall
            } else {
                self.facing
            };
            self.facing = lunge_dir;
            self.lunging = t.lunge_time;
            self.lunge_ready = false;
            self.lunge_cooldown = t.lunge_cooldown;
            self.vx = lunge_dir as f32 * t.lunge_speed;
            self.vy = 0.0;
            ev.lunged = true;
        }

        // Jumps, in order of priority: from the ground (or just after leaving it),
        // off a wall, then the Swoon.
        if self.jump_buffer > 0.0 {
            if self.coyote > 0.0 {
                self.vy = -t.jump_speed;
                ev.jumped = true;
            } else if self.wall_coyote > 0.0 {
                let away = -self.wall_coyote_dir;
                self.vx = away as f32 * t.wall_jump_x;
                self.vy = -t.wall_jump_y;
                self.facing = away;
                self.wall_lock = t.wall_jump_lock;
                self.wall_lock_dir = -away;
                self.wall_coyote = 0.0;
                ev.wall_jumped = Some(away);
            } else if self.swoon_ready && !self.lands_before_buffer_ends(room, t, dt) {
                self.vy = -t.swoon_speed;
                self.swoon_ready = false;
                self.swooning = true;
                ev.swooned = true;
            }
            if ev.jumped || ev.wall_jumped.is_some() || ev.swooned {
                self.jump_buffer = 0.0;
                self.coyote = 0.0;
                self.jump_cut_done = false;
                self.grounded = false;
                // A jump ends a Lunge but keeps its speed.
                self.lunging = 0.0;
            }
        }

        if self.lunging > 0.0 {
            self.lunging = (self.lunging - dt).max(0.0);
            self.vy = 0.0;
            if self.lunging <= 0.0 {
                self.vx = self.facing as f32 * self.vx.abs().min(t.run_speed);
            }
        } else {
            self.run(t, dir, dt);
            self.fall(t, input, down, dt);
        }

        let airborne = !self.grounded;
        let pop = if self.lunging > 0.0 { t.ledge_pop } else { 0 };
        if self.step_x(room, self.vx * dt, pop) {
            self.vx = 0.0;
            self.lunging = 0.0;
        }
        if self.step_y(room, self.vy * dt, t.corner_correction) {
            if airborne && self.vy > 0.0 {
                ev.landed = Some(self.vy);
                self.grounded = true;
            }
            self.vy = 0.0;
        }
        ev
    }

    /// Whether the body, falling as it is now, touches the ground before the jump buffer runs out.
    /// A press that lands in time is a full jump from the ground, not a Swoon spent just above it.
    fn lands_before_buffer_ends(&self, room: &Room, t: &MoveTuning, dt: f32) -> bool {
        // On the buffer's last tick the press is spent on the Swoon rather than lost.
        let left = self.jump_buffer - dt;
        if left <= 0.0 {
            return false;
        }
        let drop =
            (self.vy * left + 0.5 * t.gravity * left * left).min(self.vy.max(t.max_fall) * left);
        (1..=drop.ceil() as i32).any(|dy| self.body.collides_at(room, 0, dy))
    }

    fn run(&mut self, t: &MoveTuning, dir: i32, dt: f32) {
        let dir = dir as f32;
        let target = dir * t.run_speed;
        let same_way = self.vx * dir > 0.0;
        let accel = if self.grounded {
            match (dir != 0.0, same_way || self.vx == 0.0) {
                (true, true) => t.run_accel,
                (true, false) => t.turn_accel,
                (false, _) => t.run_decel,
            }
        } else if same_way && self.vx.abs() > t.run_speed {
            t.air_overspeed_decel
        } else if dir != 0.0 {
            t.air_accel
        } else {
            t.air_decel
        };
        self.vx = move_towards(self.vx, target, accel * dt);
    }

    fn fall(&mut self, t: &MoveTuning, input: Controls, down: bool, dt: f32) {
        if self.vy < 0.0 && !input.jump_held && !self.jump_cut_done {
            self.vy *= t.jump_cut;
            self.jump_cut_done = true;
        }
        let (apex_threshold, apex_gravity) = if self.swooning {
            (t.swoon_apex_threshold, t.swoon_apex_gravity)
        } else {
            (t.apex_threshold, t.apex_gravity)
        };
        let mut gravity = t.gravity;
        let mut max_fall = t.max_fall;
        if input.jump_held && self.vy.abs() < apex_threshold {
            gravity *= apex_gravity;
        }
        if down && self.vy > 0.0 {
            gravity *= t.fast_fall_gravity;
            max_fall = t.fast_fall_max;
        }
        if self.wall != 0 {
            max_fall = t.wall_slide_max;
        }
        self.vy = (self.vy + gravity * dt).min(max_fall.max(self.vy));
        if self.vy > max_fall {
            // Entering a wall slide from a fast fall eases down rather than stopping dead.
            self.vy = move_towards(self.vy, max_fall, t.gravity * 2.0 * dt);
        }
    }

    /// Moves horizontally one pixel at a time. A move blocked by a ledge whose top is within
    /// `pop` pixels of the feet steps up onto it. Returns true if the move was blocked.
    fn step_x(&mut self, room: &Room, amount: f32, pop: i32) -> bool {
        let b = &mut self.body;
        b.rem_x += amount;
        let steps = b.rem_x.round() as i32;
        b.rem_x -= steps as f32;
        let dir = steps.signum();
        for _ in 0..steps.abs() {
            if !b.collides_at(room, dir, 0) {
                b.x += dir;
                continue;
            }
            let up =
                (1..=pop).find(|&k| !b.collides_at(room, 0, -k) && !b.collides_at(room, dir, -k));
            if let Some(up) = up {
                b.y -= up;
                b.x += dir;
                continue;
            }
            b.rem_x = 0.0;
            return true;
        }
        false
    }

    /// Moves vertically one pixel at a time. Rising into a ceiling corner nudges the body
    /// up to `correction` pixels sideways round it, never against the way it is moving.
    /// Returns true if the move was blocked.
    fn step_y(&mut self, room: &Room, amount: f32, correction: i32) -> bool {
        let vx = self.vx;
        let b = &mut self.body;
        b.rem_y += amount;
        let steps = b.rem_y.round() as i32;
        b.rem_y -= steps as f32;
        let dir = steps.signum();
        for _ in 0..steps.abs() {
            if !b.collides_at(room, 0, dir) {
                b.y += dir;
                continue;
            }
            if dir < 0 {
                let nudge = (1..=correction)
                    .flat_map(|d| [(vx >= 0.0).then_some(d), (vx <= 0.0).then_some(-d)])
                    .flatten()
                    .find(|&dx| !b.collides_at(room, dx, 0) && !b.collides_at(room, dx, -1));
                if let Some(dx) = nudge {
                    b.x += dx;
                    b.y -= 1;
                    continue;
                }
            }
            b.rem_y = 0.0;
            return true;
        }
        false
    }
}

fn move_towards(from: f32, to: f32, step: f32) -> f32 {
    if (to - from).abs() <= step {
        to
    } else {
        from + (to - from).signum() * step
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controls_survive_a_recording() {
        let c = Controls {
            x: -0.5,
            y: 1.0,
            jump_held: true,
            jump_pressed: false,
            lunge_pressed: true,
        };
        assert_eq!(Controls::decode(&c.encode()), Some(c));
        let none = Controls::default();
        assert_eq!(Controls::decode(&none.encode()), Some(none));
    }
}
