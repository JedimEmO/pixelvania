//! The Lure: a grapple that only takes hold of light.
//!
//! Each test plays a scripted input against a small room at the game's tick rate,
//! like the movement tests, with chandeliers placed as the room's entities.

use world::{Abilities, Controller, Controls, MoveTuning, Room, TILE};

const DT: f32 = 1.0 / 120.0;
const W: i32 = 12;
const H: i32 = 34;

/// A room 40 tiles wide and 16 tall with a floor at row 14, an optional pit,
/// and the given entity lines.
fn room(pit: Option<(usize, usize)>, entities: &[&str]) -> Room {
    let mut src = String::from("[legend]\n# stone\n[tiles]\n");
    src.push_str(&"#".repeat(40));
    src.push('\n');
    for _ in 1..14 {
        src.push_str(&format!("#{}#\n", ".".repeat(38)));
    }
    for _ in 14..16 {
        let row: String = (0..40)
            .map(|x| match pit {
                Some((a, b)) if x >= a && x < b => '.',
                _ => '#',
            })
            .collect();
        src.push_str(&row);
        src.push('\n');
    }
    src.push_str("[entities]\n");
    for e in entities {
        src.push_str(e);
        src.push('\n');
    }
    Room::parse(&src).unwrap()
}

const FLOOR: f32 = (14 * TILE) as f32;

struct Sim {
    room: Room,
    t: MoveTuning,
    c: Controller,
}

impl Sim {
    fn new(room: Room, feet_x: f32) -> Self {
        let mut c = Controller::standing_at(feet_x, FLOOR, W, H, Abilities::ALL);
        let t = MoveTuning::default();
        for _ in 0..12 {
            c.tick(&room, &t, Controls::default(), DT);
        }
        Self { room, t, c }
    }

    fn tick(&mut self, input: Controls) -> world::Events {
        self.c.tick(&self.room, &self.t, input, DT)
    }

    fn rope_gap(&self) -> f32 {
        let rope = self.c.rope.expect("on the rope");
        let h = self.c.hands();
        ((h.0 - rope.anchor.0).powi(2) + (h.1 - rope.anchor.1).powi(2)).sqrt() - rope.length
    }
}

fn press_lure(x: f32, y: f32) -> Controls {
    Controls {
        x,
        y,
        lure_pressed: true,
        lure_held: true,
        ..Controls::default()
    }
}

fn hold_lure(x: f32) -> Controls {
    Controls {
        x,
        lure_held: true,
        ..Controls::default()
    }
}

#[test]
fn the_lure_takes_hold_of_a_light_up_and_ahead() {
    // A chandelier five tiles up and four ahead of the player's hands.
    let mut sim = Sim::new(room(None, &["chandelier 14 5"]), 160.0);
    let ev = sim.tick(press_lure(0.0, 0.0));
    assert_eq!(ev.lured, Some((14.0 * 16.0, 5.0 * 16.0)));
    for _ in 0..60 {
        sim.tick(hold_lure(0.0));
    }
    let rope = sim.c.rope.expect("still on the rope");
    assert_eq!(rope.length, sim.t.lure_swing_length, "the rope reels in");
    assert!(
        sim.rope_gap().abs() < 2.0,
        "the body hangs at the rope's end: gap {} hands {:?} v {} {} g {}",
        sim.rope_gap(),
        sim.c.hands(),
        sim.c.vx,
        sim.c.vy,
        sim.c.grounded
    );
}

#[test]
fn the_lure_ignores_lights_out_of_reach_behind_walls_or_below() {
    let far = room(None, &["chandelier 30 1"]);
    let mut sim = Sim::new(far, 100.0);
    assert_eq!(sim.tick(press_lure(1.0, 1.0)).lured, None, "out of reach");

    let mut walled = room(None, &["chandelier 14 5"]);
    for ty in 2..14 {
        walled.tiles[ty * walled.width + 12] = walled.tiles[0];
    }
    let mut sim = Sim::new(walled, 160.0);
    assert_eq!(sim.tick(press_lure(1.0, 1.0)).lured, None, "behind a wall");

    let mut sim = Sim::new(room(None, &["lantern 14 13.5"]), 160.0);
    assert_eq!(
        sim.tick(press_lure(1.0, 0.0)).lured,
        None,
        "below the hands"
    );
}

#[test]
fn the_stick_picks_between_lights() {
    let lights = ["chandelier 6 6", "chandelier 18 6"];
    let feet = 12.0 * 16.0;
    let mut sim = Sim::new(room(None, &lights), feet);
    assert_eq!(sim.tick(press_lure(-0.7, 0.7)).lured, Some((96.0, 96.0)));
    let mut sim = Sim::new(room(None, &lights), feet);
    assert_eq!(sim.tick(press_lure(0.7, 0.7)).lured, Some((288.0, 96.0)));
    // At rest the aim is up and forward, and the player starts facing right.
    let mut sim = Sim::new(room(None, &lights), feet);
    assert_eq!(sim.tick(press_lure(0.0, 0.0)).lured, Some((288.0, 96.0)));
}

#[test]
fn the_rope_holds_its_length_through_a_swing() {
    let mut sim = Sim::new(room(None, &["chandelier 20 4"]), 14.0 * 16.0);
    // Jump, then take hold behind and above, so the swing starts with the rope taut.
    sim.tick(Controls {
        jump_pressed: true,
        jump_held: true,
        ..Controls::default()
    });
    for _ in 0..10 {
        sim.tick(Controls {
            jump_held: true,
            ..Controls::default()
        });
    }
    assert!(sim.tick(press_lure(1.0, 1.0)).lured.is_some());
    let mut worst: f32 = 0.0;
    let mut xs = Vec::new();
    for i in 0..360 {
        sim.tick(hold_lure(0.0));
        if i > 60 && sim.c.rope.is_some() {
            worst = worst.max(sim.rope_gap());
            xs.push(sim.c.hands().0);
        }
    }
    assert!(worst < 2.5, "the rope stretched by {worst} px");
    let (lo, hi) = xs
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), &x| (lo.min(x), hi.max(x)));
    assert!(
        lo < 320.0 && hi > 320.0,
        "the body swings through the bottom"
    );
}

/// Swings across a pit too wide to jump and lets go on the way up.
#[test]
fn a_swing_carries_further_than_a_running_jump() {
    // The pit runs from tile 12 to tile 30: 18 tiles, wider than any jump.
    let pit = Some((12, 30));
    let mut sim = Sim::new(
        room(pit, &["chandelier 18 3", "chandelier 26 3"]),
        9.0 * 16.0,
    );
    let run = |sim: &mut Sim, n: u32, c: Controls| {
        for _ in 0..n {
            sim.tick(c);
        }
    };
    run(
        &mut sim,
        30,
        Controls {
            x: 1.0,
            ..Controls::default()
        },
    );
    sim.tick(Controls {
        x: 1.0,
        jump_pressed: true,
        jump_held: true,
        ..Controls::default()
    });
    run(
        &mut sim,
        12,
        Controls {
            x: 1.0,
            jump_held: true,
            ..Controls::default()
        },
    );
    assert_eq!(sim.tick(press_lure(1.0, 1.0)).lured, Some((288.0, 48.0)));
    // Ride the first chandelier until the body rises past it on the far side, then let go.
    let mut released = false;
    for _ in 0..240 {
        sim.tick(hold_lure(1.0));
        if sim.c.hands().0 > 288.0 + 40.0 && sim.c.vx > 0.0 && sim.c.vy < 0.0 {
            let ev = sim.tick(Controls {
                x: 1.0,
                ..Controls::default()
            });
            assert!(ev.released);
            released = true;
            break;
        }
    }
    assert!(released, "the swing never carried past the chandelier");
    assert!(
        sim.c.vx > sim.t.run_speed,
        "letting go flings: vx {}",
        sim.c.vx
    );
    // Catch the second chandelier in flight, swing, and let go towards the far side.
    let mut caught = false;
    for _ in 0..120 {
        if sim.tick(press_lure(1.0, 1.0)).lured == Some((416.0, 48.0)) {
            caught = true;
            break;
        }
    }
    assert!(
        caught,
        "the second chandelier was out of reach after the release"
    );
    for _ in 0..240 {
        sim.tick(hold_lure(1.0));
        if sim.c.hands().0 > 416.0 + 40.0 && sim.c.vx > 0.0 && sim.c.vy < 0.0 {
            break;
        }
    }
    for _ in 0..240 {
        sim.tick(Controls {
            x: 1.0,
            ..Controls::default()
        });
        if sim.c.grounded {
            break;
        }
    }
    let feet = sim.c.body.x + sim.c.body.w / 2;
    assert!(sim.c.grounded, "never landed");
    assert!(feet >= 30 * 16, "fell into the pit at x {feet}");
}

#[test]
fn jumping_off_the_rope_lets_go_and_rises() {
    let mut sim = Sim::new(room(None, &["chandelier 14 5"]), 160.0);
    sim.tick(press_lure(0.0, 0.0));
    for _ in 0..60 {
        sim.tick(hold_lure(0.0));
    }
    let ev = sim.tick(Controls {
        jump_pressed: true,
        jump_held: true,
        lure_held: true,
        ..Controls::default()
    });
    assert!(ev.jumped && ev.released);
    assert!(sim.c.rope.is_none());
    // Gravity has already had one tick at the jump.
    assert!(sim.c.vy <= -0.95 * sim.t.lure_jump_speed);
}

#[test]
fn taking_hold_gives_back_the_swoon() {
    let mut sim = Sim::new(room(None, &["chandelier 14 1.5"]), 160.0);
    let jump = Controls {
        jump_pressed: true,
        jump_held: true,
        ..Controls::default()
    };
    sim.tick(jump);
    for _ in 0..30 {
        sim.tick(Controls {
            jump_held: true,
            ..Controls::default()
        });
    }
    assert!(sim.tick(jump).swooned, "the Swoon is spent in the air");
    for _ in 0..4 {
        sim.tick(Controls::default());
    }
    assert!(sim.tick(press_lure(1.0, 1.0)).lured.is_some());
    sim.tick(hold_lure(0.0));
    sim.tick(Controls::default());
    assert!(sim.tick(jump).swooned, "the Lure gives the Swoon back");
}
