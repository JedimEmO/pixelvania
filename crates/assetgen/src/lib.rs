//! Seeded 16 × 16 surface tiles and room painting, without Bevy.
//!
//! [`rasterize`] returns unlit sRGB pixels. Empty cells are transparent; every pixel
//! of a solid cell is opaque, so mortar and grain never produce false rim lights.

pub mod backgrounds;
pub mod furniture;
pub mod hanging;
pub mod map;
#[cfg(feature = "sfx")]
pub mod music;
#[cfg(feature = "sfx")]
pub mod sfx;

use image::{Rgba, RgbaImage};
use world::{Room, TILE, Tile};

pub const MATERIALS: [&str; 5] = ["stone", "wood", "velvet", "gilt", "cracked"];
pub const VARIANTS: u32 = 8;
pub const SIZE: u32 = TILE as u32;

/// Same-material neighbours, clockwise from north: N, NE, E, SE, S, SW, W, NW.
/// Diagonals affect concave corners only when both adjoining sides are connected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Neighbours(pub u8);

impl Neighbours {
    pub const INTERIOR: Self = Self(255);
    pub const ISOLATED: Self = Self(0);

    fn has(self, direction: u8) -> bool {
        self.0 & (1 << direction) != 0
    }

    /// Reads all eight neighbours. Outside the image is exposed, even though
    /// `Room::tile` treats it as solid for collision.
    pub fn at(room: &Room, x: i32, y: i32) -> Self {
        let tile = room.tile(x, y);
        let mut mask = 0;
        for (bit, (dx, dy)) in [
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ]
        .into_iter()
        .enumerate()
        {
            let (nx, ny) = (x + dx, y + dy);
            if nx >= 0
                && ny >= 0
                && nx < room.width as i32
                && ny < room.height as i32
                && room.tile(nx, ny) == tile
                && tile != Tile::Empty
            {
                mask |= 1 << bit;
            }
        }
        Self(mask)
    }
}

/// Five sRGB tones, from seam to worn edge. These are albedo, not baked lighting.
pub fn palette(material: &str) -> Option<[[u8; 3]; 5]> {
    Some(match material {
        "stone" => [
            [16, 16, 21],
            [25, 25, 31],
            [34, 33, 40],
            [43, 42, 49],
            [58, 56, 63],
        ],
        "wood" => [
            [19, 16, 17],
            [29, 23, 22],
            [40, 32, 28],
            [50, 41, 34],
            [65, 54, 43],
        ],
        "velvet" => [
            [19, 14, 21],
            [29, 21, 30],
            [39, 28, 39],
            [48, 35, 47],
            [60, 45, 58],
        ],
        "gilt" => [
            [21, 19, 16],
            [32, 29, 23],
            [44, 39, 29],
            [58, 51, 36],
            [76, 67, 47],
        ],
        // Stone that the Plunge breaks: a touch warmer and lighter than the rest, so it reads.
        "cracked" => [
            [24, 20, 22],
            [42, 36, 38],
            [58, 50, 50],
            [74, 64, 60],
            [96, 82, 72],
        ],
        _ => return None,
    })
}

/// Stone split by jagged cracks that run from block to block.
fn cracked(x: u32, y: u32, v: u32) -> usize {
    // A crack zigzags down the tile, stepping a pixel sideways every two rows,
    // with a branch off it; both are the darkest tone.
    let main = 4 + v % 5 + [0, 1, 1, 2, 1, 0, 0, 1][(y / 2) as usize % 8];
    if x == main || (y.is_multiple_of(3) && x == main + 1) {
        return 0;
    }
    let branch_y = 5 + v % 6;
    if y == branch_y && x > main && x <= main + 4 + v % 3 {
        return 0;
    }
    if x == main + 1 || (y == branch_y + 1 && x > main && x <= main + 3) {
        return 1;
    }
    stone(x, y, v)
}

fn hash(x: u32, y: u32) -> u32 {
    let mut h = x.wrapping_mul(0x9e37_79b9) ^ y.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^ (h >> 12)
}

fn stone(x: u32, y: u32, v: u32) -> usize {
    // Two staggered courses; joints meet across tiles, wear stays inside each block.
    let bx = (x + if y < 8 { 0 } else { 8 }) % 16;
    let by = y % 8;
    if by == 0 || bx == 0 {
        return 0;
    }
    if by == 1 {
        return 3;
    }
    if bx == 15 || by == 7 {
        return 1;
    }
    if y / 8 == v % 2 && by == 2 + v % 3 && bx >= 3 + v % 4 && bx <= 5 + v % 4 {
        return 1;
    }
    if v.is_multiple_of(3) && bx > 10 && by > 4 {
        return 3;
    }
    2
}

fn wood(x: u32, y: u32, v: u32) -> usize {
    // Horizontal boards with long, sparse grain and an occasional knot.
    let by = y % 8;
    if by == 0 {
        return 0;
    }
    if by == 1 {
        return 3;
    }
    if by == 7 {
        return 1;
    }
    let kx = 4 + v;
    let ky = 3 + (v % 2);
    let distance = x.abs_diff(kx) + by.abs_diff(ky) * 2;
    if y < 8 && v.is_multiple_of(2) && distance <= 2 {
        return if distance == 2 { 3 } else { 1 };
    }
    if y / 8 == v % 2 && by == 3 + v % 3 && (x + v * 3) % 16 < 10 {
        return 1;
    }
    2
}

fn velvet(x: u32, y: u32, v: u32) -> usize {
    // Broad folds with stepped shoulders, never per-pixel noise.
    let fold = (x + v % 4 + u32::from(y > 5 + v % 5)) % 16;
    match fold {
        0..=1 => 0,
        2..=4 | 14..=15 => 1,
        5..=7 => 2,
        8..=10 => 3,
        _ => 2,
    }
}

fn gilt(x: u32, y: u32, v: u32) -> usize {
    // A small lozenge and curled leaves between two tarnished rails.
    if y == 1 && x == 3 + v {
        return 2;
    }
    if y == 1 || y == 14 {
        return 3;
    }
    if y == 2 || y == 15 {
        return 0;
    }
    let dx = x.abs_diff(7);
    let dy = y.abs_diff(8);
    if dx + dy == 4 {
        return if x < 8 { 4 } else { 2 };
    }
    if dx + dy < 4 {
        return 1;
    }
    let leaf_y = if v.is_multiple_of(2) { 6 } else { 10 };
    if (x == 1 || x == 13) && y.abs_diff(leaf_y) <= 1 {
        return 3;
    }
    if (x == 2 || x == 12) && y == leaf_y - 1 {
        return 2;
    }
    if y == 8 && (x <= 2 + v % 3 || x >= 12 + v % 3) {
        return 2;
    }
    1
}

fn edge(x: u32, y: u32, n: Neighbours) -> Option<usize> {
    // Undersides stay dark. The top lip is a quiet bevel; the shader supplies light.
    if !n.has(4) && y >= 14 {
        return Some(if y == 15 { 0 } else { 1 });
    }
    if (!n.has(6) && x == 0) || (!n.has(2) && x == 15) {
        return Some(1);
    }
    if !n.has(0) && y <= 1 {
        return Some(if y == 0 { 4 } else { 3 });
    }
    if !n.has(6) && x == 1 {
        return Some(3);
    }
    if !n.has(2) && x == 14 {
        return Some(1);
    }
    for (a, diagonal, b, cx, cy) in [
        (0, 1, 2, 15, 0),
        (2, 3, 4, 15, 15),
        (4, 5, 6, 0, 15),
        (6, 7, 0, 0, 0),
    ] {
        if n.has(a) && n.has(b) && !n.has(diagonal) && x.abs_diff(cx) + y.abs_diff(cy) <= 2 {
            return Some(if cy == 0 { 3 } else { 1 });
        }
    }
    None
}

/// Draws a seeded variant. Unknown materials produce an opaque magenta checker.
/// Seeds 0 through [`VARIANTS`] minus one enumerate the authored variations.
pub fn tile(material: &str, seed: u32, neighbours: Neighbours) -> RgbaImage {
    let ramp = palette(material);
    RgbaImage::from_fn(SIZE, SIZE, |x, y| {
        let Some(ramp) = ramp else {
            return Rgba(if (x / 4 + y / 4).is_multiple_of(2) {
                [180, 30, 150, 255]
            } else {
                [40, 10, 38, 255]
            });
        };
        let v = seed % VARIANTS;
        let tone = edge(x, y, neighbours).unwrap_or_else(|| match material {
            "stone" => stone(x, y, v),
            "wood" => wood(x, y, v),
            "velvet" => velvet(x, y, v),
            "gilt" => gilt(x, y, v),
            "cracked" => cracked(x, y, v),
            _ => unreachable!("palette and generators share the material list"),
        });
        let [r, g, b] = ramp[tone];
        Rgba([r, g, b, 255])
    })
}

/// Paints a room at one pixel per game pixel. Seeds depend only on tile coordinates.
pub fn rasterize(room: &Room) -> RgbaImage {
    let (w, h) = room.pixel_size();
    let mut image = RgbaImage::new(w as u32, h as u32);
    for y in 0..room.height as i32 {
        for x in 0..room.width as i32 {
            if let Tile::Solid(m) = room.tile(x, y) {
                let material = room.materials.get(m as usize).map_or("?", String::as_str);
                let tile = tile(
                    material,
                    hash(x as u32, y as u32),
                    Neighbours::at(room, x, y),
                );
                image::imageops::replace(
                    &mut image,
                    &tile,
                    i64::from(x * TILE),
                    i64::from(y * TILE),
                );
            }
        }
    }
    image
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashSet, fs, path::Path};

    #[test]
    fn deterministic_and_varied_with_only_opaque_palette_colours() {
        for material in MATERIALS {
            let images: Vec<_> = (0..VARIANTS)
                .map(|v| tile(material, v, Neighbours::INTERIOR))
                .collect();
            let unique: HashSet<_> = images.iter().map(|i| i.as_raw()).collect();
            assert_eq!(unique.len(), VARIANTS as usize, "{material}");
            for mask in 0..=255 {
                let a = tile(material, 12345, Neighbours(mask));
                assert_eq!(a, tile(material, 12345, Neighbours(mask)));
                assert!(a.pixels().all(
                    |p| p[3] == 255 && palette(material).unwrap().contains(&[p[0], p[1], p[2]])
                ));
            }
        }
    }

    #[test]
    fn room_pixels_preserve_collision_coverage_and_determinism() {
        let room = Room::parse(
            "[legend]\n# stone\nw wood\nv velvet\ng gilt\n? missing\n[tiles]\n#wvg?\n.....\n",
        )
        .unwrap();
        let a = rasterize(&room);
        assert_eq!(a.dimensions(), (80, 32));
        assert_eq!(a, rasterize(&room));
        for (x, y, p) in a.enumerate_pixels() {
            if room.tile((x / SIZE) as i32, (y / SIZE) as i32) == Tile::Empty {
                assert_eq!(p.0, [0; 4]);
            } else {
                assert_eq!(p[3], 255);
            }
        }
        assert_eq!(*a.get_pixel(64, 0), Rgba([180, 30, 150, 255]));
    }

    #[test]
    fn all_eight_neighbours_and_material_boundaries_affect_edges() {
        let full = Room::parse("[legend]\n# stone\nw wood\n[tiles]\n###\n###\n###\n").unwrap();
        let offsets = [1, 2, 5, 8, 7, 6, 3, 0];
        let interior = tile("stone", 0, Neighbours::INTERIOR);
        assert_eq!(Neighbours::at(&full, 1, 1), Neighbours::INTERIOR);
        for (bit, index) in offsets.into_iter().enumerate() {
            for replacement in [Tile::Empty, Tile::Solid(1)] {
                let mut room = full.clone();
                room.tiles[index] = replacement;
                let n = Neighbours::at(&room, 1, 1);
                assert_eq!(n.0, 255 ^ (1 << bit));
                assert_ne!(tile("stone", 0, n), interior);
            }
        }
        let single = Room::parse("[legend]\n# stone\n[tiles]\n#\n").unwrap();
        assert_eq!(Neighbours::at(&single, 0, 0), Neighbours::ISOLATED);
    }

    #[test]
    fn every_repository_room_material_has_a_generator() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/rooms");
        let mut count = 0;
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "room") {
                let room = Room::parse(&fs::read_to_string(&path).unwrap()).unwrap();
                for m in room.materials {
                    assert!(MATERIALS.contains(&m.as_str()), "{}: {m}", path.display());
                    assert!(palette(&m).is_some());
                }
                count += 1;
            }
        }
        assert!(count > 0);
    }
}
