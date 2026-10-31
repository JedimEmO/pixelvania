//! The movement numbers level design relies on.
//!
//! Each test plays a scripted input against a small room at the game's tick rate.
//! When a tuning change breaks one of these, either the change is wrong
//! or the level metrics in `docs/design/game-feel.md` need to change with it.

use world::{Abilities, Controller, Controls, MoveTuning, Room, TILE};

const DT: f32 = 1.0 / 120.0;
const W: i32 = 12;
const H: i32 = 34;

/// Builds a room from rows of tiles, `#` solid.
fn room(rows: &[&str]) -> Room {
    let mut src = String::from("[legend]\n# stone\n[tiles]\n");
    for r in rows {
        src.push_str(r);
        src.push('\n');
    }
    Room::parse(&src).unwrap()
}

/// A wide, open room with a floor whose top is at row 10.
fn flat() -> Room {
    let open = format!("#{}#", ".".repeat(60));
    let mut rows = vec![open.as_str(); 10];
    let floor = "#".repeat(62);
    rows.push(&floor);
    room(&rows)
}

/// Feet y of the floor in rooms whose floor top is at row `row`.
fn floor_y(row: i32) -> f32 {
    (row * TILE) as f32
}

struct Sim {
    room: Room,
    tuning: MoveTuning,
    c: Controller,
    ticks: u32,
}

impl Sim {
    fn new(room: Room, feet_x: f32, feet_y: f32) -> Self {
        let c = Controller::standing_at(feet_x, feet_y, W, H, Abilities::ALL);
        Self {
            room,
            tuning: MoveTuning::default(),
            c,
            ticks: 0,
        }
    }

    fn tick(&mut self, input: Controls) -> world::Events {
        self.ticks += 1;
        self.c.tick(&self.room, &self.tuning, input, DT)
    }

    fn feet(&self) -> (f32, f32) {
        let b = &self.c.body;
        ((b.x + b.w / 2) as f32, (b.y + b.h) as f32)
    }

    fn run_for(&mut self, secs: f32, input: Controls) {
        for _ in 0..(secs / DT).round() as u32 {
            self.tick(input);
        }
    }
}

fn hold(x: f32) -> Controls {
    Controls {
        x,
        ..Controls::default()
    }
}

fn press_jump(x: f32) -> Controls {
    Controls {
        x,
        jump_held: true,
        jump_pressed: true,
        ..Controls::default()
    }
}

fn hold_jump(x: f32) -> Controls {
    Controls {
        x,
        jump_held: true,
        ..Controls::default()
    }
}

/// Jumps from standing and returns the highest the feet rose, in pixels.
fn jump_height(sim: &mut Sim, held_ticks: u32) -> f32 {
    let start = sim.feet().1;
    let mut best: f32 = 0.0;
    sim.tick(press_jump(0.0));
    for i in 0..240 {
        let input = if i < held_ticks {
            hold_jump(0.0)
        } else {
            Controls::default()
        };
        sim.tick(input);
        best = best.max(start - sim.feet().1);
    }
    best
}

#[test]
fn a_full_jump_rises_about_five_tiles() {
    let mut sim = Sim::new(flat(), 100.0, floor_y(10));
    sim.run_for(0.1, Controls::default());
    let h = jump_height(&mut sim, u32::MAX);
    println!("full jump height: {h} px = {:.2} tiles", h / TILE as f32);
    assert!(h >= 4.5 * TILE as f32, "full jump rose only {h} px");
    assert!(
        h < 5.5 * TILE as f32,
        "full jump rose {h} px, past five and a half tiles"
    );
}

#[test]
fn a_tapped_jump_is_a_short_hop() {
    let mut sim = Sim::new(flat(), 100.0, floor_y(10));
    sim.run_for(0.1, Controls::default());
    let h = jump_height(&mut sim, 1);
    println!("short hop height: {h} px");
    assert!(h <= 1.25 * TILE as f32, "a tap rose {h} px");
    assert!(h >= 0.5 * TILE as f32, "a tap rose only {h} px");
}

#[test]
fn a_full_jump_lands_on_a_three_tile_ledge() {
    // Floor at row 10, a ledge three tiles high starting at column 12.
    let mut rows: Vec<String> = vec![format!("#{}#", ".".repeat(30)); 7];
    for _ in 0..3 {
        rows.push(format!("#{}{}#", ".".repeat(11), "#".repeat(19)));
    }
    rows.push("#".repeat(32));
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let mut sim = Sim::new(room(&rows), 8.0 * TILE as f32, floor_y(10));
    sim.run_for(0.1, Controls::default());
    sim.tick(press_jump(1.0));
    sim.run_for(1.5, hold_jump(1.0));
    assert!(sim.c.grounded);
    assert_eq!(sim.feet().1, floor_y(7), "did not get on top of the ledge");
}

/// Runs at full speed off a platform edge, jumps on the last tick of coyote time
/// or the first tick off the ground, and returns whether the far platform was reached.
fn running_jump_crosses(gap_tiles: usize, lunge: bool) -> bool {
    let left = 20;
    let right = 30;
    let open = format!("#{}#", ".".repeat(left + gap_tiles + right));
    let floor = format!(
        "#{}{}{}#",
        "#".repeat(left),
        ".".repeat(gap_tiles),
        "#".repeat(right)
    );
    let mut rows = vec![open.as_str(); 10];
    rows.push(&floor);
    for _ in 0..6 {
        rows.push(&open);
    }
    rows.push(&floor);
    let mut sim = Sim::new(room(&rows), 2.0 * TILE as f32, floor_y(10));
    sim.run_for(0.1, Controls::default());
    // Run until the feet leave the ground.
    while sim.c.grounded || sim.ticks < 20 {
        sim.tick(hold(1.0));
        assert!(sim.ticks < 2000);
    }
    sim.tick(press_jump(1.0));
    let mut lunged = !lunge;
    for _ in 0..240 {
        // Lunge at the top of the jump.
        if !lunged && sim.c.vy >= 0.0 {
            sim.tick(Controls {
                lunge_pressed: true,
                ..hold_jump(1.0)
            });
            lunged = true;
        } else {
            sim.tick(hold_jump(1.0));
        }
        if sim.c.grounded {
            break;
        }
    }
    let far_edge = ((1 + left + gap_tiles) as i32 * TILE) as f32;
    sim.c.grounded && sim.feet().1 == floor_y(10) && sim.feet().0 > far_edge
}

/// The widest gap crossed, in whole tiles.
fn widest_gap(lunge: bool) -> usize {
    (1..20)
        .take_while(|&g| running_jump_crosses(g, lunge))
        .last()
        .unwrap_or(0)
}

#[test]
fn a_running_jump_clears_six_tiles_and_not_eight() {
    let gap = widest_gap(false);
    println!("widest gap, running jump: {gap} tiles");
    assert!((6..8).contains(&gap), "a running jump clears {gap} tiles");
}

#[test]
fn a_running_jump_and_lunge_clears_nine_tiles_and_not_twelve() {
    let gap = widest_gap(true);
    println!("widest gap, running jump and Lunge: {gap} tiles");
    assert!((9..12).contains(&gap), "jump and Lunge clears {gap} tiles");
}

#[test]
fn a_ground_lunge_covers_three_to_four_tiles() {
    let mut sim = Sim::new(flat(), 100.0, floor_y(10));
    sim.run_for(0.1, Controls::default());
    let start = sim.feet().0;
    sim.tick(Controls {
        lunge_pressed: true,
        ..Controls::default()
    });
    sim.run_for(0.5, Controls::default());
    let d = sim.feet().0 - start;
    println!("ground Lunge: {d} px = {:.2} tiles", d / TILE as f32);
    assert!((48.0..=64.0).contains(&d), "a Lunge covered {d} px");
}

#[test]
fn jumping_just_after_leaving_a_ledge_still_works() {
    for (late, expect_jump) in [(0.06, true), (0.15, false)] {
        let rows = [
            "#..............#",
            "#..............#",
            "#..............#",
            "#..............#",
            "#..............#",
            "#####..........#",
            "#..............#",
            "#..............#",
            "################",
        ];
        let mut sim = Sim::new(room(&rows), 3.0 * TILE as f32, floor_y(5));
        sim.run_for(0.1, Controls::default());
        while sim.c.grounded {
            sim.tick(hold(1.0));
        }
        sim.run_for(late, hold(1.0));
        let ev = sim.tick(press_jump(1.0));
        assert_eq!(ev.jumped, expect_jump, "jump {late} s after the ledge");
    }
}

#[test]
fn a_jump_pressed_just_before_landing_fires_on_landing() {
    // With the Swoon ready: a press that lands in time is a jump from the ground, not a Swoon.
    for (early, expect_jump) in [(0.1, true), (0.2, false)] {
        let mut sim = Sim::new(flat(), 100.0, floor_y(10) - 80.0);
        // Fall until `early` seconds before landing: find the landing tick first.
        let mut probe = Sim::new(flat(), 100.0, floor_y(10) - 80.0);
        let mut land_tick = 0;
        while !probe.c.grounded {
            probe.tick(hold_jump(0.0));
            land_tick += 1;
        }
        let press_at = land_tick - (early / DT).round() as u32;
        let (mut jumped, mut swooned) = (false, false);
        for i in 0..land_tick + 4 {
            let input = if i == press_at {
                press_jump(0.0)
            } else {
                hold_jump(0.0)
            };
            let ev = sim.tick(input);
            jumped |= ev.jumped;
            swooned |= ev.swooned;
        }
        assert_eq!(jumped, expect_jump, "jump pressed {early} s early");
        assert_eq!(swooned, !expect_jump, "Swoon on a press {early} s early");
    }
}

#[test]
fn a_press_in_the_air_with_no_ground_coming_still_swoons() {
    // Walk off a ledge over a deep drop and press jump straight away: the Swoon fires.
    let mut sim = Sim::new(flat(), 100.0, floor_y(10) - 120.0);
    sim.tick(Controls::default());
    let ev = sim.tick(press_jump(0.0));
    assert!(ev.swooned);
}

#[test]
fn a_jump_that_clips_a_ceiling_corner_slides_round_it() {
    // A ceiling block above the left part of the room; the gap starts at column 5.
    let rows = [
        "#..........#",
        "#..........#",
        "#..........#",
        "#..........#",
        "#####......#",
        "#..........#",
        "#..........#",
        "#..........#",
        "############",
    ];
    let edge = 5.0 * TILE as f32;
    // Body overlaps the block by 3 px.
    let mut sim = Sim::new(room(&rows), edge + W as f32 / 2.0 - 3.0, floor_y(8));
    sim.run_for(0.1, Controls::default());
    sim.tick(press_jump(0.0));
    let mut top = sim.c.body.y;
    for _ in 0..60 {
        sim.tick(hold_jump(0.0));
        top = top.min(sim.c.body.y);
    }
    assert!(
        top < 5 * TILE,
        "the jump bonked: the body rose only to y {top}"
    );
    assert_eq!(sim.c.body.x, 5 * TILE);
}

#[test]
fn a_lunge_that_clips_a_ledge_top_pops_onto_it() {
    let rows = [
        "#..............#",
        "#..............#",
        "#..............#",
        "#.......########",
        "#..............#",
        "#..............#",
        "################",
    ];
    // In the air, with the feet 2 px below the ledge top, facing it.
    let mut sim = Sim::new(room(&rows), 6.0 * TILE as f32, floor_y(3) + 2.0);
    sim.c.facing = 1;
    sim.tick(Controls {
        lunge_pressed: true,
        x: 1.0,
        ..Controls::default()
    });
    sim.run_for(0.3, hold(1.0));
    assert_eq!(
        sim.feet().1,
        floor_y(3),
        "the Lunge did not pop onto the ledge"
    );
    assert!(sim.c.body.x > 8 * TILE);
}

#[test]
fn holding_down_falls_faster() {
    let fall_time = |down: bool| {
        let mut sim = Sim::new(flat(), 100.0, floor_y(10) - 120.0);
        let input = Controls {
            y: if down { -1.0 } else { 0.0 },
            ..Controls::default()
        };
        while !sim.c.grounded {
            sim.tick(input);
        }
        sim.ticks
    };
    assert!(fall_time(true) < fall_time(false));
}

#[test]
fn a_jump_and_swoon_at_the_apex_rise_seven_to_nine_tiles() {
    // The lab's Swoon wall is seven tiles high; a jump alone must not clear it.
    let open = format!("#{}#", ".".repeat(20));
    let floor = "#".repeat(22);
    let mut rows = vec![open.as_str(); 16];
    rows.push(&floor);
    let mut sim = Sim::new(room(&rows), 100.0, floor_y(16));
    sim.run_for(0.1, Controls::default());
    let start = sim.feet().1;
    sim.tick(press_jump(0.0));
    while sim.c.vy < 0.0 {
        sim.tick(hold_jump(0.0));
    }
    assert!(sim.tick(press_jump(0.0)).swooned);
    let mut best: f32 = 0.0;
    for _ in 0..240 {
        sim.tick(hold_jump(0.0));
        best = best.max(start - sim.feet().1);
    }
    println!(
        "jump and Swoon: {best} px = {:.2} tiles",
        best / TILE as f32
    );
    assert!(
        (7.0 * TILE as f32..9.0 * TILE as f32).contains(&best),
        "a jump and Swoon rose {best} px"
    );
}

#[test]
fn the_swoon_jumps_once_in_the_air() {
    let mut sim = Sim::new(flat(), 100.0, floor_y(10));
    sim.run_for(0.1, Controls::default());
    sim.tick(press_jump(0.0));
    sim.run_for(0.2, hold_jump(0.0));
    assert!(sim.tick(press_jump(0.0)).swooned);
    sim.run_for(0.1, hold_jump(0.0));
    let ev = sim.tick(press_jump(0.0));
    assert!(!ev.swooned && !ev.jumped);
}

#[test]
fn wall_jumps_climb_a_shaft() {
    // A shaft three tiles wide and twenty tall.
    let mut rows = vec!["#####...#####"; 20];
    rows.push("#############");
    let mut sim = Sim::new(room(&rows), 6.5 * TILE as f32, floor_y(20));
    sim.run_for(0.1, Controls::default());
    sim.tick(press_jump(1.0));
    let start = sim.c.body.y;
    // Hold towards the wall being approached, and jump whenever sliding.
    let mut toward = 1.0;
    for _ in 0..600 {
        let input = if sim.c.wall != 0 {
            toward = -toward;
            press_jump(-(sim.c.wall as f32))
        } else {
            hold_jump(toward)
        };
        sim.tick(input);
    }
    let climbed = start - sim.c.body.y;
    println!("climbed {climbed} px in 5 s");
    assert!(climbed > 8 * TILE, "wall jumps climbed only {climbed} px");
}
