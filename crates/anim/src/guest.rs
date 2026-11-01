//! Guest costumes, proportions and poses.

use std::f64::consts::TAU;

use pixelfx::color::hex;
use pixelfx::rig::{Hair, Hat, Mask, Measurements, Pose, Rig, Weapon};

/// A guest type, with its own proportions and costume.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Light,
    Medium,
    Heavy,
    Spinner,
    Toaster,
}

impl Class {
    pub const ALL: [Self; 5] = [
        Self::Light,
        Self::Medium,
        Self::Heavy,
        Self::Spinner,
        Self::Toaster,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Medium => "medium",
            Self::Heavy => "heavy",
            Self::Spinner => "spinner",
            Self::Toaster => "toaster",
        }
    }

    /// Size relative to the player.
    pub fn scale(self) -> f64 {
        match self {
            Self::Light | Self::Spinner => 0.85,
            Self::Medium | Self::Toaster => 1.0,
            Self::Heavy => 1.25,
        }
    }

    pub fn weapon(self) -> Weapon {
        match self {
            Self::Light => Weapon::Dagger,
            Self::Medium => Weapon::Spear,
            Self::Heavy => Weapon::Hammer,
            Self::Spinner => Weapon::Handle,
            Self::Toaster => Weapon::Coupe,
        }
    }

    /// Hurtbox width and height in pixels, standing.
    pub fn size(self) -> (i32, i32) {
        match self {
            Self::Light | Self::Spinner => (10, 29),
            Self::Medium | Self::Toaster => (12, 34),
            Self::Heavy => (18, 43),
        }
    }
}

/// How a body is reacting to being hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reaction {
    Flinch,
    Stagger,
    Knockdown,
    Launched,
    /// Flat against a wall after being thrown into it.
    Splat,
    Dead,
}

impl Reaction {
    pub const ALL: [Self; 6] = [
        Self::Flinch,
        Self::Stagger,
        Self::Knockdown,
        Self::Launched,
        Self::Splat,
        Self::Dead,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Flinch => "flinch",
            Self::Stagger => "stagger",
            Self::Knockdown => "knockdown",
            Self::Launched => "launched",
            Self::Splat => "splat",
            Self::Dead => "dead",
        }
    }
}

/// The rig for a guest of a class: scaled bones, and a faded costume in the class's colours.
pub fn rig(class: Class) -> Rig {
    let k = class.scale();
    let d = Measurements::default();
    let wide = if class == Class::Heavy { 1.35 } else { 1.0 };
    let measurements = Measurements {
        pelvis: d.pelvis * k,
        spine1: d.spine1 * k,
        spine2: d.spine2 * k,
        shoulder_up: d.shoulder_up * k,
        shoulder_out: d.shoulder_out * k * wide,
        neck_up: d.neck_up * k,
        head_up: d.head_up * k,
        hip_down: d.hip_down * k,
        hip_out: d.hip_out * k * wide,
        thigh: d.thigh * k,
        shin: d.shin * k,
        upper: d.upper * k,
        fore: d.fore * k,
        turn: d.turn,
    };
    let (shell, core) = match class {
        Class::Spinner => (
            [hex(0xc6a9c8), hex(0x976d99), hex(0x624560), hex(0x36283b)],
            [hex(0x493249), hex(0x322332), hex(0x221a24), hex(0x221a24)],
        ),
        Class::Toaster => (
            [hex(0xb7c8dd), hex(0x8397b5), hex(0x536580), hex(0x303b53)],
            [hex(0x354053), hex(0x262e40), hex(0x1b2231), hex(0x1b2231)],
        ),
        // Faded teal silk.
        Class::Light => (
            [hex(0xa8d8cc), hex(0x6aa89c), hex(0x3e7068), hex(0x24443f)],
            [hex(0x34504c), hex(0x243834), hex(0x182624), hex(0x182624)],
        ),
        // Tarnished gold brocade.
        Class::Medium => (
            [hex(0xe8d49a), hex(0xb89a5c), hex(0x7c6234), hex(0x4a3a1e)],
            [hex(0x50442c), hex(0x3a301e), hex(0x282014), hex(0x282014)],
        ),
        // Bottle-green velvet, nearly black.
        Class::Heavy => (
            [hex(0x9cc49a), hex(0x5c8a62), hex(0x335a3c), hex(0x1c3424)],
            [hex(0x2c3c2e), hex(0x1e2a20), hex(0x141c16), hex(0x141c16)],
        ),
    };
    let (hat, mask, hair, costume, trim) = match class {
        Class::Light | Class::Spinner => (
            Hat::None,
            Mask::Domino,
            Hair::Bouffant,
            [hex(0x72615a), hex(0x483b3c), hex(0x2c242b), hex(0x19161e)],
            [hex(0xf1e4ca), hex(0xd5c8a9), hex(0xb1a084), hex(0x88755f)],
        ),
        Class::Toaster => (
            Hat::None,
            Mask::None,
            Hair::Tower,
            [hex(0x69798e), hex(0x47566c), hex(0x2c374c), hex(0x1a2334)],
            [hex(0xe2ded5), hex(0xc0bbb1), hex(0x97938d), hex(0x6c6870)],
        ),
        Class::Medium => (
            Hat::Plume,
            Mask::Beak,
            Hair::None,
            [hex(0x64513b), hex(0x403223), hex(0x2a211c), hex(0x19151a)],
            [hex(0xf0d9a2), hex(0xceaf72), hex(0xba985e), hex(0x987442)],
        ),
        Class::Heavy => (
            Hat::TopHat,
            Mask::None,
            Hair::Long,
            [hex(0x536150), hex(0x343e33), hex(0x222b24), hex(0x141b17)],
            [hex(0xdfc593), hex(0xb79a66), hex(0x907444), hex(0x695333)],
        ),
    };
    Rig {
        measurements,
        skirt: if class == Class::Spinner { 12.0 } else { 0.0 },
        tails: if class == Class::Toaster { 14.0 } else { 0.0 },
        hat,
        mask,
        hair,
        costume,
        trim,
        shell,
        core,
        ..Rig::default()
    }
}

/// Scales a pose authored for the player's size to a guest's.
pub fn scale(p: &Pose, k: f64) -> Pose {
    Pose {
        x: p.x * k,
        y: p.y * k,
        lean: p.lean * k,
        crouch: p.crouch * k,
        fx: p.fx * k,
        fy: p.fy * k,
        bx: p.bx * k,
        by: p.by * k,
        ffx: p.ffx * k,
        ffy: p.ffy * k,
        fbx: p.fbx * k,
        fby: p.fby * k,
        hx: p.hx * k,
        hy: p.hy * k,
        ..p.clone()
    }
}

/// Puts a class's weapon in a pose's hands, two-handed for the heavy mallet.
pub fn arm(p: Pose, class: Class) -> Pose {
    Pose {
        weapon: (class != Class::Spinner).then(|| class.weapon()),
        two: p.two || class == Class::Heavy,
        ..p
    }
}

/// The pose of a reaction `t` seconds in, at the player's size.
pub fn reaction_pose(r: Reaction, t: f64) -> Pose {
    match r {
        Reaction::Flinch => Pose {
            lean: -5.0,
            crouch: 3.0,
            hx: -2.0,
            fx: -2.0,
            fy: -27.0,
            bx: 5.0,
            by: -22.0,
            ffx: 5.0,
            fbx: -6.0,
            ..Pose::default()
        },
        Reaction::Stagger => {
            // Reeling: the body sways from side to side, arms hanging.
            let sway = (t * TAU * 1.6).sin();
            Pose {
                lean: -3.0 + sway * 3.0,
                crouch: 5.0,
                hx: sway * 2.0,
                fx: 3.0 + sway * 2.0,
                fy: -12.0,
                bx: 1.0 + sway * 2.0,
                by: -12.0,
                ffx: 4.0,
                fbx: -5.0,
                toe_f: sway > 0.5,
                ..Pose::default()
            }
        }
        Reaction::Knockdown => Pose {
            // Thrown to the floor: sitting, braced on the hands behind.
            crouch: 13.0,
            lean: -8.0,
            fx: -9.0,
            fy: -5.0,
            bx: -12.0,
            by: -5.0,
            ffx: 12.0,
            ffy: -4.0,
            fbx: 8.0,
            fby: -4.0,
            ..Pose::default()
        },
        Reaction::Launched => {
            // Tumbling through the air.
            Pose {
                y: -1.0,
                yaw: t * TAU * 1.5,
                lean: -6.0,
                crouch: 3.0,
                fx: -10.0,
                fy: -34.0,
                bx: 9.0,
                by: -33.0,
                ffx: 7.0,
                ffy: -12.0,
                fbx: -6.0,
                fby: -6.0,
                ..Pose::default()
            }
        }
        Reaction::Splat => Pose {
            // Spread flat against the wall behind.
            y: -1.0,
            lean: -2.0,
            fx: -8.0,
            fy: -34.0,
            bx: 8.0,
            by: -34.0,
            ffx: 6.0,
            ffy: -6.0,
            fbx: -6.0,
            fby: -6.0,
            ..Pose::default()
        },
        Reaction::Dead => Pose {
            crouch: 14.0,
            lean: 8.0,
            fx: 9.0,
            fy: -4.0,
            bx: 6.0,
            by: -4.0,
            ffx: -9.0,
            ffy: -4.0,
            fbx: -11.0,
            fby: -4.0,
            ..Pose::default()
        },
    }
}

/// The dancer's run and the raised glass, at player scale.
pub fn loop_pose(class: Class, phase: crate::swing::Phase, t: f64) -> Option<Pose> {
    use crate::swing::Phase;
    match class {
        Class::Spinner => Some(match phase {
            Phase::Windup => Pose {
                fx: 15.0,
                fy: -27.0,
                bx: -14.0,
                by: -27.0,
                ffx: 0.0,
                fbx: -4.0,
                fby: -7.0,
                toe_f: true,
                ..Pose::default()
            },
            Phase::Active => Pose {
                yaw: t * TAU * 3.0,
                fx: 16.0,
                fy: -25.0,
                bx: -16.0,
                by: -25.0,
                ffx: 1.0,
                fbx: -1.0,
                toe_f: true,
                ..Pose::default()
            },
            Phase::Recovery => reaction_pose(Reaction::Stagger, t),
        }),
        Class::Toaster => Some(match phase {
            Phase::Active => Pose {
                lean: 4.0,
                fx: 17.0,
                fy: -26.0,
                bx: -6.0,
                by: -19.0,
                sa: 0.0,
                ..Pose::default()
            },
            _ => Pose {
                fx: 7.0,
                fy: -43.0,
                bx: -7.0,
                by: -19.0,
                sa: -90.0,
                ..Pose::default()
            },
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swing::{Phase, Swing, swing_pose};

    #[test]
    fn every_guest_reaches_its_poses() {
        for class in Class::ALL {
            let rig = rig(class);
            let mut poses: Vec<Pose> = Reaction::ALL
                .iter()
                .flat_map(|r| (0..4).map(move |i| reaction_pose(*r, i as f64 / 12.0)))
                .collect();
            for phase in [Phase::Windup, Phase::Active, Phase::Recovery] {
                if let Some(p) = loop_pose(class, phase, 0.2) {
                    poses.push(p);
                }
                poses.push(swing_pose(Swing::Combo1, phase, 1.0));
                poses.push(swing_pose(Swing::Combo3, phase, 1.0));
            }
            for p in poses {
                let p = arm(scale(&p, class.scale()), class);
                let miss = rig.shortfall(&p);
                assert!(miss < 4.0, "{} misses by {miss:.1} px", class.name());
            }
        }
    }
}
