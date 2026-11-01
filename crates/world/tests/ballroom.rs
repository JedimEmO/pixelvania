//! The ballroom's routes, played headless against the real room file.
//!
//! The room is built for the Lure, so its distances only work with the movement as tuned.
//! Each test searches for a way through one route, a stage at a time:
//! from a rope or the floor, swing for a while, let go or jump off, maybe Swoon,
//! then reach for the next light or land on the next floor.
//! It does not need the best line, only proof that one exists;
//! when a change to the tuning or the room breaks a route, these fail.

use world::{Abilities, Controller, Controls, MoveTuning, Room, TILE};

const DT: f32 = 1.0 / 120.0;

fn ballroom() -> Room {
    Room::parse(include_str!("../../../assets/rooms/ballroom.room")).unwrap()
}

fn tile(x: f32, y: f32) -> (f32, f32) {
    (x * TILE as f32, y * TILE as f32)
}

fn feet(c: &Controller) -> (f32, f32) {
    let b = &c.body;
    ((b.x + b.w / 2) as f32, (b.y + b.h) as f32)
}

/// Where a stage has to end.
#[derive(Clone, Copy, Debug)]
enum Goal {
    /// On the rope of the light at this point.
    Light((f32, f32)),
    /// Standing with the feet at this height, between these x, all in pixels.
    Floor { y: f32, x0: f32, x1: f32 },
}

fn reached(c: &Controller, goal: Goal) -> bool {
    match goal {
        Goal::Light(p) => c.rope.is_some_and(|r| r.anchor == p),
        Goal::Floor { y, x0, x1 } => {
            let (fx, fy) = feet(c);
            c.grounded && c.rope.is_none() && (fy - y).abs() < 1.0 && fx >= x0 && fx <= x1
        }
    }
}

fn goal_x(goal: Goal) -> f32 {
    match goal {
        Goal::Light(p) => p.0,
        Goal::Floor { x0, x1, .. } => (x0 + x1) / 2.0,
    }
}

/// Flies towards the goal for up to two seconds: steers at it, reaches for its light
/// every tick (aimed straight at it), and optionally Swoons at the top of the arc.
fn fly(
    room: &Room,
    t: &MoveTuning,
    mut c: Controller,
    goal: Goal,
    swoon: bool,
) -> Option<Controller> {
    let mut swooned = !swoon;
    for _ in 0..240 {
        let dx = goal_x(goal) - feet(&c).0;
        let x = if dx.abs() < 4.0 { 0.0 } else { dx.signum() };
        let mut input = Controls {
            x,
            jump_held: true,
            ..Controls::default()
        };
        if let Goal::Light(p) = goal {
            let h = c.hands();
            let (ax, ay) = (p.0 - h.0, h.1 - p.1);
            let n = (ax * ax + ay * ay).sqrt().max(1.0);
            input = Controls {
                x: ax / n,
                y: ay / n,
                lure_pressed: true,
                lure_held: true,
                jump_held: true,
                ..Controls::default()
            };
        }
        if !swooned && c.vy > 0.0 {
            input.jump_pressed = true;
            swooned = true;
        }
        let ev = c.tick(room, t, input, DT);
        if let Some(p) = ev.lured
            && !matches!(goal, Goal::Light(g) if g == p)
        {
            return None;
        }
        if reached(&c, goal) {
            return Some(c);
        }
        if c.grounded && c.rope.is_none() && !matches!(goal, Goal::Floor { .. }) {
            return None;
        }
    }
    None
}

/// From a body on a rope, searches swing times, stick directions, climbing and ways of
/// letting go for one that reaches the goal.
fn from_rope(room: &Room, t: &MoveTuning, c: &Controller, goal: Goal) -> Option<Controller> {
    for hold in (0..360).step_by(3) {
        for (dir, climb) in [(1.0, 0.0), (-1.0, 0.0), (1.0, 1.0), (-1.0, 1.0)] {
            for jump_off in [false, true] {
                for swoon in [false, true] {
                    let mut s = c.clone();
                    for _ in 0..hold {
                        let input = Controls {
                            x: dir,
                            y: climb,
                            lure_held: true,
                            ..Controls::default()
                        };
                        s.tick(room, t, input, DT);
                    }
                    if s.rope.is_none() {
                        continue;
                    }
                    let exit = Controls {
                        x: dir,
                        jump_pressed: jump_off,
                        jump_held: jump_off,
                        ..Controls::default()
                    };
                    s.tick(room, t, exit, DT);
                    if let Some(done) = fly(room, t, s, goal, swoon) {
                        return Some(done);
                    }
                }
            }
        }
    }
    None
}

/// From standing, walks to each spot along the floor and jumps or reaches from there.
fn from_floor(room: &Room, t: &MoveTuning, c: &Controller, goal: Goal) -> Option<Controller> {
    for walk in (0..240).step_by(4) {
        for dir in [1.0, -1.0] {
            for jump in [false, true] {
                for swoon in [false, true] {
                    let mut s = c.clone();
                    for _ in 0..walk {
                        s.tick(
                            room,
                            t,
                            Controls {
                                x: dir,
                                ..Controls::default()
                            },
                            DT,
                        );
                    }
                    if jump {
                        let input = Controls {
                            x: dir,
                            jump_pressed: true,
                            jump_held: true,
                            ..Controls::default()
                        };
                        s.tick(room, t, input, DT);
                    }
                    if let Some(done) = fly(room, t, s, goal, swoon && jump) {
                        return Some(done);
                    }
                }
            }
        }
    }
    None
}

fn walk_route(start: (f32, f32), stages: &[(&str, Goal)]) {
    let room = ballroom();
    let t = MoveTuning::default();
    let mut c = Controller::standing_at(start.0, start.1, 12, 34, Abilities::ALL);
    for _ in 0..12 {
        c.tick(&room, &t, Controls::default(), DT);
    }
    for (name, goal) in stages {
        let next = if c.rope.is_some() {
            // Failing that, let go, and go on from wherever the body comes to stand.
            from_rope(&room, &t, &c, *goal).or_else(|| {
                let mut s = c.clone();
                for _ in 0..240 {
                    s.tick(&room, &t, Controls::default(), DT);
                }
                s.grounded
                    .then(|| from_floor(&room, &t, &s, *goal))
                    .flatten()
            })
        } else {
            from_floor(&room, &t, &c, *goal)
        };
        c = next.unwrap_or_else(|| panic!("no way to {name} from {:?}", feet(&c)));
        // Let a landing settle before the next stage.
        if c.rope.is_none() {
            for _ in 0..6 {
                c.tick(&room, &t, Controls::default(), DT);
            }
        }
    }
}

#[test]
fn the_chandeliers_lead_from_the_dance_floor_to_the_lounge() {
    walk_route(
        tile(4.0, 37.0),
        &[
            ("the first chandelier", Goal::Light(tile(15.0, 26.0))),
            ("the second", Goal::Light(tile(27.0, 21.0))),
            ("the third", Goal::Light(tile(25.0, 14.0))),
            (
                "the gallery",
                Goal::Floor {
                    y: 13.0 * 16.0,
                    x0: 30.0 * 16.0,
                    x1: 56.0 * 16.0,
                },
            ),
            (
                "the chandelier above the gallery",
                Goal::Light(tile(20.0, 5.5)),
            ),
            (
                "the lounge",
                Goal::Floor {
                    y: 8.0 * 16.0,
                    x0: 16.0,
                    x1: 12.0 * 16.0,
                },
            ),
        ],
    );
}

#[test]
fn the_lanterns_lead_up_the_shaft_to_the_gallery() {
    walk_route(
        tile(58.0, 37.0),
        &[
            ("the lowest lantern", Goal::Light(tile(57.5, 30.0))),
            ("the second lantern", Goal::Light(tile(61.5, 22.0))),
            ("the third lantern", Goal::Light(tile(59.5, 13.0))),
            ("the top lantern", Goal::Light(tile(58.5, 5.0))),
            (
                "the gallery",
                Goal::Floor {
                    y: 13.0 * 16.0,
                    x0: 30.0 * 16.0,
                    x1: 56.0 * 16.0,
                },
            ),
        ],
    );
}

#[test]
fn the_pit_chandeliers_carry_across_the_orchestra() {
    walk_route(
        tile(40.0, 37.0),
        &[
            ("the first pit chandelier", Goal::Light(tile(47.0, 28.0))),
            ("the second", Goal::Light(tile(53.0, 28.0))),
            (
                "the far side",
                Goal::Floor {
                    y: 37.0 * 16.0,
                    x0: 57.0 * 16.0,
                    x1: 62.0 * 16.0,
                },
            ),
        ],
    );
}
