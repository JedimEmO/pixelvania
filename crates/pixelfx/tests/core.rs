//! Determinism and integration contracts that raw reference frames do not cover.
use pixelfx::{
    Effect, EffectId, Frame, Options,
    act::{ActClock, pose_at, timing},
    color::WHITE,
    effects::DustVariant,
    math::{Rng, hash},
    rig::{Pose, Rig},
    sim::{Bit, Bits, Kind, Particles},
};
#[test]
fn rng_matches_javascript_wrapping_and_zero_seed() {
    let cases = [
        (
            0,
            [
                2693262067, 11749833, 2265367787, 4213581821, 4159151403, 1207330352, 2632122864,
                3095568220,
            ],
        ),
        (
            4,
            [
                3966987260, 1430421160, 952056385, 351504504, 1005010505, 2183977359, 332393912,
                4182705100,
            ],
        ),
        (
            u32::MAX,
            [
                3850105811, 813802916, 3073704848, 4054706436, 3630262831, 2315588663, 2922715533,
                2042566601,
            ],
        ),
    ];
    for (seed, expected) in cases {
        let mut rng = Rng::new(seed);
        for n in expected {
            assert_eq!((rng.sample() * 4294967296.0) as u32, n);
        }
    }
    assert_eq!(hash(&[0, -1, i32::MAX]), 1590112545);
}
#[test]
fn sparks_and_debris_bounce_on_caller_ground() {
    let floor = |x: f64| if x < 0.0 { -8.0 } else { -4.0 };
    let mut rng = Rng::new(5);
    let mut ps = Particles::default();
    ps.spawn(
        &mut rng,
        Kind::Spark,
        2.0,
        -5.0,
        0.0,
        200.0,
        1.0,
        WHITE,
        1.0,
        0.0,
        false,
    );
    ps.update(1.0 / 120.0, &mut rng, &floor);
    assert_eq!(ps.list[0].y, -4.25);
    assert!(ps.list[0].vy < 0.0);
    let mut bits = Bits::default();
    bits.add(Bit {
        x: -2.0,
        y: -9.0,
        vy: 200.0,
        bounce: 0.5,
        ..Bit::default()
    });
    bits.update(1.0 / 120.0, &floor);
    assert_eq!(bits.list[0].y, -8.01);
    assert!(bits.list[0].vy < 0.0);
}
fn snapshot(frame: &Frame, mut hash: u64) -> u64 {
    for byte in frame.color.iter().chain(&frame.light).flatten() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}
#[test]
fn dust_variants_have_deterministic_snapshots() {
    let expected = [
        4905726819735890514_u64,
        5158966048020890068,
        7032071082741193231,
    ];
    for (i, variant) in [DustVariant::Run, DustVariant::Jump, DustVariant::Land]
        .into_iter()
        .enumerate()
    {
        let options = Options {
            dust: variant,
            ..Options::default()
        };
        let mut fx = Effect::new(EffectId::Dust, 42, options);
        let mut f = Frame::new(96, 56);
        let mut h = 14695981039346656037;
        for frame in 0..=24 {
            if frame > 0 {
                fx.update(1.0 / 30.0, &|_| 0.0);
            }
            f.clear();
            fx.draw(&mut f, [48.0, 40.0]);
            h = snapshot(&f, h);
        }
        println!("dust {variant:?}: {h}");
        assert_eq!(h, expected[i]);
    }
}
#[test]
fn seeking_replays_the_preview_clock_and_rewinds() {
    let mut fx = Effect::new(EffectId::Poof, 21, Options::default());
    let mut a = Frame::new(112, 92);
    for _ in 0..8 {
        fx.update(1.0 / 30.0, &|_| 0.0);
    }
    fx.draw(&mut a, [56.0, 80.0]);
    fx.seek(1.2, &|_| 0.0);
    fx.seek(8.0 / 30.0, &|_| 0.0);
    let mut b = Frame::new(112, 92);
    fx.draw(&mut b, [56.0, 80.0]);
    assert_eq!(a, b);
    fx.stop();
    fx.update(0.31, &|_| 0.0);
    assert!(fx.finished());
}
#[test]
fn palette_and_seed_change_effects_and_layers_compose() {
    for id in EffectId::ALL {
        let m = id.metadata();
        let mut fx = Effect::new(id, m.seed, Options::default());
        fx.seek(0.2, &|_| 0.0);
        let mut a = Frame::new(m.size[0], m.size[1]);
        fx.draw(&mut a, m.origin);
        let mut b = Frame::new(m.size[0], m.size[1]);
        fx.draw_layer(&mut b, m.origin, pixelfx::sim::Layer::Back);
        fx.draw_layer(&mut b, m.origin, pixelfx::sim::Layer::Front);
        assert_eq!(a, b);
        let options = Options {
            palette: vec![[255, 0, 128, 255]],
            ..Options::default()
        };
        let mut recolored = Effect::new(id, 55, options);
        recolored.seek(0.2, &|_| 0.0);
        b.clear();
        recolored.draw(&mut b, m.origin);
        assert_ne!(a, b, "{} ignores palette and seed", id.name());
    }
}
#[test]
fn act_freezes_and_emits_the_hit_once() {
    let timing = timing(EffectId::SwordSlash).unwrap();
    assert_eq!(timing.windup, 0.55);
    let mut clock = ActClock::new(timing);
    let hits = clock.update(0.62);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].freeze, 0.14);
    let t = clock.time();
    assert!(clock.frozen());
    assert!(clock.update(0.1).is_empty());
    assert_eq!(clock.time(), t);
    assert!(clock.update(0.05).is_empty());
    assert_eq!(clock.time(), t);
    assert!(!clock.frozen());
    assert!(clock.update(0.05).is_empty());
    assert!(clock.time() > t);
}
#[test]
fn poses_hold_limbs_at_twelve_fps_and_snap_at_contact() {
    let a = timing(EffectId::SwordSlash).unwrap();
    let p = pose_at(&a.poses, -0.54);
    let q = pose_at(&a.poses, -0.50);
    assert_eq!(p.fx, q.fx);
    let held = pose_at(&a.poses, 0.059);
    let contact = pose_at(&a.poses, 0.06);
    assert_eq!(held, a.poses[3].pose);
    assert_eq!(contact, a.poses[4].pose);
}
#[test]
fn rig_is_reentrant_and_limbs_have_fixed_lengths() {
    let rig = Rig::default();
    let q = Pose {
        fx: 60.0,
        fy: -30.0,
        ..Pose::ready()
    };
    assert!(rig.shortfall(&q) > 20.0);
    let joints = rig.skeleton(&q);
    let get = |name| joints.iter().find(|(n, _)| *n == name).unwrap().1;
    let distance = |a: [f64; 3], b: [f64; 3]| (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2]);
    assert!((distance(get("shoulderN"), get("elbowN")) - rig.measurements.upper).abs() < 1e-10);
    assert!((distance(get("elbowN"), get("handN")) - rig.measurements.fore).abs() < 1e-10);
    let a = rig.render(&Pose::ready());
    let _ = rig.render(&q);
    let b = rig.render(&Pose::ready());
    assert_eq!(
        a.pixels
            .iter()
            .map(|p| (p.x, p.y, p.color))
            .collect::<Vec<_>>(),
        b.pixels
            .iter()
            .map(|p| (p.x, p.y, p.color))
            .collect::<Vec<_>>()
    );
}
