//! The player's movement: running, jumping, the Lunge, the Swoon, the Cling and the Lure.
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

    /// How far away a light can be for the Lure to reach it.
    pub lure_range: f32,
    /// Half the angle, in radians, of the cone around the aim in which a light can be picked.
    pub lure_cone: f32,
    /// The rope reels in at this speed until it is `lure_swing_length` long.
    pub lure_reel_speed: f32,
    pub lure_swing_length: f32,
    /// Up and down on the stick climb and lower along the rope, within these lengths.
    pub lure_min_length: f32,
    pub lure_climb_speed: f32,
    /// Acceleration along the swing from the stick, for pumping.
    pub lure_pump: f32,
    pub lure_max_speed: f32,
    /// Letting go multiplies the speed by this, so a release flings.
    pub lure_release_boost: f32,
    /// Upward speed of a jump off the rope.
    pub lure_jump_speed: f32,
    /// A rope snapping taut that would take more speed than this keeps the speed instead,
    /// turned along the swing.
    pub lure_snap_keep: f32,
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
            lure_range: 190.0,
            lure_cone: 0.75,
            lure_reel_speed: 420.0,
            lure_swing_length: 96.0,
            lure_min_length: 36.0,
            lure_climb_speed: 110.0,
            lure_pump: 520.0,
            lure_max_speed: 560.0,
            lure_release_boost: 1.12,
            lure_jump_speed: 420.0,
            lure_snap_keep: 40.0,
        }
    }
}

/// Which movement abilities the player has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Abilities {
    pub lunge: bool,
    pub swoon: bool,
    pub cling: bool,
    pub lure: bool,
}

impl Abilities {
    pub const ALL: Abilities = Abilities {
        lunge: true,
        swoon: true,
        cling: true,
        lure: true,
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
    pub attack_pressed: bool,
    pub attack_held: bool,
    pub parry_pressed: bool,
    pub lure_pressed: bool,
    pub lure_held: bool,
}

impl Controls {
    /// One line of an input recording: `x y flags`, where flags are `h` (jump held),
    /// `j` (jump pressed), `l` (Lunge pressed), `a` (attack pressed),
    /// `b` (attack held), `p` (parry pressed), `g` (Lure pressed) and `r` (Lure held),
    /// or `-` for none.
    pub fn encode(&self) -> String {
        let mut flags = String::new();
        for (on, c) in [
            (self.jump_held, 'h'),
            (self.jump_pressed, 'j'),
            (self.lunge_pressed, 'l'),
            (self.attack_pressed, 'a'),
            (self.attack_held, 'b'),
            (self.parry_pressed, 'p'),
            (self.lure_pressed, 'g'),
            (self.lure_held, 'r'),
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
            attack_pressed: flags.contains('a'),
            attack_held: flags.contains('b'),
            parry_pressed: flags.contains('p'),
            lure_pressed: flags.contains('g'),
            lure_held: flags.contains('r'),
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
    /// The Lure took hold of the light at this point.
    pub lured: Option<(f32, f32)>,
    /// Let go of the Lure.
    pub released: bool,
}

impl Events {
    /// Combines the events of several ticks.
    pub fn merge(&mut self, other: Events) {
        self.jumped |= other.jumped;
        self.swooned |= other.swooned;
        self.wall_jumped = self.wall_jumped.or(other.wall_jumped);
        self.lunged |= other.lunged;
        self.lured = self.lured.or(other.lured);
        self.released |= other.released;
        self.landed = match (self.landed, other.landed) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        };
    }
}

/// Movement imposed by a committed attack. The Lunge is the only optional escape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementLock {
    pub vx: f32,
    pub air_hang: bool,
    pub allow_lunge: bool,
}

/// How far above the top of the body the hands reach when hanging, in pixels.
pub const HANDS_ABOVE: f32 = 8.0;

/// The Lure's hold on a light.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rope {
    /// The light held, in pixels with y down.
    pub anchor: (f32, f32),
    /// The rope's length now, from the anchor to the body's hands.
    pub length: f32,
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
    /// The Lure's rope while it holds a light.
    pub rope: Option<Rope>,
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
            rope: None,
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
        self.tick_with_lock(room, t, input, dt, None)
    }

    /// Advances movement with an optional combat lock. Ordinary movement uses [`Self::tick`].
    pub fn tick_with_lock(
        &mut self,
        room: &Room,
        t: &MoveTuning,
        mut input: Controls,
        dt: f32,
        mut lock: Option<MovementLock>,
    ) -> Events {
        let mut ev = Events::default();
        // On the rope an attack swings from it: the rope keeps the body, not the attack.
        if self.rope.is_some() {
            if let Some(l) = lock {
                input.lunge_pressed &= l.allow_lunge;
            }
            lock = None;
        }
        if let Some(lock) = lock {
            input.x = 0.0;
            input.y = 0.0;
            input.jump_pressed = false;
            input.jump_held = true;
            input.lunge_pressed &= lock.allow_lunge;
            self.stop_actions();
        }

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

        // The Lure: take hold of a light, or let go of one.
        if self.rope.is_some() && !input.lure_held {
            self.release(t);
            ev.released = true;
        }
        if input.lure_pressed
            && self.abilities.lure
            && self.rope.is_none()
            && self.lunging <= 0.0
            && let Some(anchor) = self.lure_target(room, t, input)
        {
            let length = distance(self.hands(), anchor);
            self.rope = Some(Rope { anchor, length });
            self.wall = 0;
            self.swoon_ready = self.abilities.swoon;
            self.swooning = false;
            self.lunge_ready = self.abilities.lunge;
            self.jump_cut_done = true;
            ev.lured = Some(anchor);
        }
        if self.rope.is_some() {
            self.wall = 0;
            // Jumping off the rope, or Lunging off it, lets go.
            if self.jump_buffer > 0.0 {
                self.rope = None;
                self.vy = self.vy.min(-t.lure_jump_speed);
                self.jump_buffer = 0.0;
                self.jump_cut_done = false;
                self.grounded = false;
                ev.jumped = true;
                ev.released = true;
            } else if input.lunge_pressed && self.lunge_ready && self.lunge_cooldown <= 0.0 {
                self.rope = None;
                ev.released = true;
            }
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
            lock = None;
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

        if self.rope.is_some() {
            self.swing(t, input, dir, dt);
        } else if self.lunging > 0.0 {
            self.lunging = (self.lunging - dt).max(0.0);
            self.vy = 0.0;
            if self.lunging <= 0.0 {
                self.vx = self.facing as f32 * self.vx.abs().min(t.run_speed);
            }
        } else {
            if let Some(lock) = lock {
                self.vx = lock.vx;
                if lock.air_hang {
                    self.vy = 0.0;
                } else {
                    self.fall(t, input, down, dt);
                }
            } else {
                self.run(t, dir, dt);
                self.fall(t, input, down, dt);
            }
        }

        let airborne = !self.grounded;
        let pop = if self.lunging > 0.0 { t.ledge_pop } else { 0 };
        if self.step_x(room, self.vx * dt, pop) {
            self.vx = 0.0;
            self.lunging = 0.0;
        }
        if self.step_y(room, self.vy * dt, t.corner_correction) {
            if airborne && self.vy > 0.0 && self.rope.is_none() {
                ev.landed = Some(self.vy);
                self.grounded = true;
            }
            self.vy = 0.0;
        }
        // A rope that loses sight of its light snaps.
        if let Some(rope) = self.rope
            && !room.clear_line(self.hands(), rope.anchor)
        {
            self.release(t);
            ev.released = true;
        }
        ev
    }

    /// Where the rope meets the body: the hands, raised above the head.
    pub fn hands(&self) -> (f32, f32) {
        let b = &self.body;
        (
            b.x as f32 + b.w as f32 / 2.0 + b.rem_x,
            b.y as f32 - HANDS_ABOVE + b.rem_y,
        )
    }

    /// The light the Lure would take hold of with this input, if any.
    ///
    /// The stick aims; with the stick at rest, the aim is up and forward.
    /// Of the lights in reach, in sight and inside the cone around the aim,
    /// the one closest to the aim wins, with nearer lights preferred a little.
    /// Only lights above the hands count, since the rope can only hang from them.
    pub fn lure_target(&self, room: &Room, t: &MoveTuning, input: Controls) -> Option<(f32, f32)> {
        let (mut ax, mut ay) = (input.x, -input.y);
        if (ax * ax + ay * ay).sqrt() < t.walk_threshold {
            (ax, ay) = (self.facing as f32, -1.0);
        }
        let aim = ay.atan2(ax);
        let hands = self.hands();
        room.lure_points()
            .into_iter()
            .filter_map(|p| {
                let (dx, dy) = (p.0 - hands.0, p.1 - hands.1);
                let d = (dx * dx + dy * dy).sqrt();
                if d > t.lure_range || dy > -8.0 {
                    return None;
                }
                let off = angle_between(dy.atan2(dx), aim);
                (off <= t.lure_cone && room.clear_line(hands, p))
                    .then_some((off + 0.4 * d / t.lure_range, p))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, p)| p)
    }

    /// One tick on the rope: a pendulum under gravity, pumped by the stick,
    /// reeling in to its swing length and climbed with up and down.
    fn swing(&mut self, t: &MoveTuning, input: Controls, dir: i32, dt: f32) {
        let Some(mut rope) = self.rope else { return };
        let reeling = rope.length > t.lure_swing_length;
        if reeling {
            rope.length = (rope.length - t.lure_reel_speed * dt).max(t.lure_swing_length);
        }
        if input.y > t.down_threshold {
            rope.length = (rope.length - t.lure_climb_speed * dt).max(t.lure_min_length);
        } else if input.y < -t.down_threshold {
            rope.length = (rope.length + t.lure_climb_speed * dt).min(t.lure_range);
        }

        let p = self.hands();
        let (dx, dy) = (p.0 - rope.anchor.0, p.1 - rope.anchor.1);
        let d = (dx * dx + dy * dy).sqrt().max(1.0);
        if self.grounded && d < rope.length {
            // Slack on the ground: walk as usual.
            self.run(t, dir, dt);
        } else {
            // Pumping pushes along the swing, never along the rope.
            let (tx, ty) = (dy / d, -dx / d);
            self.vx += tx * dir as f32 * t.lure_pump * dt;
            self.vy += ty * dir as f32 * t.lure_pump * dt;
        }
        self.vy += t.gravity * dt;

        // Where the body would go, pulled back onto the rope's circle if it would stretch it.
        let (nx, ny) = (p.0 + self.vx * dt, p.1 + self.vy * dt);
        let (ex, ey) = (nx - rope.anchor.0, ny - rope.anchor.1);
        let e = (ex * ex + ey * ey).sqrt().max(1.0);
        if e > rope.length {
            let before = (self.vx * self.vx + self.vy * self.vy).sqrt();
            let k = rope.length / e;
            let (cx, cy) = (rope.anchor.0 + ex * k, rope.anchor.1 + ey * k);
            self.vx = (cx - p.0) / dt;
            self.vy = (cy - p.1) / dt;
            // A rope snapping taut turns the body's flight into the swing instead of stopping it,
            // so momentum carries through a catch. Ordinary ticks on a taut rope lose almost
            // nothing here and keep what the projection gives.
            // The reel's own pull is not momentum to keep.
            let after = (self.vx * self.vx + self.vy * self.vy).sqrt();
            if !reeling && before - after > t.lure_snap_keep && after > 1.0 {
                self.vx *= before / after;
                self.vy *= before / after;
            }
        }
        // When the reel stops, so does its pull along the rope; the swing keeps the rest.
        if reeling && rope.length <= t.lure_swing_length {
            let (ux, uy) = (dx / d, dy / d);
            let radial = self.vx * ux + self.vy * uy;
            if radial < 0.0 {
                self.vx -= radial * ux;
                self.vy -= radial * uy;
            }
        }
        let speed = (self.vx * self.vx + self.vy * self.vy).sqrt();
        if speed > t.lure_max_speed {
            self.vx *= t.lure_max_speed / speed;
            self.vy *= t.lure_max_speed / speed;
        }
        if self.vx.abs() > 1.0 {
            self.facing = self.vx.signum() as i32;
        }
        self.rope = Some(rope);
    }

    /// Lets go of the rope, flung on by the swing.
    fn release(&mut self, t: &MoveTuning) {
        if self.rope.take().is_some() {
            self.vx *= t.lure_release_boost;
            self.vy *= t.lure_release_boost;
            self.jump_cut_done = true;
        }
    }

    /// Ends movement actions when a hit takes control of the body.
    pub fn stop_actions(&mut self) {
        self.lunging = 0.0;
        self.rope = None;
        self.jump_buffer = 0.0;
        self.coyote = 0.0;
        self.wall_coyote = 0.0;
        self.wall_lock = 0.0;
        self.jump_cut_done = true;
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

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// The unsigned difference between two angles, in radians, 0..=PI.
fn angle_between(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(std::f32::consts::TAU);
    d.min(std::f32::consts::TAU - d)
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
            attack_pressed: true,
            attack_held: true,
            parry_pressed: true,
            lure_pressed: true,
            lure_held: false,
        };
        assert_eq!(Controls::decode(&c.encode()), Some(c));
        let none = Controls::default();
        assert_eq!(Controls::decode(&none.encode()), Some(none));
    }
}
