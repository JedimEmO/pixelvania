//! The guests: the party's lost dancers, built on the same rig with their own proportions.
//!
//! A guest comes in one of three weight classes, and the class shows in the silhouette:
//! light guests are slight and quick with a knife, medium guests are the player's size with a
//! cane-spear, and heavy guests are broad and tall and swing a mallet with both hands.
//! Their poses are the player's, scaled to their size, plus the ways a body reacts to a hit.

use std::f64::consts::TAU;

use pixelfx::color::hex;
use pixelfx::rig::{Hair, Hat, Mask, Measurements, Pose, Rig, Weapon};

/// A guest's weight class, which sets its size, weapon and how it takes a hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Light,
    Medium,
    Heavy,
}

impl Class {
    pub const ALL: [Self; 3] = [Self::Light, Self::Medium, Self::Heavy];

    pub fn name(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Medium => "medium",
            Self::Heavy => "heavy",
        }
    }

    /// Size relative to the player.
    pub fn scale(self) -> f64 {
        match self {
            Self::Light => 0.85,
            Self::Medium => 1.0,
            Self::Heavy => 1.25,
        }
    }

    pub fn weapon(self) -> Weapon {
        match self {
            Self::Light => Weapon::Dagger,
            Self::Medium => Weapon::Spear,
            Self::Heavy => Weapon::Hammer,
        }
    }

    /// Hurtbox width and height in pixels, standing.
    pub fn size(self) -> (i32, i32) {
        match self {
            Self::Light => (10, 29),
            Self::Medium => (12, 34),
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
        Class::Light => (
            Hat::None,
            Mask::Domino,
            Hair::Bouffant,
            [hex(0x72615a), hex(0x483b3c), hex(0x2c242b), hex(0x19161e)],
            [hex(0xf1e4ca), hex(0xd5c8a9), hex(0xb1a084), hex(0x88755f)],
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
        weapon: Some(class.weapon()),
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
