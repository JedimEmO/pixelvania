//! Reachability: where a player can get to, with a set of abilities, using the real movement.
//!
//! [`explore`] searches one room from where the player enters it. The places it finds are
//! nodes: standing on a tile, clinging to a wall, or hanging from a light. From each node it
//! plays a fixed set of short input scripts through the real [`Controller`] (walk, jump, Swoon,
//! Lunge, wall jump, take hold of a light, swing and let go) and adds every node they end at,
//! until nothing new turns up. Along the way it notes every door, pickup and lounge touched.
//!
//! [`check_world`] runs that room by room, through the doors, from the world's start,
//! once for each step of the progression, and reports what is reachable and where a player
//! who walks in cannot walk out again.
//!
//! The scripts are a sample of what a player can do, not all of it, so the search can miss
//! a route a skilled player finds; it does not invent routes the movement does not allow.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use crate::controller::{Abilities, Controller, Controls, MoveTuning};
use crate::doors::{Door, World};
use crate::progress::Ability;
use crate::room::{Room, TILE};

const DT: f32 = 1.0 / 120.0;
/// The player's body, as in the game.
const W: i32 = 12;
const H: i32 = 34;

/// A place a player can be at rest, or nearly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Node {
    /// Standing, feet on this tile column and row.
    Stand(i32, i32),
    /// Sliding down a wall on this side, at this tile column and row of the body's centre.
    Wall(i32, i32, i32),
    /// Hanging from the light at this index in the room's Lure points.
    Rope(usize),
}

/// What [`explore`] found in a room.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Reach {
    /// Doors touched, by id.
    pub doors: BTreeSet<String>,
    /// Pickups touched, by item name.
    pub pickups: BTreeSet<String>,
    /// Lounges stood at, by id.
    pub lounges: BTreeSet<String>,
    /// How many places the search visited.
    pub places: usize,
}

struct Explorer<'a> {
    room: &'a Room,
    t: &'a MoveTuning,
    doors: Vec<Door>,
    anchors: Vec<(f32, f32)>,
    pickups: Vec<(String, (f32, f32))>,
    lounges: Vec<(String, (f32, f32))>,
    found: Reach,
}

/// How a scripted move ended.
enum End {
    At(Node, Box<Controller>),
    /// Left through a door, or got nowhere new.
    Gone,
}

impl Explorer<'_> {
    fn feet(c: &Controller) -> (f32, f32) {
        let b = &c.body;
        ((b.x + b.w / 2) as f32, (b.y + b.h) as f32)
    }

    /// Notes what the body touches this tick. Returns true if it went through a door.
    fn touch(&mut self, c: &Controller) -> bool {
        let b = &c.body;
        let (fx, fy) = Self::feet(c);
        for (name, (x, y)) in &self.pickups {
            let r = 8.0;
            if (b.x as f32) < x + r
                && x - r < (b.x + b.w) as f32
                && (b.y as f32) < y + r
                && y - r < (b.y + b.h) as f32
            {
                self.found.pickups.insert(name.clone());
            }
        }
        if c.grounded {
            for (id, (x, y)) in &self.lounges {
                if (fx - x).abs() < 28.0 && (fy - y).abs() < 4.0 {
                    self.found.lounges.insert(id.clone());
                }
            }
        }
        for d in &self.doors {
            if d.touches(b.x as f32, b.y as f32, b.w as f32, b.h as f32) {
                self.found.doors.insert(d.id.clone());
                return true;
            }
        }
        false
    }

    fn tick(&mut self, c: &mut Controller, input: Controls) -> bool {
        c.tick(self.room, self.t, input, DT);
        self.touch(c)
    }

    /// Lets a landing settle, and names the node it ends at.
    fn settle(&mut self, mut c: Controller) -> End {
        for _ in 0..8 {
            if self.tick(&mut c, Controls::default()) {
                return End::Gone;
            }
        }
        if !c.grounded {
            return End::Gone;
        }
        let (fx, fy) = Self::feet(&c);
        let node = Node::Stand(
            (fx.floor() as i32).div_euclid(TILE),
            (fy.round() as i32 - 1).div_euclid(TILE),
        );
        End::At(node, Box::new(c))
    }

    /// Flies until the body stands, clings or hangs again.
    /// `swoon` and `lunge` are spent at the top of the arc; `lure` reaches for that light
    /// on every tick, and `wall_jump` leaps off the first wall the body slides down.
    fn fly(
        &mut self,
        mut c: Controller,
        dir: f32,
        swoon: bool,
        lunge: bool,
        lure: Option<usize>,
    ) -> End {
        let (mut swooned, mut lunged) = (!swoon, !lunge);
        let mut airborne = !c.grounded;
        for _ in 0..360 {
            let mut input = Controls {
                x: dir,
                jump_held: true,
                lure_held: lure.is_some(),
                ..Controls::default()
            };
            if c.vy > -60.0 && airborne {
                if !swooned {
                    input.jump_pressed = true;
                    swooned = true;
                } else if !lunged {
                    input.lunge_pressed = true;
                    lunged = true;
                }
            }
            if let Some(i) = lure {
                let h = c.hands();
                let (ax, ay) = (self.anchors[i].0 - h.0, h.1 - self.anchors[i].1);
                let n = (ax * ax + ay * ay).sqrt().max(1.0);
                input.x = ax / n;
                input.y = ay / n;
                input.lure_pressed = true;
            }
            if self.tick(&mut c, input) {
                return End::Gone;
            }
            airborne |= !c.grounded;
            if let Some(rope) = c.rope {
                let i = self.anchors.iter().position(|a| *a == rope.anchor);
                return match (i, lure) {
                    (Some(i), Some(want)) if i == want => End::At(Node::Rope(i), Box::new(c)),
                    _ => End::Gone,
                };
            }
            if c.wall != 0 {
                let (fx, fy) = Self::feet(&c);
                let node = Node::Wall(
                    (fx.floor() as i32).div_euclid(TILE),
                    ((fy - H as f32 / 2.0).floor() as i32).div_euclid(TILE),
                    c.wall,
                );
                return End::At(node, Box::new(c));
            }
            if airborne && c.grounded {
                return self.settle(c);
            }
        }
        End::Gone
    }

    /// The lights in reach of the hands, by index.
    fn anchors_near(&self, c: &Controller) -> Vec<usize> {
        let h = c.hands();
        (0..self.anchors.len())
            .filter(|&i| {
                let a = self.anchors[i];
                let d = ((a.0 - h.0).powi(2) + (a.1 - h.1).powi(2)).sqrt();
                d < self.t.lure_range + 120.0 && a.1 < h.1 + 40.0
            })
            .collect()
    }

    /// Every scripted move from a node.
    fn moves(&mut self, node: Node, c: &Controller) -> Vec<End> {
        let ab = c.abilities;
        let mut out = Vec::new();
        let near = if ab.lure {
            self.anchors_near(c)
        } else {
            Vec::new()
        };
        let lures: Vec<Option<usize>> = std::iter::once(None)
            .chain(near.iter().map(|&i| Some(i)))
            .collect();
        match node {
            Node::Stand(..) => {
                for dir in [-1.0, 1.0] {
                    for walk in [0, 6, 14, 30, 60, 120] {
                        let mut s = c.clone();
                        let mut gone = false;
                        let mut fell = false;
                        for _ in 0..walk {
                            if self.tick(
                                &mut s,
                                Controls {
                                    x: dir,
                                    ..Controls::default()
                                },
                            ) {
                                gone = true;
                                break;
                            }
                            if !s.grounded {
                                fell = true;
                                break;
                            }
                        }
                        if gone {
                            continue;
                        }
                        if fell {
                            out.push(self.fly(s.clone(), dir, false, false, None));
                            out.push(self.fly(s, -dir, false, false, None));
                            continue;
                        }
                        for &lure in &lures {
                            for (swoon, lunge) in
                                [(false, false), (true, false), (false, true), (true, true)]
                            {
                                if (swoon && !ab.swoon) || (lunge && !ab.lunge) {
                                    continue;
                                }
                                let mut j = s.clone();
                                let input = Controls {
                                    x: dir,
                                    jump_pressed: true,
                                    jump_held: true,
                                    ..Controls::default()
                                };
                                if self.tick(&mut j, input) {
                                    continue;
                                }
                                out.push(self.fly(j, dir, swoon, lunge, lure));
                            }
                            // Reaching from the ground, and a Lunge along it.
                            if lure.is_some() {
                                out.push(self.fly(s.clone(), dir, false, false, lure));
                            }
                        }
                        if ab.lunge {
                            let mut l = s.clone();
                            let input = Controls {
                                x: dir,
                                lunge_pressed: true,
                                ..Controls::default()
                            };
                            if !self.tick(&mut l, input) {
                                out.push(self.fly(l, dir, false, false, None));
                            }
                        }
                    }
                }
            }
            Node::Wall(_, _, side) => {
                let away = -side as f32;
                for after in [away, 0.0, -away] {
                    for &lure in &lures {
                        for (swoon, lunge) in [(false, false), (true, false), (false, true)] {
                            if (swoon && !ab.swoon) || (lunge && !ab.lunge) {
                                continue;
                            }
                            let mut j = c.clone();
                            let input = Controls {
                                x: side as f32,
                                jump_pressed: true,
                                jump_held: true,
                                ..Controls::default()
                            };
                            if self.tick(&mut j, input) {
                                continue;
                            }
                            // Hold away from the wall briefly, then steer.
                            let mut gone = false;
                            for _ in 0..10 {
                                let input = Controls {
                                    x: away,
                                    jump_held: true,
                                    ..Controls::default()
                                };
                                if self.tick(&mut j, input) {
                                    gone = true;
                                    break;
                                }
                            }
                            if !gone {
                                out.push(self.fly(j, after, swoon, lunge, lure));
                            }
                        }
                    }
                }
                // Let go and drop.
                let mut d = c.clone();
                let input = Controls {
                    x: away,
                    ..Controls::default()
                };
                if !self.tick(&mut d, input) {
                    out.push(self.fly(d, 0.0, false, false, None));
                }
            }
            Node::Rope(here) => {
                let others: Vec<Option<usize>> =
                    lures.iter().copied().filter(|l| *l != Some(here)).collect();
                for hold in (0..300).step_by(15) {
                    for (dir, climb) in [(1.0, 0.0), (-1.0, 0.0), (1.0, 1.0), (-1.0, 1.0)] {
                        let mut s = c.clone();
                        let mut gone = false;
                        for _ in 0..hold {
                            let input = Controls {
                                x: dir,
                                y: climb,
                                lure_held: true,
                                ..Controls::default()
                            };
                            if self.tick(&mut s, input) {
                                gone = true;
                                break;
                            }
                        }
                        if gone || s.rope.is_none() {
                            continue;
                        }
                        for jump_off in [false, true] {
                            for &lure in &others {
                                for swoon in [false, true] {
                                    if swoon && !ab.swoon {
                                        continue;
                                    }
                                    let mut r = s.clone();
                                    let exit = Controls {
                                        x: dir,
                                        jump_pressed: jump_off,
                                        jump_held: jump_off,
                                        ..Controls::default()
                                    };
                                    if self.tick(&mut r, exit) {
                                        continue;
                                    }
                                    out.push(self.fly(r, dir, swoon, false, lure));
                                }
                            }
                        }
                    }
                }
            }
        }
        out
    }
}

/// Explores a room with some abilities, from a body standing with its feet at `start`.
/// With the Plunge, cracked floors count as already broken.
pub fn explore(room: &Room, t: &MoveTuning, abilities: Abilities, start: (f32, f32)) -> Reach {
    let opened;
    let room = if abilities.plunge {
        let mut r = room.clone();
        for ty in 0..r.height as i32 {
            for tx in 0..r.width as i32 {
                if r.material(tx, ty) == Some(crate::room::CRACKED) {
                    r.clear(tx, ty);
                }
            }
        }
        opened = r;
        &opened
    } else {
        room
    };
    let named = |kind: &str, key: &str| -> Vec<(String, (f32, f32))> {
        room.entities_of(kind)
            .filter_map(|e| Some((e.props.get(key)?.clone(), e.pixel_pos())))
            .collect()
    };
    let mut ex = Explorer {
        room,
        t,
        doors: room.doors().unwrap_or_default(),
        anchors: room.lure_points(),
        pickups: named("pickup", "item"),
        lounges: named("lounge", "id"),
        found: Reach::default(),
    };
    let c = Controller::standing_at(start.0, start.1, W, H, abilities);
    let mut seen: HashMap<Node, ()> = HashMap::new();
    let mut queue = VecDeque::new();
    // Arriving over open air, through a floor or a ceiling, the body first falls.
    let mut c = c;
    if !c.body.on_ground(room) {
        c.tick(room, t, Controls::default(), DT);
    }
    let first = if c.body.on_ground(room) {
        ex.settle(c)
    } else {
        ex.fly(c, 0.0, false, false, None)
    };
    if let End::At(node, c) = first {
        seen.insert(node, ());
        queue.push_back((node, c));
    }
    while let Some((node, c)) = queue.pop_front() {
        for end in ex.moves(node, &c) {
            if let End::At(next, c) = end
                && seen.insert(next, ()).is_none()
            {
                queue.push_back((next, c));
            }
        }
    }
    ex.found.places = seen.len();
    ex.found
}

/// What [`check_world`] found with one set of abilities.
#[derive(Debug, Clone, Default)]
pub struct WorldReach {
    /// Rooms entered.
    pub rooms: BTreeSet<String>,
    /// Pickups reachable, as `room:item`.
    pub pickups: BTreeSet<String>,
    /// Lounges reachable, as `room:id`.
    pub lounges: BTreeSet<String>,
    /// Ways in from which no door out can be reached, as `room:door`.
    pub traps: BTreeSet<String>,
}

/// The ability sets [`check_world`] tries: none, then each ability added in the progression's order.
pub fn progression_steps() -> Vec<BTreeSet<Ability>> {
    let mut steps = vec![BTreeSet::new()];
    for a in Ability::ALL {
        let mut next = steps.last().unwrap().clone();
        next.insert(a);
        steps.push(next);
    }
    steps
}

/// Walks the world from its start, through doors, with these abilities.
pub fn check_world(
    world: &World,
    rooms: &BTreeMap<String, Room>,
    t: &MoveTuning,
    abilities: &BTreeSet<Ability>,
) -> WorldReach {
    let movement = crate::progress::Progress {
        abilities: abilities.clone(),
        ..Default::default()
    }
    .movement();
    let mut out = WorldReach::default();
    // A way into a room: the room and the door entered by, or none for the start.
    let mut queue: VecDeque<(String, Option<String>)> =
        VecDeque::from([(world.start.clone(), None)]);
    let mut done = BTreeSet::new();
    while let Some((name, entry)) = queue.pop_front() {
        if !done.insert((name.clone(), entry.clone())) {
            continue;
        }
        let Some(room) = rooms.get(&name) else {
            continue;
        };
        let start = match &entry {
            None => room.entities_of("spawn").next().map(|s| s.pixel_pos()),
            Some(door) => room
                .door(door)
                .and_then(|d| d.arrival(room))
                .map(|(f, _)| f),
        };
        let Some(start) = start else { continue };
        out.rooms.insert(name.clone());
        let reach = explore(room, t, movement, start);
        for p in &reach.pickups {
            out.pickups.insert(format!("{name}:{p}"));
        }
        for l in &reach.lounges {
            out.lounges.insert(format!("{name}:{l}"));
        }
        if reach.doors.is_empty()
            && let Some(door) = &entry
        {
            out.traps.insert(format!("{name}:{door}"));
        }
        for d in room.doors().unwrap_or_default() {
            if reach.doors.contains(&d.id) {
                queue.push_back((d.to_room.clone(), Some(d.to_door.clone())));
            }
        }
    }
    out
}
