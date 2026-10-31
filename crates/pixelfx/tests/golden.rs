//! Raw buffer comparisons against the pinned JavaScript implementation. Node is not used by tests.
use pixelfx::{
    Effect, EffectId, Frame, Options,
    act::timing,
    painter::Painter,
    rig::{Pose, Rig},
};
use std::path::PathBuf;
fn compare(name: &str, frame: &Frame) {
    for (layer, actual) in [("color", &frame.color), ("light", &frame.light)] {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/golden")
            .join(format!("{name}-{layer}.png"));
        let expected = image::open(&path)
            .unwrap_or_else(|e| panic!("{}: {e}; run cargo xtask golden", path.display()))
            .to_rgba8();
        assert_eq!(expected.dimensions(), (frame.width, frame.height));
        let differences = actual
            .iter()
            .zip(expected.pixels())
            .filter(|(a, b)| **a != b.0)
            .count();
        let fraction = differences as f64 / actual.len() as f64;
        println!(
            "{name} {layer}: {differences}/{} = {:.6}% pixels differ",
            actual.len(),
            fraction * 100.0
        );
        assert!(
            fraction <= 0.005,
            "{name} {layer}: {:.6}% differs (limit 0.5%)",
            fraction * 100.0
        );
    }
}
fn effect(id: EffectId, indices: &[usize]) {
    let m = id.metadata();
    let mut fx = Effect::new(id, m.seed, Options::default());
    let mut frame = Frame::new(m.size[0], m.size[1]);
    for i in 0..=*indices.last().unwrap() {
        if i > 0 {
            fx.update(1.0 / 30.0, &|_| 0.0);
        }
        if indices.contains(&i) {
            frame.clear();
            fx.draw(&mut frame, m.origin);
            compare(&format!("{}-{i}", id.name().replace('/', "-")), &frame);
        }
    }
}
#[test]
fn hit_spark() {
    effect(EffectId::HitSpark, &[0, 1, 2, 3, 6, 12]);
}
#[test]
fn heavy_landing() {
    effect(EffectId::HeavyLanding, &[0, 1, 4, 9, 18, 36]);
}
#[test]
fn sword_slash() {
    effect(EffectId::SwordSlash, &[0, 1, 2, 4, 8, 16]);
}
#[test]
fn hammer_smash() {
    effect(EffectId::HammerSmash, &[0, 3, 5, 7, 15, 36]);
}
#[test]
fn poof() {
    effect(EffectId::Poof, &[0, 1, 3, 8, 18, 36]);
}
#[test]
fn rig() {
    let rig = Rig::default();
    let poses = [
        Pose::default(),
        Pose::ready(),
        timing(EffectId::SwordSlash).unwrap().poses[3].pose.clone(),
        timing(EffectId::HammerSmash).unwrap().poses[2].pose.clone(),
        Pose {
            yaw: 1.2,
            ..Pose::ready()
        },
    ];
    for (i, pose) in poses.iter().enumerate() {
        let mut frame = Frame::new(128, 128);
        let mut p = Painter::new(&mut frame);
        p.place(64.0, 100.0, false, 0.0);
        rig.render(pose).draw(&mut p);
        compare(&format!("rig-{i}"), &frame);
    }
}
