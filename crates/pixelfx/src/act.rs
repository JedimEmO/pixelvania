//! Wind-up, recovery, hit-stop events and poses held on twelve-frame-per-second steps.
// Ported from pixelfx core/act.js. The freeze, shake and poses held on 12 fps steps follow
// Hollow Crown (https://github.com/rehan-remade/hollow-crown), MIT License,
// Copyright (c) 2026 rehan-remade.
use crate::{
    effects::EffectId,
    math::lerp,
    rig::{Pose, Weapon},
};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Arrival {
    #[default]
    Smooth,
    Out,
    In,
    Linear,
    Snap,
}
impl Arrival {
    fn ease(self, u: f64) -> f64 {
        match self {
            Self::Out => 1.0 - (1.0 - u) * (1.0 - u),
            Self::In => u * u,
            Self::Linear => u,
            _ => u * u * (3.0 - 2.0 * u),
        }
    }
}
#[derive(Clone, Debug)]
pub struct PoseKey {
    pub time: f64,
    pub pose: Pose,
    pub arrival: Arrival,
}
#[derive(Clone, Debug)]
pub struct Hit {
    pub time: f64,
    pub freeze: f64,
    pub shake: f64,
    pub amplitude: f64,
    pub push: f64,
}
#[derive(Clone, Debug)]
pub struct ActTiming {
    pub windup: f64,
    pub recovery: f64,
    pub duration: f64,
    pub targets: Vec<f64>,
    pub hits: Vec<Hit>,
    pub poses: Vec<PoseKey>,
}
/// Interpolates numeric pose values. Flags and weapons change at a key boundary.
pub fn lerp_pose(a: &Pose, b: &Pose, u: f64) -> Pose {
    let mut out = a.clone();
    out.x = lerp(a.x, b.x, u);
    out.y = lerp(a.y, b.y, u);
    out.lean = lerp(a.lean, b.lean, u);
    out.crouch = lerp(a.crouch, b.crouch, u);
    out.yaw = lerp(a.yaw, b.yaw, u);
    out.fx = lerp(a.fx, b.fx, u);
    out.fy = lerp(a.fy, b.fy, u);
    out.bx = lerp(a.bx, b.bx, u);
    out.by = lerp(a.by, b.by, u);
    out.ffx = lerp(a.ffx, b.ffx, u);
    out.ffy = lerp(a.ffy, b.ffy, u);
    out.fbx = lerp(a.fbx, b.fbx, u);
    out.fby = lerp(a.fby, b.fby, u);
    out.hx = lerp(a.hx, b.hx, u);
    out.hy = lerp(a.hy, b.hy, u);
    out.twist = lerp(a.twist, b.twist, u);
    out.sa = lerp(a.sa, b.sa, u);
    out.sb = lerp(a.sb, b.sb, u);
    out.wl = lerp(a.wl, b.wl, u);
    out.wz = lerp(a.wz, b.wz, u);
    out.ghost = lerp(a.ghost, b.ghost, u);
    out.turn = match (a.turn, b.turn) {
        (Some(a), Some(b)) => Some(lerp(a, b, u)),
        (None, Some(b)) => Some(b),
        (a, None) => a,
    };
    out.fz = match (a.fz, b.fz) {
        (Some(a), Some(b)) => Some(lerp(a, b, u)),
        (None, Some(b)) => Some(b),
        (a, None) => a,
    };
    out.bz = match (a.bz, b.bz) {
        (Some(a), Some(b)) => Some(lerp(a, b, u)),
        (None, Some(b)) => Some(b),
        (a, None) => a,
    };
    out.fzz = match (a.fzz, b.fzz) {
        (Some(a), Some(b)) => Some(lerp(a, b, u)),
        (None, Some(b)) => Some(b),
        (a, None) => a,
    };
    out.bzz = match (a.bzz, b.bzz) {
        (Some(a), Some(b)) => Some(lerp(a, b, u)),
        (None, Some(b)) => Some(b),
        (a, None) => a,
    };
    out
}
/// Returns a pose at effect time, including negative wind-up time.
pub fn pose_at(track: &[PoseKey], t: f64) -> Pose {
    assert!(!track.is_empty());
    if t <= track[0].time {
        return track[0].pose.clone();
    }
    for pair in track.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        if t >= b.time {
            continue;
        }
        if b.arrival == Arrival::Snap {
            return a.pose.clone();
        }
        let stepped = a.time + ((t - a.time) * 12.0 + 1e-6).floor() / 12.0;
        let mut out = lerp_pose(
            &a.pose,
            &b.pose,
            b.arrival.ease((stepped - a.time) / (b.time - a.time)),
        );
        let u = b.arrival.ease((t - a.time) / (b.time - a.time));
        out.x = lerp(a.pose.x, b.pose.x, u);
        out.y = lerp(a.pose.y, b.pose.y, u);
        return out;
    }
    track.last().unwrap().pose.clone()
}
/// An act clock that pauses effect time during hit-stop and emits each hit once.
/// Advance the effect by the difference between successive `time()` values.
#[derive(Clone, Debug)]
pub struct ActClock {
    pub timing: ActTiming,
    time: f64,
    stop: f64,
    next_hit: usize,
}
impl ActClock {
    pub fn new(timing: ActTiming) -> Self {
        let time = -timing.windup;
        Self {
            timing,
            time,
            stop: 0.0,
            next_hit: 0,
        }
    }
    pub fn time(&self) -> f64 {
        self.time
    }
    pub fn frozen(&self) -> bool {
        self.stop > 0.0
    }
    pub fn update(&mut self, dt: f64) -> Vec<Hit> {
        assert!(dt.is_finite() && dt >= 0.0);
        if self.stop > 0.0 {
            self.stop -= dt;
            return vec![];
        }
        self.time += dt;
        let mut hits = vec![];
        while let Some(hit) = self
            .timing
            .hits
            .get(self.next_hit)
            .filter(|h| h.time <= self.time)
        {
            self.stop = self.stop.max(hit.freeze);
            hits.push(hit.clone());
            self.next_hit += 1;
        }
        hits
    }
    pub fn pose(&self) -> Pose {
        pose_at(&self.timing.poses, self.time)
    }
    pub fn finished(&self) -> bool {
        self.time >= self.timing.duration + self.timing.recovery && self.stop <= 0.0
    }
}
/// Timing and poses from the stock effects. Effects without an act return `None`.
pub fn timing(id: EffectId) -> Option<ActTiming> {
    match id {
        EffectId::HeavyLanding => Some(ActTiming {
            windup: 0.26,
            recovery: 0.0,
            duration: id.metadata().duration,
            targets: vec![],
            hits: vec![Hit {
                time: 0.005,
                freeze: 0.13,
                shake: 0.32,
                amplitude: 3.0,
                push: 1.2,
            }],
            poses: vec![
                PoseKey {
                    time: -0.26,
                    arrival: Arrival::Smooth,
                    pose: Pose {
                        turn: Some(0.8),
                        y: -46.0,
                        crouch: -1.0,
                        lean: -1.0,
                        fx: -9.0,
                        fy: -42.0,
                        bx: 10.0,
                        by: -42.0,
                        ffx: 6.0,
                        ffy: -8.0,
                        fbx: -3.0,
                        fby: -3.0,
                        toe_b: true,
                        ghost: 1.0,
                        ..Pose::default()
                    },
                },
                PoseKey {
                    time: -0.01,
                    arrival: Arrival::In,
                    pose: Pose {
                        turn: Some(0.8),
                        y: -6.0,
                        crouch: -1.0,
                        lean: -1.0,
                        fx: -9.0,
                        fy: -42.0,
                        bx: 10.0,
                        by: -42.0,
                        ffx: 6.0,
                        ffy: -8.0,
                        fbx: -3.0,
                        fby: -3.0,
                        toe_b: true,
                        ghost: 1.0,
                        ..Pose::default()
                    },
                },
                PoseKey {
                    time: 0.0,
                    arrival: Arrival::Snap,
                    pose: Pose {
                        y: 0.0,
                        crouch: 8.0,
                        lean: 5.0,
                        fx: 13.0,
                        fy: -8.0,
                        bx: -6.0,
                        by: -23.0,
                        ffx: 11.0,
                        fbx: -10.0,
                        ..Pose::default()
                    },
                },
                PoseKey {
                    time: 0.55,
                    arrival: Arrival::Smooth,
                    pose: Pose {
                        y: 0.0,
                        crouch: 7.0,
                        lean: 5.0,
                        fx: 13.0,
                        fy: -8.0,
                        bx: -6.0,
                        by: -23.0,
                        ffx: 11.0,
                        fbx: -10.0,
                        ..Pose::default()
                    },
                },
                PoseKey {
                    time: 0.95,
                    arrival: Arrival::Smooth,
                    pose: Pose {
                        y: 0.0,
                        crouch: 5.0,
                        lean: 3.0,
                        fx: 13.0,
                        fy: -15.0,
                        bx: -5.0,
                        by: -20.0,
                        ffx: 11.0,
                        fbx: -10.0,
                        ..Pose::default()
                    },
                },
                PoseKey {
                    time: 1.3,
                    arrival: Arrival::Smooth,
                    pose: Pose {
                        crouch: 3.0,
                        fx: 11.0,
                        fy: -21.0,
                        bx: 9.0,
                        by: -21.0,
                        ffx: 7.0,
                        fbx: -7.0,
                        ..Pose::default()
                    },
                },
                PoseKey {
                    time: 1.6,
                    arrival: Arrival::Smooth,
                    pose: Pose {
                        crouch: 3.0,
                        fx: 11.0,
                        fy: -21.0,
                        bx: 9.0,
                        by: -21.0,
                        ffx: 7.0,
                        fbx: -7.0,
                        ..Pose::default()
                    },
                },
            ],
        }),
        EffectId::SwordSlash => Some(ActTiming {
            windup: 0.55,
            recovery: 0.10999999999999999,
            duration: id.metadata().duration,
            targets: vec![58.0],
            hits: vec![Hit {
                time: 0.06,
                freeze: 0.14,
                shake: 0.18,
                amplitude: 2.0,
                push: 2.8,
            }],
            poses: vec![
                PoseKey {
                    time: -0.55,
                    arrival: Arrival::Smooth,
                    pose: Pose {
                        crouch: 3.0,
                        fx: 12.0,
                        fy: -21.0,
                        bx: 9.0,
                        by: -21.0,
                        ffx: 7.0,
                        fbx: -7.0,
                        weapon: Some(Weapon::Sword),
                        sa: -0.85,
                        x: -8.0,
                        ..Pose::default()
                    },
                },
                PoseKey {
                    time: -0.3,
                    arrival: Arrival::Smooth,
                    pose: Pose {
                        crouch: 4.0,
                        fx: -7.0,
                        fy: -35.0,
                        bx: 9.0,
                        by: -25.0,
                        ffx: 9.0,
                        fbx: -8.0,
                        weapon: Some(Weapon::Sword),
                        sa: -2.65,
                        x: -8.0,
                        lean: -4.0,
                        arms_front: true,
                        ..Pose::default()
                    },
                },
                PoseKey {
                    time: -0.08,
                    arrival: Arrival::Smooth,
                    pose: Pose {
                        crouch: 5.0,
                        fx: -7.0,
                        fy: -35.0,
                        bx: 9.0,
                        by: -25.0,
                        ffx: 9.0,
                        fbx: -8.0,
                        weapon: Some(Weapon::Sword),
                        sa: -2.65,
                        x: -8.0,
                        lean: -5.0,
                        arms_front: true,
                        ..Pose::default()
                    },
                },
                PoseKey {
                    time: 0.0,
                    arrival: Arrival::Snap,
                    pose: Pose {
                        crouch: 2.0,
                        fx: 16.0,
                        fy: -37.0,
                        bx: -7.0,
                        by: -24.0,
                        ffx: 11.0,
                        fbx: -10.0,
                        weapon: Some(Weapon::Sword),
                        sa: -1.45,
                        x: 0.0,
                        lean: 3.0,
                        ghost: 1.0,
                        ..Pose::default()
                    },
                },
                PoseKey {
                    time: 0.06,
                    arrival: Arrival::Snap,
                    pose: Pose {
                        crouch: 5.0,
                        fx: 20.0,
                        fy: -21.0,
                        bx: -9.0,
                        by: -24.0,
                        ffx: 13.0,
                        fbx: -10.0,
                        weapon: Some(Weapon::Sword),
                        sa: 0.62,
                        x: 2.0,
                        lean: 5.0,
                        toe_b: true,
                        ..Pose::default()
                    },
                },
                PoseKey {
                    time: 0.3,
                    arrival: Arrival::Smooth,
                    pose: Pose {
                        crouch: 6.0,
                        fx: 20.0,
                        fy: -21.0,
                        bx: -9.0,
                        by: -24.0,
                        ffx: 13.0,
                        fbx: -10.0,
                        weapon: Some(Weapon::Sword),
                        sa: 0.62,
                        x: 3.0,
                        lean: 5.0,
                        toe_b: true,
                        ..Pose::default()
                    },
                },
                PoseKey {
                    time: 0.58,
                    arrival: Arrival::Smooth,
                    pose: Pose {
                        crouch: 3.0,
                        fx: 15.0,
                        fy: -21.0,
                        bx: -5.0,
                        by: -21.0,
                        ffx: 9.0,
                        fbx: -7.0,
                        weapon: Some(Weapon::Sword),
                        sa: 0.1,
                        x: 1.0,
                        ..Pose::default()
                    },
                },
                PoseKey {
                    time: 0.86,
                    arrival: Arrival::Smooth,
                    pose: Pose {
                        crouch: 3.0,
                        fx: 12.0,
                        fy: -21.0,
                        bx: 9.0,
                        by: -21.0,
                        ffx: 7.0,
                        fbx: -7.0,
                        weapon: Some(Weapon::Sword),
                        sa: -0.85,
                        x: -8.0,
                        ..Pose::default()
                    },
                },
            ],
        }),
        EffectId::HammerSmash => Some(ActTiming {
            windup: 0.72,
            recovery: 0.0,
            duration: id.metadata().duration,
            targets: vec![66.0],
            hits: vec![Hit {
                time: 0.15,
                freeze: 0.2,
                shake: 0.34,
                amplitude: 3.0,
                push: 3.0,
            }],
            poses: vec![
                PoseKey {
                    time: -0.72,
                    arrival: Arrival::Smooth,
                    pose: Pose {
                        crouch: 3.0,
                        fx: 11.0,
                        fy: -21.0,
                        bx: 9.0,
                        by: -21.0,
                        ffx: 7.0,
                        fbx: -7.0,
                        weapon: Some(Weapon::Hammer),
                        two: true,
                        sa: 0.62,
                        x: -3.0,
                        ..Pose::default()
                    },
                },
                PoseKey {
                    time: -0.44,
                    arrival: Arrival::Smooth,
                    pose: Pose {
                        crouch: 5.0,
                        fx: 16.0,
                        fy: -28.0,
                        bx: 9.0,
                        by: -21.0,
                        ffx: 7.0,
                        fbx: -7.0,
                        weapon: Some(Weapon::Hammer),
                        two: true,
                        sa: -0.9,
                        x: -3.0,
                        lean: -1.0,
                        ..Pose::default()
                    },
                },
                PoseKey {
                    time: -0.2,
                    arrival: Arrival::Smooth,
                    pose: Pose {
                        crouch: 0.0,
                        fx: 3.0,
                        fy: -41.0,
                        bx: 9.0,
                        by: -21.0,
                        ffx: 8.0,
                        fbx: -5.0,
                        weapon: Some(Weapon::Hammer),
                        two: true,
                        sa: -2.1,
                        x: -3.0,
                        lean: -3.0,
                        ffy: -8.0,
                        toe_b: true,
                        ..Pose::default()
                    },
                },
                PoseKey {
                    time: -0.03,
                    arrival: Arrival::Smooth,
                    pose: Pose {
                        crouch: 0.0,
                        fx: 3.0,
                        fy: -41.0,
                        bx: 9.0,
                        by: -21.0,
                        ffx: 8.0,
                        fbx: -5.0,
                        weapon: Some(Weapon::Hammer),
                        two: true,
                        sa: -2.1,
                        x: -3.0,
                        lean: -3.0,
                        ffy: -8.0,
                        toe_b: true,
                        y: -2.0,
                        ..Pose::default()
                    },
                },
                PoseKey {
                    time: 0.06,
                    arrival: Arrival::Snap,
                    pose: Pose {
                        crouch: -1.0,
                        fx: 13.0,
                        fy: -41.0,
                        bx: 9.0,
                        by: -21.0,
                        ffx: 11.0,
                        fbx: -7.0,
                        weapon: Some(Weapon::Hammer),
                        two: true,
                        sa: -1.2,
                        x: 0.0,
                        y: -3.0,
                        lean: 2.0,
                        ffy: -8.0,
                        ghost: 1.0,
                        ..Pose::default()
                    },
                },
                PoseKey {
                    time: 0.11,
                    arrival: Arrival::Snap,
                    pose: Pose {
                        crouch: 3.0,
                        fx: 19.0,
                        fy: -30.0,
                        bx: 9.0,
                        by: -21.0,
                        ffx: 13.0,
                        fbx: -7.0,
                        weapon: Some(Weapon::Hammer),
                        two: true,
                        sa: -0.15,
                        x: 0.0,
                        y: -1.0,
                        lean: 5.0,
                        ghost: 1.0,
                        ..Pose::default()
                    },
                },
                PoseKey {
                    time: 0.15,
                    arrival: Arrival::Snap,
                    pose: Pose {
                        crouch: 7.0,
                        fx: 14.0,
                        fy: -19.0,
                        bx: 9.0,
                        by: -21.0,
                        ffx: 11.0,
                        fbx: -10.0,
                        weapon: Some(Weapon::Hammer),
                        two: true,
                        sa: 0.62,
                        x: -1.0,
                        lean: 6.0,
                        ..Pose::default()
                    },
                },
                PoseKey {
                    time: 0.62,
                    arrival: Arrival::Smooth,
                    pose: Pose {
                        crouch: 6.0,
                        fx: 14.0,
                        fy: -19.0,
                        bx: 9.0,
                        by: -21.0,
                        ffx: 11.0,
                        fbx: -10.0,
                        weapon: Some(Weapon::Hammer),
                        two: true,
                        sa: 0.62,
                        x: -1.0,
                        lean: 5.0,
                        ..Pose::default()
                    },
                },
                PoseKey {
                    time: 1.02,
                    arrival: Arrival::Smooth,
                    pose: Pose {
                        crouch: 5.0,
                        fx: 14.0,
                        fy: -24.0,
                        bx: 9.0,
                        by: -21.0,
                        ffx: 7.0,
                        fbx: -7.0,
                        weapon: Some(Weapon::Hammer),
                        two: true,
                        sa: 0.3,
                        x: -2.0,
                        lean: 2.0,
                        ..Pose::default()
                    },
                },
                PoseKey {
                    time: 1.4,
                    arrival: Arrival::Smooth,
                    pose: Pose {
                        crouch: 3.0,
                        fx: 11.0,
                        fy: -21.0,
                        bx: 9.0,
                        by: -21.0,
                        ffx: 7.0,
                        fbx: -7.0,
                        weapon: Some(Weapon::Hammer),
                        two: true,
                        sa: 0.62,
                        x: -3.0,
                        ..Pose::default()
                    },
                },
            ],
        }),
        _ => None,
    }
}
