//! Character animation on the pixelfx rig: from how a body is moving to the pose it holds.
//!
//! An [`Animator`] reads a [`Motion`] each frame, picks a state such as running or falling,
//! and gives a rig [`Pose`]. Poses hold on twelve-frames-per-second steps, like the rest of the
//! rig's animation, and a change of state shows one in-between before the new pose.
//! There is no engine here, so the tools can draw contact sheets and the tests run headless.

use std::f64::consts::{PI, TAU};

pub mod guest;
pub mod swing;

use pixelfx::Frame;
use pixelfx::act::lerp_pose;
use pixelfx::color::hex;
use pixelfx::painter::Painter;
pub use pixelfx::rig::{Pose, Rig};

/// Animation steps per second.
pub const FPS: f64 = 12.0;

/// How a body is moving this frame, in pixels and seconds, with y down.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Motion {
    pub grounded: bool,
    pub vx: f32,
    pub vy: f32,
    /// -1 or 1.
    pub facing: i32,
    /// The wall being slid down, -1 or 1, or 0.
    pub wall: i32,
    pub lunging: bool,
    /// In the rise of a Swoon, the second jump.
    pub swooning: bool,
    /// Running speed, which sets the stride.
    pub run_speed: f32,
    /// On the Lure's rope: the direction from the hands to the light, a unit vector with y down.
    pub rope: Option<(f32, f32)>,
}

/// What the body is doing, which decides the pose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Idle,
    Run,
    /// Running one way while still moving the other: a skid.
    Turn,
    Rise,
    Apex,
    Fall,
    /// The crouch for a moment after touching down.
    Land,
    Lunge,
    Swoon,
    WallSlide,
    /// On the Lure's rope, both hands on it.
    Hang,
}

impl State {
    pub const ALL: [Self; 11] = [
        Self::Idle,
        Self::Run,
        Self::Turn,
        Self::Rise,
        Self::Apex,
        Self::Fall,
        Self::Land,
        Self::Lunge,
        Self::Swoon,
        Self::WallSlide,
        Self::Hang,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Run => "run",
            Self::Turn => "turn",
            Self::Rise => "rise",
            Self::Apex => "apex",
            Self::Fall => "fall",
            Self::Land => "land",
            Self::Lunge => "lunge",
            Self::Swoon => "swoon",
            Self::WallSlide => "wall-slide",
            Self::Hang => "hang",
        }
    }
}

/// How long the landing crouch lasts, in seconds.
const LAND_TIME: f64 = 0.12;
/// Vertical speed below which an airborne body is at the top of its arc.
const APEX_SPEED: f32 = 60.0;
/// Pixels of travel per run cycle.
const STRIDE: f64 = 64.0;
/// How long the Swoon's spin takes, in seconds.
const SWOON_SPIN: f64 = 0.4;

/// Picks states from motion and gives the pose for the current one.
#[derive(Clone, Debug)]
pub struct Animator {
    state: State,
    /// Seconds in the current state.
    time: f64,
    /// Where the run cycle is, 0 to 1.
    run_phase: f64,
    /// Seconds of landing crouch left.
    landing: f64,
    /// The pose shown when the state changed, for the one in-between.
    from: Option<Pose>,
    was_grounded: bool,
    facing: i32,
    /// A foot came down during the last update.
    stepped: bool,
    /// For [`State::Hang`]: the rope's direction, and the body's speed across it.
    rope: (f64, f64),
    swing: f64,
}

impl Default for Animator {
    fn default() -> Self {
        Self {
            state: State::Idle,
            time: 0.0,
            run_phase: 0.0,
            landing: 0.0,
            from: None,
            was_grounded: true,
            facing: 1,
            stepped: false,
            rope: (0.0, -1.0),
            swing: 0.0,
        }
    }
}

impl Animator {
    pub fn state(&self) -> State {
        self.state
    }

    /// Whether a running foot came down during the last update, for footstep sounds.
    pub fn stepped(&self) -> bool {
        self.stepped
    }

    /// Advances by `dt` seconds of motion.
    pub fn update(&mut self, m: Motion, dt: f64) {
        if m.grounded && !self.was_grounded {
            self.landing = LAND_TIME;
        }
        self.was_grounded = m.grounded;
        self.landing = (self.landing - dt).max(0.0);
        self.stepped = false;
        if m.grounded {
            let before = self.run_phase;
            self.run_phase = (self.run_phase + (m.vx.abs() as f64 * dt) / STRIDE).fract();
            // A foot lands twice a stride: at the start of the cycle and halfway through it.
            let half = |p: f64| (p * 2.0).floor();
            self.stepped = self.state == State::Run
                && (half(self.run_phase) != half(before) || self.run_phase < before);
        }
        // A wall slide faces away from the wall; everything else faces the way the body does.
        self.facing = if m.wall != 0 { -m.wall } else { m.facing };
        if let Some((x, y)) = m.rope {
            self.rope = (x as f64, y as f64);
            // Speed across the rope, so the legs trail behind the swing.
            self.swing = (m.vx * -y + m.vy * x) as f64;
        }

        let next = self.pick(m);
        if next != self.state {
            self.from = Some(self.pose());
            self.state = next;
            self.time = 0.0;
        } else {
            self.time += dt;
        }
    }

    fn pick(&self, m: Motion) -> State {
        if m.rope.is_some() && !m.grounded {
            return State::Hang;
        }
        if m.lunging {
            return State::Lunge;
        }
        if m.wall != 0 && !m.grounded {
            return State::WallSlide;
        }
        if !m.grounded {
            return if m.swooning
                && (self.state == State::Swoon && self.time < SWOON_SPIN || m.vy < 0.0)
            {
                State::Swoon
            } else if m.vy < -APEX_SPEED {
                State::Rise
            } else if m.vy < APEX_SPEED {
                State::Apex
            } else {
                State::Fall
            };
        }
        if self.landing > 0.0 && m.vx.abs() < m.run_speed * 0.5 {
            return State::Land;
        }
        if m.vx.abs() > 10.0 {
            if m.vx.signum() as i32 != m.facing {
                State::Turn
            } else {
                State::Run
            }
        } else {
            State::Idle
        }
    }

    /// The pose for this frame: the state's pose, after one in-between on a change.
    pub fn pose(&self) -> Pose {
        let step = (self.time * FPS + 1e-6).floor() / FPS;
        let mut pose = if self.state == State::Hang {
            hang(self.rope, self.swing, self.facing)
        } else {
            pose_for(self.state, step, self.run_phase)
        };
        if let Some(from) = &self.from
            && self.time < 1.0 / FPS
        {
            pose = lerp_pose(from, &pose, 0.5);
        }
        pose.face = self.facing;
        pose
    }
}

/// The pose of a state `t` seconds into it; `phase` is where the run cycle is, 0 to 1.
pub fn pose_for(state: State, t: f64, phase: f64) -> Pose {
    match state {
        State::Idle => {
            // A slow breath: the body sinks a pixel and rises again.
            let breath = ((t * TAU / 2.4).sin() * 0.5 + 0.5).round();
            Pose {
                crouch: 1.0 + breath,
                fx: 0.0,
                fy: -15.0 + breath,
                bx: 3.0,
                by: -15.0 + breath,
                ffx: 3.0,
                fbx: -3.0,
                ..Pose::default()
            }
        }
        State::Run => {
            // Run cycles step on the animation rate too, eight steps to a stride.
            let p = (phase * 8.0).floor() / 8.0 * TAU;
            let (s, c) = (p.sin(), p.cos());
            // The foot swinging forward lifts; the one pushing back stays down.
            let lift = |c: f64| -4.0 - (c.max(0.0) * 5.0);
            Pose {
                lean: 4.0,
                crouch: 2.0 + (2.0 * p).cos().abs(),
                ffx: 1.0 + s * 7.0,
                ffy: lift(c),
                fbx: 1.0 - s * 7.0,
                fby: lift(-c),
                fx: 3.0 - s * 6.0,
                fy: -17.0,
                bx: 3.0 + s * 6.0,
                by: -17.0,
                ..Pose::default()
            }
        }
        State::Turn => Pose {
            lean: -4.0,
            crouch: 5.0,
            ffx: 10.0,
            ffy: -4.0,
            fbx: -3.0,
            fby: -6.0,
            fx: -6.0,
            fy: -24.0,
            bx: 6.0,
            by: -20.0,
            ..Pose::default()
        },
        State::Rise => Pose {
            lean: 2.0,
            ffx: 4.0,
            ffy: -13.0,
            fbx: -3.0,
            fby: -5.0,
            fx: 7.0,
            fy: -33.0,
            bx: -5.0,
            by: -25.0,
            ..air()
        },
        State::Apex => Pose {
            lean: 1.0,
            crouch: 2.0,
            ffx: 6.0,
            ffy: -11.0,
            fbx: -4.0,
            fby: -9.0,
            fx: 10.0,
            fy: -27.0,
            bx: -8.0,
            by: -27.0,
            ..air()
        },
        State::Fall => Pose {
            lean: -1.0,
            ffx: 3.0,
            ffy: -4.0,
            fbx: -4.0,
            fby: -8.0,
            fx: 11.0,
            fy: -30.0,
            bx: -9.0,
            by: -31.0,
            ..air()
        },
        State::Land => Pose {
            crouch: 8.0,
            lean: 4.0,
            ffx: 8.0,
            fbx: -7.0,
            fx: 9.0,
            fy: -11.0,
            bx: -3.0,
            by: -12.0,
            ..Pose::default()
        },
        State::Lunge => Pose {
            lean: 8.0,
            crouch: 4.0,
            ffx: 9.0,
            ffy: -6.0,
            fbx: -9.0,
            fby: -8.0,
            fx: -3.0,
            fy: -19.0,
            bx: -6.0,
            by: -21.0,
            ..air()
        },
        State::Swoon => {
            // One slow turn of the whole body, tucked, then open.
            let u = (t / SWOON_SPIN).min(1.0);
            Pose {
                yaw: u * TAU,
                crouch: 4.0 * (u * PI).sin(),
                ffx: 4.0,
                ffy: -12.0,
                fbx: -4.0,
                fby: -10.0,
                fx: 9.0,
                fy: -31.0,
                bx: -9.0,
                by: -31.0,
                ..air()
            }
        }
        State::Hang => hang((0.0, -1.0), 0.0, 1),
        State::WallSlide => Pose {
            lean: -2.0,
            crouch: 3.0,
            // Facing away from the wall, so the wall is behind: the back hand and foot touch it.
            bx: -8.0,
            by: -27.0,
            fbx: -6.0,
            fby: -9.0,
            ffx: 4.0,
            ffy: -6.0,
            fx: 7.0,
            fy: -18.0,
            ..air()
        },
    }
}

/// Hanging from the Lure's rope: both hands up along the rope, the body tilted with it,
/// and the legs trailing behind the swing. `rope` is the direction to the light, y down;
/// `swing` the speed across the rope in pixels per second, positive clockwise on screen.
fn hang(rope: (f64, f64), swing: f64, facing: i32) -> Pose {
    let face = facing as f64;
    // Into the rig's frame, where x points the way the body faces.
    let (rx, ry) = (rope.0 * face, rope.1);
    // Hands at the end of a reach from the shoulders, quantised to whole pixels.
    let reach = |r: f64| (r * 15.0).round();
    let trail = (-swing * face / 60.0).clamp(-6.0, 6.0).round();
    Pose {
        lean: (rx * 8.0).round(),
        crouch: 1.0,
        fx: 1.0 + reach(rx),
        fy: -29.0 + reach(ry),
        bx: -1.0 + reach(rx),
        by: -29.0 + reach(ry),
        ffx: 2.0 + trail,
        ffy: -3.0,
        fbx: -3.0 + trail,
        fby: -6.0,
        ..air()
    }
}

/// A pose off the ground. The rig clips feet at the ground line only when standing,
/// which a body offset of a pixel switches off; [`draw`] moves it back down.
fn air() -> Pose {
    Pose {
        y: -AIR_LIFT,
        ..Pose::default()
    }
}

const AIR_LIFT: f64 = 1.0;

/// The player's rig: the stock figure in a burgundy velvet tailcoat.
pub fn player_rig() -> Rig {
    Rig {
        shell: [hex(0xe8b4bc), hex(0xb46a7c), hex(0x7c3c50), hex(0x4c2034)],
        core: [hex(0x5c3a4a), hex(0x402834), hex(0x2c1a24), hex(0x2c1a24)],
        tails: 14.0,
        ..Rig::default()
    }
}

/// Draws a pose into a frame with the feet at `origin`.
pub fn draw(rig: &Rig, pose: &Pose, frame: &mut Frame, origin: [f64; 2]) {
    let mut p = Painter::new(frame);
    let lift = if pose.y < -0.5 { -pose.y } else { 0.0 };
    p.place(
        origin[0] + pose.x.round(),
        origin[1] + pose.y.round() + lift.min(AIR_LIFT),
        false,
        0.0,
    );
    rig.render(pose).draw(&mut p);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn motion() -> Motion {
        Motion {
            grounded: true,
            facing: 1,
            run_speed: 150.0,
            ..Motion::default()
        }
    }

    fn run(a: &mut Animator, m: Motion, secs: f64) {
        for _ in 0..(secs * 120.0) as usize {
            a.update(m, 1.0 / 120.0);
        }
    }

    #[test]
    fn states_follow_motion() {
        let mut a = Animator::default();
        run(&mut a, motion(), 0.2);
        assert_eq!(a.state(), State::Idle);
        run(
            &mut a,
            Motion {
                vx: 150.0,
                ..motion()
            },
            0.2,
        );
        assert_eq!(a.state(), State::Run);
        run(
            &mut a,
            Motion {
                vx: 150.0,
                facing: -1,
                ..motion()
            },
            0.05,
        );
        assert_eq!(a.state(), State::Turn);
        let air = Motion {
            grounded: false,
            ..motion()
        };
        run(&mut a, Motion { vy: -300.0, ..air }, 0.1);
        assert_eq!(a.state(), State::Rise);
        run(&mut a, Motion { vy: 0.0, ..air }, 0.05);
        assert_eq!(a.state(), State::Apex);
        run(&mut a, Motion { vy: 300.0, ..air }, 0.1);
        assert_eq!(a.state(), State::Fall);
        run(&mut a, motion(), 0.05);
        assert_eq!(a.state(), State::Land);
        run(&mut a, motion(), 0.2);
        assert_eq!(a.state(), State::Idle);
        run(
            &mut a,
            Motion {
                lunging: true,
                ..motion()
            },
            0.05,
        );
        assert_eq!(a.state(), State::Lunge);
        run(
            &mut a,
            Motion {
                wall: 1,
                vy: 60.0,
                ..air
            },
            0.05,
        );
        assert_eq!(a.state(), State::WallSlide);
        assert_eq!(a.pose().face, -1, "a wall slide faces away from the wall");
    }

    #[test]
    fn running_steps_twice_a_stride() {
        let mut a = Animator::default();
        let m = Motion {
            vx: 150.0,
            ..motion()
        };
        let mut steps = 0;
        for _ in 0..120 {
            a.update(m, 1.0 / 120.0);
            steps += a.stepped() as u32;
        }
        // 150 px in a second over a 64 px stride is about 2.3 strides, so 4 or 5 steps.
        assert!((4..=5).contains(&steps), "{steps} steps in a second");
        a.update(
            Motion {
                vx: 0.0,
                ..motion()
            },
            0.5,
        );
        assert!(!a.stepped());
    }

    #[test]
    fn the_swoon_spins_once_then_falls() {
        let mut a = Animator::default();
        let air = Motion {
            grounded: false,
            swooning: true,
            ..motion()
        };
        run(&mut a, Motion { vy: -200.0, ..air }, 0.2);
        assert_eq!(a.state(), State::Swoon);
        run(&mut a, Motion { vy: 200.0, ..air }, 0.3);
        assert_eq!(a.state(), State::Fall);
    }

    #[test]
    fn poses_hold_on_animation_steps() {
        let mut a = Animator::default();
        let m = Motion {
            grounded: false,
            vy: -300.0,
            ..motion()
        };
        run(&mut a, m, 0.2);
        let first = a.pose();
        a.update(m, 0.001);
        assert_eq!(
            a.pose(),
            first,
            "the pose changed within one animation step"
        );
    }

    #[test]
    fn every_pose_reaches_its_hands_and_feet() {
        let rig = player_rig();
        for state in State::ALL {
            for i in 0..8 {
                let pose = pose_for(state, i as f64 / FPS, i as f64 / 8.0);
                let miss = rig.shortfall(&pose);
                assert!(
                    miss < 3.0,
                    "{} misses its hands by {miss:.1} px",
                    state.name()
                );
            }
        }
    }
}
