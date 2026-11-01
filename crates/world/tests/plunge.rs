//! The Plunge: a dive straight down from the air.

use world::{Abilities, Controller, Controls, MoveTuning, Room, TILE};

const DT: f32 = 1.0 / 120.0;

fn room() -> Room {
    let mut src = String::from("[legend]\n# stone\n[tiles]\n");
    src.push_str(&"#".repeat(20));
    src.push('\n');
    for _ in 1..30 {
        src.push_str(&format!("#{}#\n", ".".repeat(18)));
    }
    src.push_str(&"#".repeat(20));
    src.push('\n');
    Room::parse(&src).unwrap()
}

fn standing(abilities: Abilities) -> (Room, MoveTuning, Controller) {
    let r = room();
    let t = MoveTuning::default();
    let mut c = Controller::standing_at(160.0, (30 * TILE) as f32, 12, 34, abilities);
    for _ in 0..6 {
        c.tick(&r, &t, Controls::default(), DT);
    }
    (r, t, c)
}

fn press(plunge: bool, jump: bool) -> Controls {
    Controls {
        plunge_pressed: plunge,
        jump_pressed: jump,
        jump_held: jump,
        ..Controls::default()
    }
}

#[test]
fn a_plunge_hangs_then_dives_and_lands_hard() {
    let (r, t, mut c) = standing(Abilities::ALL);
    c.tick(&r, &t, press(false, true), DT);
    for _ in 0..20 {
        c.tick(
            &r,
            &t,
            Controls {
                jump_held: true,
                ..Controls::default()
            },
            DT,
        );
    }
    let ev = c.tick(&r, &t, press(true, false), DT);
    assert!(ev.plunge_started);
    assert_eq!((c.vx, c.vy), (0.0, 0.0), "it hangs at its start");
    let mut landed = None;
    for i in 0..240 {
        let ev = c.tick(
            &r,
            &t,
            Controls {
                x: 1.0,
                ..Controls::default()
            },
            DT,
        );
        assert_eq!(c.vx, 0.0, "a Plunge goes straight down");
        if ev.plunged {
            landed = Some((i, ev.landed));
            break;
        }
    }
    let (ticks, speed) = landed.expect("the Plunge lands");
    assert!(
        ticks < 30,
        "a dive from a jump's height is quick: {ticks} ticks"
    );
    assert_eq!(speed, Some(t.plunge_speed));
    assert!(c.plunging.is_none());
}

#[test]
fn no_plunge_from_the_ground_or_without_the_ability() {
    let (r, t, mut c) = standing(Abilities::ALL);
    assert!(!c.tick(&r, &t, press(true, false), DT).plunge_started);
    let without = Abilities {
        plunge: false,
        ..Abilities::ALL
    };
    let (r, t, mut c) = standing(without);
    c.tick(&r, &t, press(false, true), DT);
    for _ in 0..10 {
        c.tick(&r, &t, Controls::default(), DT);
    }
    assert!(!c.tick(&r, &t, press(true, false), DT).plunge_started);
}
