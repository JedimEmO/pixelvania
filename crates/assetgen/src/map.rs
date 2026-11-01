//! The world map, drawn from the rooms themselves.
//!
//! Each room is drawn at its place on the world map, one pixel per tile, scaled up:
//! open space in a dusky rose, walls left dark, and a gold outline round the room.
//! Lounges are marked with a small gold square. The room the player is in is brighter.

use std::collections::{BTreeMap, BTreeSet};

use image::{Rgba, RgbaImage};
use world::{Room, Tile, World};

/// What to draw.
pub struct MapView<'a> {
    pub world: &'a World,
    pub rooms: &'a BTreeMap<String, Room>,
    /// The rooms to show; the rest stay hidden.
    pub shown: &'a BTreeSet<String>,
    pub current: Option<&'a str>,
    /// The player, in tiles from the current room's top-left.
    pub player: Option<(i32, i32)>,
    /// Draw the player's dot this frame; the caller blinks it.
    pub dot: bool,
}

const BACKDROP: [u8; 3] = [16, 10, 18];
const OPEN: [u8; 3] = [86, 44, 66];
const OPEN_HERE: [u8; 3] = [140, 70, 104];
const EDGE: [u8; 3] = [150, 116, 60];
const LOUNGE: [u8; 3] = [255, 214, 120];
const PLAYER: [u8; 3] = [255, 245, 235];

/// The world's extent on the map, in tiles: left, top, right, bottom (exclusive).
pub fn bounds(world: &World, rooms: &BTreeMap<String, Room>) -> (i32, i32, i32, i32) {
    let mut b = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for r in &world.rooms {
        let (Some(pos), Some(room)) = (r.pos, rooms.get(&r.name)) else {
            continue;
        };
        b.0 = b.0.min(pos.0);
        b.1 = b.1.min(pos.1);
        b.2 = b.2.max(pos.0 + room.width as i32);
        b.3 = b.3.max(pos.1 + room.height as i32);
    }
    if b.0 > b.2 { (0, 0, 1, 1) } else { b }
}

/// Draws the map into an image of `size`, `scale` pixels per tile,
/// with the tile at `centre` (world map tiles) in the middle.
pub fn draw(view: &MapView, size: (u32, u32), scale: u32, centre: (i32, i32)) -> RgbaImage {
    let mut img = RgbaImage::from_pixel(size.0, size.1, rgba(BACKDROP));
    let s = scale as i32;
    let origin = (
        size.0 as i32 / 2 - centre.0 * s,
        size.1 as i32 / 2 - centre.1 * s,
    );
    let cell = |img: &mut RgbaImage, tx: i32, ty: i32, c: [u8; 3]| {
        for dy in 0..s {
            for dx in 0..s {
                let (x, y) = (origin.0 + tx * s + dx, origin.1 + ty * s + dy);
                if x >= 0 && y >= 0 && (x as u32) < size.0 && (y as u32) < size.1 {
                    img.put_pixel(x as u32, y as u32, rgba(c));
                }
            }
        }
    };
    for r in &view.world.rooms {
        if !view.shown.contains(&r.name) {
            continue;
        }
        let (Some(pos), Some(room)) = (r.pos, view.rooms.get(&r.name)) else {
            continue;
        };
        let here = view.current == Some(r.name.as_str());
        let (w, h) = (room.width as i32, room.height as i32);
        for ty in 0..h {
            for tx in 0..w {
                let edge = tx == 0 || ty == 0 || tx == w - 1 || ty == h - 1;
                let open = room.tile(tx, ty) == Tile::Empty;
                let c = if open {
                    if here { OPEN_HERE } else { OPEN }
                } else if edge {
                    EDGE
                } else {
                    continue;
                };
                cell(&mut img, pos.0 + tx, pos.1 + ty, c);
            }
        }
        for l in room.entities_of("lounge") {
            let (x, y) = (l.x.floor() as i32, l.y.floor() as i32 - 1);
            cell(&mut img, pos.0 + x, pos.1 + y, LOUNGE);
        }
        if here
            && view.dot
            && let Some((px, py)) = view.player
        {
            cell(&mut img, pos.0 + px, pos.1 + py, PLAYER);
            cell(&mut img, pos.0 + px, pos.1 + py - 1, PLAYER);
        }
    }
    img
}

fn rgba(c: [u8; 3]) -> Rgba<u8> {
    Rgba([c[0], c[1], c[2], 255])
}

#[cfg(test)]
mod tests {
    use super::*;
    use world::WorldRoom;

    #[test]
    fn shown_rooms_are_drawn_and_hidden_ones_are_not() {
        let room = Room::parse("[legend]\n# stone\n[tiles]\n####\n#..#\n####\n").unwrap();
        let world = World {
            start: "a".into(),
            rooms: vec![
                WorldRoom {
                    name: "a".into(),
                    area: "x".into(),
                    pos: Some((0, 0)),
                },
                WorldRoom {
                    name: "b".into(),
                    area: "x".into(),
                    pos: Some((4, 0)),
                },
            ],
        };
        let rooms = BTreeMap::from([("a".to_owned(), room.clone()), ("b".to_owned(), room)]);
        let shown = BTreeSet::from(["a".to_owned()]);
        let view = MapView {
            world: &world,
            rooms: &rooms,
            shown: &shown,
            current: Some("a"),
            player: None,
            dot: false,
        };
        let img = draw(&view, (16, 6), 1, (4, 1));
        // Room a's open tile at (1, 1) lands at (8 - 4 + 1, 3 - 1 + 1).
        assert_eq!(img.get_pixel(5, 3), &rgba(OPEN_HERE));
        // Room b's open tile is hidden.
        assert_eq!(img.get_pixel(9, 3), &rgba(BACKDROP));
        assert_eq!(bounds(&world, &rooms), (0, 0, 8, 3));
    }
}
