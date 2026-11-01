//! The Dancer's open turns, low dash and exhausted bows.
use crate::{Pose, Rig, guest};
use pixelfx::color::hex;
use pixelfx::rig::{Hat, Mask};

#[derive(Clone, Copy)]
pub enum PoseKind {
    Idle,
    SweepWindup,
    Sweep,
    LeapWindup,
    Leap,
    DashWindup,
    Dash,
    Exhausted,
    Stagger,
    Death,
}

pub fn rig() -> Rig {
    let mut r = guest::rig(guest::Class::Medium);
    let m = &mut r.measurements;
    m.pelvis *= 1.5;
    m.spine1 *= 1.5;
    m.spine2 *= 1.5;
    m.shoulder_up *= 1.5;
    m.shoulder_out *= 1.5;
    m.neck_up *= 1.5;
    m.head_up *= 1.5;
    m.hip_down *= 1.5;
    m.hip_out *= 1.5;
    m.thigh *= 1.5;
    m.shin *= 1.5;
    m.upper *= 1.5;
    m.fore *= 1.5;
    r.hat = Hat::Feathers;
    r.ribbons = 30.0;
    r.mask = Mask::Domino;
    r.skirt = 19.0;
    r.tails = 28.0;
    r.shell = [hex(0xffafd9), hex(0xff459e), hex(0xba2267), hex(0x6c1748)];
    r.costume = r.shell;
    r.trim = [hex(0xfff5ba), hex(0xffcf54), hex(0xd79029), hex(0x92501e)];
    r
}

pub fn pose(kind: PoseKind, t: f64) -> Pose {
    use PoseKind::*;
    let p = match kind {
        Idle => Pose {
            lean: (t * 6.0).sin() * 2.0,
            fx: 12.0,
            fy: -30.0,
            bx: -11.0,
            by: -23.0,
            toe_f: true,
            ..Pose::default()
        },
        SweepWindup => Pose {
            yaw: -0.6 + (t * 1.2).min(0.6),
            crouch: 3.0 * (1.0 - (t * 2.0).min(1.0)),
            fx: 16.0,
            fy: -29.0,
            bx: -16.0,
            by: -29.0,
            ffx: 1.0,
            fbx: -5.0,
            fby: -6.0,
            toe_f: true,
            ..Pose::default()
        },
        Sweep => Pose {
            yaw: t * 18.0,
            fx: 17.0,
            fy: -25.0,
            bx: -17.0,
            by: -25.0,
            ffx: 1.0,
            fbx: -1.0,
            toe_f: true,
            ..Pose::default()
        },
        LeapWindup => Pose {
            crouch: 8.0,
            fx: 4.0,
            fy: -42.0,
            bx: -4.0,
            by: -42.0,
            ffx: 7.0,
            fbx: -7.0,
            ..Pose::default()
        },
        Leap => Pose {
            y: -1.0,
            fx: 9.0,
            fy: -39.0,
            bx: -9.0,
            by: -39.0,
            ffx: 8.0,
            ffy: -12.0,
            fbx: -8.0,
            fby: -12.0,
            ..Pose::default()
        },
        DashWindup => Pose {
            lean: -7.0,
            crouch: 9.0,
            fx: -10.0,
            fy: -20.0,
            bx: -12.0,
            by: -25.0,
            ffx: 10.0,
            fbx: -7.0,
            ..Pose::default()
        },
        Dash => Pose {
            lean: 10.0,
            crouch: 12.0,
            fx: 16.0,
            fy: -13.0,
            bx: -11.0,
            by: -16.0,
            ffx: 12.0,
            fbx: -12.0,
            ..Pose::default()
        },
        Exhausted | Stagger => guest::reaction_pose(guest::Reaction::Stagger, t),
        Death => {
            let u = (t / 0.65).clamp(0.0, 1.0);
            pixelfx::act::lerp_pose(
                &guest::reaction_pose(guest::Reaction::Stagger, 0.0),
                &Pose {
                    crouch: 24.0,
                    lean: 14.0,
                    hx: 6.0,
                    hy: 6.0,
                    fx: 18.0,
                    fy: -2.0,
                    bx: 10.0,
                    by: -2.0,
                    ffx: -12.0,
                    fbx: -15.0,
                    ..Pose::default()
                },
                u,
            )
        }
    };
    guest::scale(&p, 1.5)
}
