//! Doors between rooms, and the world file that lists the rooms.
//!
//! A door is an entity in a room file, an opening in the room's outer wall:
//!
//! ```text
//! door 0 34 h=3 id=west to=antechamber:east
//! ```
//!
//! Its position is the top-left tile of the opening, `w` and `h` its size in tiles (1 by default),
//! and `to` the room and door it leads to. Walking into it takes the player through,
//! and they arrive just inside the door it names, facing into that room.
//!
//! `assets/world.ron` lists the rooms of the game and where play starts.
//! [`World::check`] makes sure every door leads to a room in the world,
//! to a door there that leads back.

use serde::{Deserialize, Serialize};

use crate::room::{EntityDef, ParseError, Room, TILE};

/// A door, with its opening in pixels from the room's top-left corner, y down.
#[derive(Debug, Clone, PartialEq)]
pub struct Door {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// The room and the door in it that this one leads to.
    pub to_room: String,
    pub to_door: String,
}

/// Which side of the room a door opens in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

impl Door {
    fn parse(def: &EntityDef) -> Result<Door, ParseError> {
        let err = |message: String| ParseError {
            line: def.line,
            message,
        };
        let id = def
            .props
            .get("id")
            .ok_or_else(|| err("a door needs an `id`".into()))?
            .clone();
        let to = def
            .props
            .get("to")
            .ok_or_else(|| err(format!("door `{id}` needs `to=room:door`")))?;
        let (to_room, to_door) = to
            .split_once(':')
            .ok_or_else(|| err(format!("door `{id}`: `to={to}` is not `room:door`")))?;
        let (x, y) = def.pixel_pos();
        Ok(Door {
            x,
            y,
            w: def.num("w", 1.0)? * TILE as f32,
            h: def.num("h", 1.0)? * TILE as f32,
            to_room: to_room.to_owned(),
            to_door: to_door.to_owned(),
            id,
        })
    }

    /// The side of the room this door opens in: the room edge it touches.
    pub fn side(&self, room: &Room) -> Option<Side> {
        let (w, h) = room.pixel_size();
        if self.x <= 0.0 {
            Some(Side::Left)
        } else if self.x + self.w >= w as f32 {
            Some(Side::Right)
        } else if self.y <= 0.0 {
            Some(Side::Top)
        } else if self.y + self.h >= h as f32 {
            Some(Side::Bottom)
        } else {
            None
        }
    }

    /// Whether a body's box, in pixels, touches the opening.
    pub fn touches(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        x < self.x + self.w && self.x < x + w && y < self.y + self.h && self.y < y + h
    }

    /// Where a body arriving through this door stands: its feet, in pixels with y down,
    /// a tile inside the opening, and the way it faces, into the room.
    /// A door in the floor or the ceiling puts the body just inside it, mid-opening.
    pub fn arrival(&self, room: &Room) -> Option<((f32, f32), i32)> {
        let inside = TILE as f32 * 1.5;
        let bottom = self.y + self.h;
        let mid = self.x + self.w / 2.0;
        Some(match self.side(room)? {
            Side::Left => ((self.x + self.w + inside, bottom), 1),
            Side::Right => ((self.x - inside, bottom), -1),
            Side::Top => ((mid, self.y + self.h + TILE as f32 * 2.5), 1),
            Side::Bottom => ((mid, self.y - TILE as f32 * 0.5), 1),
        })
    }
}

impl Room {
    /// The room's doors, in the order they appear in the file.
    pub fn doors(&self) -> Result<Vec<Door>, ParseError> {
        self.entities_of("door").map(Door::parse).collect()
    }

    /// The door with this id.
    pub fn door(&self, id: &str) -> Option<Door> {
        self.doors().ok()?.into_iter().find(|d| d.id == id)
    }
}

/// The rooms of the game, read from `assets/world.ron`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct World {
    /// The room a new game starts in.
    pub start: String,
    pub rooms: Vec<WorldRoom>,
}

/// One room of the world.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldRoom {
    /// The room file is `assets/rooms/<name>.room`.
    pub name: String,
    /// The area it belongs to, such as `ballroom`.
    pub area: String,
    /// Where the room's top-left tile sits on the world map, in tiles.
    /// Rooms joined by a door sit side by side, so the door lines up on both sides.
    #[serde(default)]
    pub pos: Option<(i32, i32)>,
}

impl World {
    pub fn parse(src: &str) -> Result<World, ron::error::SpannedError> {
        ron::from_str(src)
    }

    /// Checks the world's doors, given a way to load each room by name.
    /// Returns every problem found, so one run lists them all.
    pub fn check(&self, load: impl Fn(&str) -> Result<Room, String>) -> Vec<String> {
        let mut problems = Vec::new();
        let mut rooms = Vec::new();
        for r in &self.rooms {
            match load(&r.name) {
                Ok(room) => rooms.push((r.name.as_str(), room)),
                Err(e) => problems.push(format!("room `{}`: {e}", r.name)),
            }
        }
        if !self.rooms.iter().any(|r| r.name == self.start) {
            problems.push(format!(
                "the start room `{}` is not in the world",
                self.start
            ));
        }
        let find = |name: &str| rooms.iter().find(|(n, _)| *n == name).map(|(_, r)| r);
        for (name, room) in &rooms {
            let doors = match room.doors() {
                Ok(d) => d,
                Err(e) => {
                    problems.push(format!("room `{name}`: {e}"));
                    continue;
                }
            };
            for d in &doors {
                let at = format!("room `{name}`, door `{}`", d.id);
                if d.side(room).is_none() {
                    problems.push(format!("{at}: not on the room's edge"));
                }
                if doors.iter().filter(|o| o.id == d.id).count() > 1 {
                    problems.push(format!("{at}: the id is used twice"));
                }
                let Some(other) = find(&d.to_room) else {
                    problems.push(format!("{at}: leads to `{}`, not in the world", d.to_room));
                    continue;
                };
                let Some(back) = other.door(&d.to_door) else {
                    problems.push(format!(
                        "{at}: leads to door `{}` in `{}`, which does not exist",
                        d.to_door, d.to_room
                    ));
                    continue;
                };
                if back.to_room != *name || back.to_door != d.id {
                    problems.push(format!(
                        "{at}: leads to `{}:{}`, which leads to `{}:{}` instead of back",
                        d.to_room, d.to_door, back.to_room, back.to_door
                    ));
                }
                // On the map the two openings must meet: side by side, along the same tiles.
                let here = self.rooms.iter().find(|r| r.name == *name).and_then(|r| r.pos);
                let there = self.rooms.iter().find(|r| r.name == d.to_room).and_then(|r| r.pos);
                if let (Some(a), Some(b)) = (here, there)
                    && !meets(d, a, room, &back, b, other)
                {
                    problems.push(format!(
                        "{at}: on the map it does not meet `{}:{}`; check both rooms' `pos`",
                        d.to_room, d.to_door
                    ));
                }
                if let Some(((x, y), _)) = back.arrival(other) {
                    let feet =
                        crate::Body::new(x.round() as i32 - 6, y.round() as i32 - 34, 12, 34);
                    if feet.collides_at(other, 0, 0) {
                        problems.push(format!(
                            "{at}: arriving through `{}:{}` lands inside a wall",
                            d.to_room, d.to_door
                        ));
                    }
                }
            }
        }
        problems
    }
}

/// A door's opening on the world map, in tiles: left, top, right, bottom (exclusive).
fn on_map(d: &Door, pos: (i32, i32)) -> (i32, i32, i32, i32) {
    let t = TILE as f32;
    let x0 = pos.0 + (d.x / t).round() as i32;
    let y0 = pos.1 + (d.y / t).round() as i32;
    (x0, y0, x0 + (d.w / t).round() as i32, y0 + (d.h / t).round() as i32)
}

/// Whether two doors meet on the map: openings on facing sides, touching, along the same tiles.
fn meets(a: &Door, pa: (i32, i32), ra: &Room, b: &Door, pb: (i32, i32), rb: &Room) -> bool {
    let (ax0, ay0, ax1, ay1) = on_map(a, pa);
    let (bx0, by0, bx1, by1) = on_map(b, pb);
    match (a.side(ra), b.side(rb)) {
        (Some(Side::Right), Some(Side::Left)) => ax1 == bx0 && (ay0, ay1) == (by0, by1),
        (Some(Side::Left), Some(Side::Right)) => bx1 == ax0 && (ay0, ay1) == (by0, by1),
        (Some(Side::Bottom), Some(Side::Top)) => ay1 == by0 && (ax0, ax1) == (bx0, bx1),
        (Some(Side::Top), Some(Side::Bottom)) => by1 == ay0 && (ax0, ax1) == (bx0, bx1),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room(name: &str, entities: &str) -> Room {
        let src = format!(
            "[room]\nname = {name}\n[legend]\n# stone\n[tiles]\n##########\n.........#\n.........#\n.........#\n##########\n[entities]\n{entities}\n"
        );
        Room::parse(&src).unwrap()
    }

    #[test]
    fn doors_lead_back_or_are_reported() {
        let world = World {
            start: "a".into(),
            rooms: vec![
                WorldRoom {
                    name: "a".into(),
                    area: "x".into(),
                    pos: None,
                },
                WorldRoom {
                    name: "b".into(),
                    area: "x".into(),
                    pos: None,
                },
            ],
        };
        let good = |name: &str| -> Result<Room, String> {
            Ok(room(
                name,
                &format!(
                    "door 0 1 h=3 id=w to={}:w",
                    if name == "a" { "b" } else { "a" }
                ),
            ))
        };
        assert_eq!(world.check(good), Vec::<String>::new());

        // Both rooms' doors lead to `b`, so `a`'s door is answered by one that does not lead back.
        let one_way =
            |name: &str| -> Result<Room, String> { Ok(room(name, "door 0 1 h=3 id=w to=b:w")) };
        let problems = world.check(one_way);
        assert!(
            problems.iter().any(|p| p.contains("instead of back")),
            "{problems:?}"
        );
    }

    #[test]
    fn arriving_through_a_left_door_faces_into_the_room() {
        let r = room("a", "door 0 1 h=3 id=w to=b:w");
        let d = r.door("w").unwrap();
        assert_eq!(d.side(&r), Some(Side::Left));
        let ((x, y), facing) = d.arrival(&r).unwrap();
        assert_eq!((x, y, facing), (40.0, 64.0, 1));
    }
}
