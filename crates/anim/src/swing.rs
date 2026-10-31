//! Weapon swings, posed by where an attack is in its frame data rather than by the clock.
//!
//! Every swing has four key poses: ready, coiled at the end of the wind-up, the release on the
//! first active frame, and the follow-through at the end of the active frames. The wind-up eases
//! into the coil, the release snaps, which is the big change of silhouette a heavy hit needs,
//! and the recovery eases back to ready. Because the poses follow the attack's phases,
//! retuning the frame data retimes the animation with it.

use pixelfx::act::lerp_pose;
use pixelfx::rig::{Pose, Weapon};

/// A swing of the sabre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Swing {
    /// The first combo hit: a forehand cut from high behind to low in front.
    Combo1,
    /// The second: a backhand cut rising from low in front.
    Combo2,
    /// The third, slower and heavier: a two-handed overhead smash.
    Combo3,
    /// Held, then released into a thrust.
    Charged,
    /// An uppercut that launches light enemies.
    Rising,
    Air1,
    Air2,
    /// Straight down in the air, to bounce off what it hits.
    Pogo,
}

impl Swing {
    pub const ALL: [Self; 8] = [
        Self::Combo1,
        Self::Combo2,
        Self::Combo3,
        Self::Charged,
        Self::Rising,
        Self::Air1,
        Self::Air2,
        Self::Pogo,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Combo1 => "combo-1",
            Self::Combo2 => "combo-2",
            Self::Combo3 => "combo-3",
            Self::Charged => "charged",
            Self::Rising => "rising",
            Self::Air1 => "air-1",
            Self::Air2 => "air-2",
            Self::Pogo => "pogo",
        }
    }

    fn airborne(self) -> bool {
        matches!(self, Self::Air1 | Self::Air2 | Self::Pogo)
    }
}

/// Where an attack is in its frame data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Windup,
    Active,
    Recovery,
}

/// The sabre held at the ready, standing.
pub fn ready() -> Pose {
    Pose {
        weapon: Some(Weapon::Sword),
        sa: -0.85,
        ..Pose::ready()
    }
}

/// The Toast: the blade raised high, like a glass, to meet a blow.
pub fn toast() -> Pose {
    Pose {
        weapon: Some(Weapon::Sword),
        crouch: 2.0,
        lean: -2.0,
        fx: 7.0,
        fy: -37.0,
        bx: -4.0,
        by: -24.0,
        sa: -1.35,
        ffx: 6.0,
        fbx: -6.0,
        ..Pose::default()
    }
}

/// The sabre held at the ready in the air.
fn ready_air() -> Pose {
    Pose {
        y: -1.0,
        ffx: 5.0,
        ffy: -11.0,
        fbx: -4.0,
        fby: -8.0,
        ..ready()
    }
}

struct Keys {
    coil: Pose,
    release: Pose,
    follow: Pose,
}

fn armed(p: Pose) -> Pose {
    Pose {
        weapon: Some(Weapon::Sword),
        ..p
    }
}

fn keys(s: Swing) -> Keys {
    let base = if s.airborne() { ready_air() } else { ready() };
    let k = |p: Pose| armed(p);
    match s {
        Swing::Combo1 | Swing::Air1 => Keys {
            coil: k(Pose {
                crouch: 4.0,
                lean: -4.0,
                x: -3.0,
                fx: -6.0,
                fy: -34.0,
                bx: 8.0,
                by: -25.0,
                sa: -2.6,
                arms_front: true,
                ..base.clone()
            }),
            release: k(Pose {
                crouch: 2.0,
                lean: 3.0,
                fx: 16.0,
                fy: -33.0,
                bx: -7.0,
                by: -24.0,
                sa: -1.2,
                ghost: 1.0,
                ..base.clone()
            }),
            follow: k(Pose {
                crouch: 5.0,
                lean: 5.0,
                x: 2.0,
                fx: 18.0,
                fy: -20.0,
                bx: -9.0,
                by: -24.0,
                sa: 0.65,
                toe_b: !s.airborne(),
                ..base
            }),
        },
        Swing::Combo2 | Swing::Air2 => Keys {
            coil: k(Pose {
                crouch: 6.0,
                lean: 3.0,
                fx: 14.0,
                fy: -13.0,
                bx: -6.0,
                by: -20.0,
                sa: 1.3,
                ..base.clone()
            }),
            release: k(Pose {
                crouch: 3.0,
                fx: 14.0,
                fy: -29.0,
                bx: -6.0,
                by: -24.0,
                sa: -0.6,
                ghost: 1.0,
                ..base.clone()
            }),
            follow: k(Pose {
                crouch: 2.0,
                lean: -2.0,
                fx: 8.0,
                fy: -37.0,
                bx: -8.0,
                by: -26.0,
                sa: -2.0,
                ..base
            }),
        },
        Swing::Combo3 => Keys {
            coil: k(Pose {
                crouch: 2.0,
                lean: -7.0,
                x: -4.0,
                fx: -3.0,
                fy: -38.0,
                bx: -1.0,
                by: -37.0,
                two: true,
                sa: -2.9,
                ffx: 8.0,
                fbx: -9.0,
                ..base.clone()
            }),
            release: k(Pose {
                crouch: 6.0,
                lean: 4.0,
                fx: 14.0,
                fy: -32.0,
                two: true,
                sa: -1.0,
                ghost: 1.0,
                ffx: 12.0,
                fbx: -10.0,
                ..base.clone()
            }),
            follow: k(Pose {
                crouch: 9.0,
                lean: 8.0,
                x: 4.0,
                fx: 17.0,
                fy: -11.0,
                two: true,
                sa: 1.1,
                ffx: 13.0,
                fbx: -11.0,
                toe_b: true,
                ..base
            }),
        },
        Swing::Charged => Keys {
            coil: k(Pose {
                crouch: 7.0,
                lean: -4.0,
                x: -4.0,
                fx: -7.0,
                fy: -22.0,
                bx: 8.0,
                by: -24.0,
                sa: 3.0,
                ffx: 10.0,
                fbx: -10.0,
                ..base.clone()
            }),
            release: k(Pose {
                crouch: 5.0,
                lean: 8.0,
                x: 6.0,
                fx: 21.0,
                fy: -24.0,
                bx: -9.0,
                by: -25.0,
                sa: 0.0,
                ghost: 1.0,
                ffx: 14.0,
                fbx: -12.0,
                ..base.clone()
            }),
            follow: k(Pose {
                crouch: 6.0,
                lean: 7.0,
                x: 6.0,
                fx: 20.0,
                fy: -23.0,
                bx: -9.0,
                by: -25.0,
                sa: 0.1,
                ffx: 14.0,
                fbx: -12.0,
                toe_b: true,
                ..base
            }),
        },
        Swing::Rising => Keys {
            coil: k(Pose {
                crouch: 9.0,
                lean: 4.0,
                fx: 13.0,
                fy: -11.0,
                bx: -6.0,
                by: -18.0,
                sa: 0.75,
                ffx: 9.0,
                fbx: -9.0,
                ..base.clone()
            }),
            release: k(Pose {
                crouch: 1.0,
                lean: -2.0,
                fx: 8.0,
                fy: -38.0,
                bx: -6.0,
                by: -27.0,
                sa: -1.4,
                ghost: 1.0,
                toe_f: true,
                ..base.clone()
            }),
            follow: k(Pose {
                lean: -3.0,
                fx: 3.0,
                fy: -39.0,
                bx: -7.0,
                by: -27.0,
                sa: -1.9,
                ..base
            }),
        },
        Swing::Pogo => Keys {
            coil: k(Pose {
                crouch: 3.0,
                fx: 8.0,
                fy: -35.0,
                bx: -6.0,
                by: -28.0,
                sa: -1.6,
                ..base.clone()
            }),
            release: k(Pose {
                crouch: 5.0,
                lean: 2.0,
                fx: 6.0,
                fy: -10.0,
                bx: -7.0,
                by: -22.0,
                sa: 1.55,
                ghost: 1.0,
                ffy: -14.0,
                fby: -12.0,
                ..base.clone()
            }),
            follow: k(Pose {
                crouch: 5.0,
                lean: 2.0,
                fx: 6.0,
                fy: -10.0,
                bx: -7.0,
                by: -22.0,
                sa: 1.55,
                ffy: -14.0,
                fby: -12.0,
                ..base
            }),
        },
    }
}

fn ease_out(u: f64) -> f64 {
    1.0 - (1.0 - u) * (1.0 - u)
}

fn smooth(u: f64) -> f64 {
    u * u * (3.0 - 2.0 * u)
}

/// The pose of a swing `u` of the way through a phase, 0 to 1.
pub fn swing_pose(s: Swing, phase: Phase, u: f64) -> Pose {
    let u = u.clamp(0.0, 1.0);
    let k = keys(s);
    let base = if s.airborne() { ready_air() } else { ready() };
    match phase {
        // Most of the wind-up is spent getting into the coil, then it holds, tense, until release.
        Phase::Windup => lerp_pose(&base, &k.coil, ease_out((u / 0.7).min(1.0))),
        Phase::Active => lerp_pose(&k.release, &k.follow, ease_out(u)),
        Phase::Recovery => lerp_pose(&k.follow, &base, smooth(u)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player_rig;

    #[test]
    fn every_swing_reaches_its_hands() {
        let rig = player_rig();
        for s in Swing::ALL {
            for phase in [Phase::Windup, Phase::Active, Phase::Recovery] {
                for i in 0..=4 {
                    let pose = swing_pose(s, phase, i as f64 / 4.0);
                    let miss = rig.shortfall(&pose);
                    assert!(miss < 3.0, "{} {phase:?} misses by {miss:.1} px", s.name());
                }
            }
        }
    }

    #[test]
    fn the_toast_reaches_its_hands() {
        assert!(player_rig().shortfall(&toast()) < 3.0);
    }

    #[test]
    fn the_release_snaps_from_the_coil() {
        // The first active frame is far from the end of the wind-up: that jump is the hit.
        for s in Swing::ALL {
            let coil = swing_pose(s, Phase::Windup, 1.0);
            let release = swing_pose(s, Phase::Active, 0.0);
            assert!(
                (coil.sa - release.sa).abs() > 1.0,
                "{} barely moves the blade on release",
                s.name()
            );
        }
    }
}
