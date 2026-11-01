//! Seeded backdrop silhouettes: the ballroom's windows, the cellars' vaults, the backstage,
//! the foyer, the banquet and the garden under glass.
//! Pixels carry their distant lighting, in sRGB.
//!
//! Each set is four repeating strips, farthest first, the last drawn in front of the room.
//! A room picks one with `background = <set>` and may tint its distance with a colour after it,
//! as in `background = ballroom 345a32`.

use image::{Rgba, RgbaImage};

pub const WIDTH: u32 = 960;
pub const HEIGHT: u32 = 400;
pub const DEFAULT_SEED: u32 = 0xba11_0042;
pub const DEFAULT_LIGHT: [u8; 3] = [96, 45, 67];

/// One repeating strip, ordered from farthest to foreground by [`ballroom`].
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub name: &'static str,
    pub image: RgbaImage,
    /// Camera travel relative to the room: zero stays still, one follows the tiles.
    pub parallax: f32,
    pub foreground: bool,
}

struct Canvas(RgbaImage);

impl Canvas {
    fn new(fill: [u8; 4]) -> Self {
        Self(RgbaImage::from_pixel(WIDTH, HEIGHT, Rgba(fill)))
    }

    fn pixel(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if (0..HEIGHT as i32).contains(&y) {
            self.0
                .put_pixel(x.rem_euclid(WIDTH as i32) as u32, y as u32, Rgba(color));
        }
    }

    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: [u8; 4]) {
        for py in y..y + h {
            for px in x..x + w {
                self.pixel(px, py, color);
            }
        }
    }

    fn line(&mut self, from: (i32, i32), to: (i32, i32), width: i32, color: [u8; 4]) {
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        let steps = dx.abs().max(dy.abs()).max(1);
        for i in 0..=steps {
            self.rect(
                from.0 + dx * i / steps,
                from.1 + dy * i / steps,
                width,
                width,
                color,
            );
        }
    }

    fn ellipse(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, color: [u8; 4]) {
        for y in -ry..=ry {
            for x in -rx..=rx {
                if i64::from(x * x) * i64::from(ry * ry) + i64::from(y * y) * i64::from(rx * rx)
                    <= i64::from(rx * rx) * i64::from(ry * ry)
                {
                    self.pixel(cx + x, cy + y, color);
                }
            }
        }
    }

    fn arch(&mut self, x: i32, top: i32, width: i32, bottom: i32, color: [u8; 4]) {
        let radius = width / 2;
        self.ellipse(x, top + radius, radius, radius, color);
        self.rect(
            x - radius,
            top + radius,
            width + 1,
            bottom - top - radius,
            color,
        );
    }

    /// Courses of brick: mortar lines every six rows, and joints that stagger by half a brick.
    /// No joint falls on the strip's first or last column, so its ends match.
    fn bricks(&mut self, top: i32, bottom: i32, mortar: [u8; 4]) {
        for y in top..bottom {
            let course = (y - top) / 6;
            if (y - top) % 6 == 5 {
                self.rect(0, y, WIDTH as i32, 1, mortar);
                continue;
            }
            let shift = if course % 2 == 0 { 4 } else { 12 };
            for x in (shift..WIDTH as i32).step_by(16) {
                self.pixel(x, y, mortar);
            }
        }
    }

    fn layer(self, name: &'static str, parallax: f32, foreground: bool) -> Layer {
        Layer {
            name,
            image: self.0,
            parallax,
            foreground,
        }
    }
}

fn noise(seed: u32, index: u32) -> u32 {
    let mut n = seed.wrapping_add(index.wrapping_mul(0x9e37_79b9));
    n = (n ^ (n >> 16)).wrapping_mul(0x85eb_ca6b);
    n = (n ^ (n >> 13)).wrapping_mul(0xc2b2_ae35);
    n ^ (n >> 16)
}

fn tint(base: [u8; 3], light: [u8; 3], amount: u16) -> [u8; 4] {
    let mut c = [0, 0, 0, 255];
    for i in 0..3 {
        c[i] = ((u16::from(base[i]) * (256 - amount) + u16::from(light[i]) * amount) / 256) as u8;
    }
    c
}

/// Generates three background layers and one sparse foreground, without antialiasing.
/// Geometry stays away from the strip seam; continuous mouldings have identical end columns.
/// The dominant light tints the distant haze and window accents, never the room's lights.
pub fn ballroom(seed: u32, dominant_light: [u8; 3]) -> Vec<Layer> {
    let haze = tint([10, 11, 17], dominant_light, 16);
    let stone = tint([14, 15, 21], dominant_light, 20);
    let frame = tint([20, 19, 26], dominant_light, 26);
    let glass = tint([24, 27, 38], dominant_light, 38);
    let moon = tint([44, 46, 59], dominant_light, 36);
    let cloth = tint([9, 8, 13], dominant_light, 12);
    let mut far = Canvas::new(haze);
    far.rect(0, 20, WIDTH as i32, 5, stone);
    far.rect(0, 29, WIDTH as i32, 2, frame);
    far.rect(0, 320, WIDTH as i32, 80, stone);
    for bay in 0..6 {
        let x = 80 + bay * 160;
        let n = noise(seed, bay as u32);
        far.arch(x, 44, 80, 310, frame);
        far.arch(x, 49, 70, 305, cloth);
        far.arch(x, 53, 62, 302, glass);
        // A sliver of moonlight; most of each pane stays dark.
        far.rect(x - 27, 88, 3, 210, moon);
        far.rect(x - 30, 300, 61, 2, frame);
        far.rect(x - 2, 59, 4, 244, frame);
        far.rect(x - 31, 132, 63, 4, frame);
        far.rect(x - 31, 216, 63, 4, frame);
        far.line((x - 23, 66), (x + 23, 113), 2, frame);
        far.line((x + 23, 66), (x - 23, 113), 2, frame);
        // Missing glass and a crooked crack interrupt the formal repetition.
        let broken_y = 155 + (n % 40) as i32;
        far.line((x + 8, broken_y), (x + 21, broken_y + 17), 2, cloth);
        far.line((x + 21, broken_y + 17), (x + 17, broken_y + 35), 1, cloth);
        far.rect(x - 49, 312, 99, 5, frame);
        for side in [-1, 1] {
            for y in 40..290 {
                let width = if y < 110 {
                    19 - (y - 40) / 7
                } else {
                    8 + (y - 110) / 19
                };
                let left = if side < 0 { x - 53 } else { x + 54 - width };
                if y < 277 + (n % 12) as i32 || (left + y) % 7 < 4 {
                    far.rect(left, y, width, 1, cloth);
                }
            }
        }
    }

    let mut middle = Canvas::new([0; 4]);
    let dark = [10, 9, 14, 255];
    let edge = tint([13, 12, 18], dominant_light, 12);
    let metal = [7, 7, 11, 255];
    for x in [20, 340, 660] {
        middle.rect(x, 27, 21, 321, dark);
        middle.rect(x + 3, 35, 3, 294, edge);
        middle.rect(x - 6, 23, 33, 8, edge);
        middle.rect(x - 3, 32, 27, 7, dark);
        middle.rect(x - 6, 338, 33, 8, edge);
    }
    middle.rect(0, 316, WIDTH as i32, 5, edge);
    middle.rect(0, 357, WIDTH as i32, 7, dark);
    for x in (12..WIDTH as i32 - 12).step_by(24) {
        // A few missing balusters are all that remains of the damage detail.
        if noise(seed, x as u32).is_multiple_of(9) {
            continue;
        }
        middle.rect(x - 3, 321, 7, 4, dark);
        middle.ellipse(x, 335, 3, 10, dark);
        middle.rect(x - 2, 344, 5, 13, dark);
    }
    for (i, x) in [180, 500, 820].into_iter().enumerate() {
        let y = 104 + (noise(seed, 30 + i as u32) % 23) as i32;
        middle.line((x, 0), (x, y + 20), 1, metal);
        middle.ellipse(x, y + 25, 6, 10, metal);
        for side in [-1, 1] {
            for arm in [17, 34] {
                middle.line((x, y + 23), (x + side * arm, y + 12), 2, metal);
                middle.line(
                    (x + side * arm, y + 12),
                    (x + side * (arm + 4), y - 4),
                    2,
                    metal,
                );
                middle.rect(x + side * (arm + 4) - 3, y - 6, 8, 3, metal);
                middle.rect(x + side * (arm + 4), y - 15, 2, 9, edge);
                middle.line((x + side * arm, y + 16), (x + side * arm, y + 27), 1, edge);
            }
        }
        middle.ellipse(x, y + 40, 2, 4, edge);
        if i == 1 {
            middle.rect(x + 21, y - 18, 1, 3, [85, 48, 29, 255]);
            middle.pixel(x + 21, y - 17, [115, 72, 39, 255]);
        }
    }

    let mut near = Canvas::new([0; 4]);
    for x in [58, 698] {
        near.rect(x - 11, 0, 22, 400, [6, 6, 10, 255]);
        near.rect(x - 15, 22, 30, 9, [9, 8, 13, 255]);
        near.rect(x - 15, 367, 30, 18, [8, 7, 11, 255]);
        near.rect(x - 8, 35, 2, 328, [10, 9, 14, 255]);
    }
    // Hanging swags frame the ceiling without crossing the play space.
    for x in 100i32..600 {
        let d = (x - 350).abs();
        let hem = 22 + (250 * 250 - d * d) / 2400;
        near.rect(x, 0, 1, hem, [7, 6, 10, 255]);
        near.pixel(x, hem, [11, 8, 13, 255]);
    }

    let mut front = Canvas::new([0; 4]);
    // Broad two-pixel contours suggest defocus without filtering or antialiasing.
    for y in (0..166).step_by(2) {
        let width = 32 - y / 6;
        front.rect(9, y, width + 4, 2, [5, 4, 8, 255]);
        front.rect(9, y, width, 2, [3, 3, 5, 255]);
    }
    let (px, py) = (904, 352);
    front.line((px + 4, 379), (px, py - 24), 4, [3, 4, 6, 255]);
    for (dx, dy) in [
        (-48, -22),
        (-36, -46),
        (-16, -58),
        (18, -50),
        (42, -30),
        (52, -4),
        (-52, 4),
    ] {
        for t in 0..=32 {
            let x = px + dx * t / 32;
            let y = py - 24 + (2 * (32 - t) * t * (dy - 14) + t * t * (24 + dy)) / 1024;
            let thickness = (8 * (32 - t) / 32).max(1);
            front.rect(x, y, 2, thickness + 2, [5, 6, 8, 255]);
            front.rect(x, y, 2, thickness, [3, 4, 6, 255]);
        }
    }
    front.rect(px - 12, 367, 28, 5, [5, 4, 7, 255]);
    front.rect(px - 9, 372, 22, 20, [3, 3, 5, 255]);

    vec![
        far.layer("windows", 0.12, false),
        middle.layer("gallery", 0.32, false),
        near.layer("pillars", 0.62, false),
        front.layer("foreground", 1.18, true),
    ]
}

/// The backdrop sets a room can name.
pub const SETS: [&str; 6] = [
    "ballroom",
    "cellars",
    "backstage",
    "foyer",
    "banquet",
    "garden",
];

/// The set with this name, or `None` if there is no such set.
pub fn set(name: &str, seed: u32, light: [u8; 3]) -> Option<Vec<Layer>> {
    match name {
        "ballroom" => Some(ballroom(seed, light)),
        "cellars" => Some(cellars(seed, light)),
        "backstage" => Some(backstage(seed, light)),
        "foyer" => Some(foyer(seed, light)),
        "banquet" => Some(banquet(seed, light)),
        "garden" => Some(garden(seed, light)),
        _ => None,
    }
}

/// The foyer: panelled walls with tall doors that stay shut, a grand stair climbing the distance,
/// and columns hung with sconces. Candlelight, seen from far off.
pub fn foyer(seed: u32, dominant_light: [u8; 3]) -> Vec<Layer> {
    let panel = tint([14, 11, 10], dominant_light, 18);
    let moulding = tint([22, 16, 13], dominant_light, 28);
    let shadow = tint([9, 7, 7], dominant_light, 8);
    let door = tint([11, 8, 8], dominant_light, 12);
    let glow = tint([40, 28, 18], dominant_light, 70);
    let mut far = Canvas::new(panel);
    // A dado rail and a cornice run the whole strip.
    far.rect(0, 26, WIDTH as i32, 4, moulding);
    far.rect(0, 30, WIDTH as i32, 2, shadow);
    far.rect(0, 250, WIDTH as i32, 3, moulding);
    far.rect(0, 330, WIDTH as i32, 70, shadow);
    far.rect(0, 328, WIDTH as i32, 2, moulding);
    // Panels between the doors.
    for x in (20..WIDTH as i32 - 20).step_by(40) {
        far.rect(x, 262, 28, 56, shadow);
        far.rect(x + 1, 263, 26, 54, panel);
    }
    for bay in 0..4 {
        let x = 120 + bay * 240;
        let n = noise(seed, 600 + bay as u32);
        // Tall double doors under a fanlight; a thread of light where they meet.
        far.arch(x, 70, 92, 328, moulding);
        far.arch(x, 76, 80, 328, door);
        far.rect(x - 1, 120, 2, 208, shadow);
        far.rect(x - 40, 118, 80, 3, moulding);
        for i in 0..5 {
            far.line((x, 116), (x - 34 + i * 17, 84), 1, moulding);
        }
        if !n.is_multiple_of(3) {
            far.rect(x, 140, 1, 186, glow);
        }
        // Panels on the leaves.
        for side in [-1, 1] {
            let px = x + side * 20 - 12;
            far.rect(px, 140, 24, 70, shadow);
            far.rect(px, 224, 24, 90, shadow);
        }
        // A portrait between this door and the next, but not across the strip's seam.
        if bay == 3 {
            continue;
        }
        let fx = x + 120;
        let fy = 110 + (n % 30) as i32;
        far.rect(fx - 22, fy, 44, 56, moulding);
        far.rect(fx - 18, fy + 4, 36, 48, shadow);
        far.ellipse(fx, fy + 22, 8, 10, door);
        far.rect(fx - 12, fy + 34, 24, 18, door);
    }

    // The grand stair: a flight climbing the distance, with its balusters.
    let mut middle = Canvas::new([0; 4]);
    let dark = [9, 7, 8, 255];
    let edge = tint([16, 12, 11], dominant_light, 18);
    for flight in 0..2 {
        let x0 = 60 + flight * 480;
        for step in 0..22 {
            let x = x0 + step * 14;
            let y = 340 - step * 9;
            middle.rect(x, y, 16, 360 - y, dark);
            middle.rect(x, y, 16, 1, edge);
            // A baluster every other step, and the handrail over them.
            if step % 2 == 0 {
                middle.rect(x + 6, y - 22, 2, 22, dark);
            }
        }
        middle.line((x0, 318), (x0 + 22 * 14, 318 - 22 * 9), 2, dark);
        middle.line((x0, 317), (x0 + 22 * 14, 317 - 22 * 9), 1, edge);
        // The landing at the top.
        let lx = x0 + 22 * 14;
        middle.rect(lx, 142, 90, 8, dark);
        middle.rect(lx, 142, 90, 1, edge);
        for b in (lx + 4..lx + 88).step_by(8) {
            middle.rect(b, 122, 2, 20, dark);
        }
        middle.rect(lx, 120, 90, 3, dark);
    }
    middle.rect(0, 352, WIDTH as i32, 8, dark);

    let mut near = Canvas::new([0; 4]);
    let column = [6, 5, 6, 255];
    let lit = [12, 9, 9, 255];
    let flame = tint([60, 40, 22], dominant_light, 90);
    for x in [180, 660] {
        near.rect(x - 13, 0, 26, 400, column);
        near.rect(x - 17, 18, 34, 10, lit);
        near.rect(x - 17, 360, 34, 24, lit);
        near.rect(x - 5, 28, 2, 332, lit);
        // A sconce on the column, two candles, lit far off.
        near.rect(x - 14, 200, 28, 3, lit);
        for c in [-10, 10] {
            near.rect(x + c - 1, 190, 3, 10, lit);
            near.pixel(x + c, 188, flame);
            near.pixel(x + c, 187, flame);
        }
    }

    let mut front = Canvas::new([0; 4]);
    // A fern on a stand, close and out of focus, in the bottom corner.
    let (px, py) = (60, 352);
    front.rect(px - 8, py, 16, 48, [3, 3, 4, 255]);
    for (dx, dy) in [
        (-44, -30),
        (-30, -52),
        (-8, -62),
        (16, -56),
        (38, -36),
        (48, -10),
        (-50, -6),
    ] {
        for t in 0..=24 {
            let x = px + dx * t / 24;
            let y = py - 4 + (dy * t / 24) + (t * t) / 30;
            let thickness = (6 * (24 - t) / 24).max(1);
            front.rect(x, y, 2, thickness + 1, [5, 5, 6, 255]);
            front.rect(x, y, 2, thickness, [3, 3, 4, 255]);
        }
    }

    vec![
        far.layer("doors", 0.12, false),
        middle.layer("grand stair", 0.32, false),
        near.layer("columns", 0.62, false),
        front.layer("foreground", 1.18, true),
    ]
}

/// The cellars: brick vaults of wine racks, heavy arches, barrels, and a cage lamp long out.
pub fn cellars(seed: u32, dominant_light: [u8; 3]) -> Vec<Layer> {
    let brick = tint([13, 11, 12], dominant_light, 18);
    let mortar = tint([9, 8, 9], dominant_light, 8);
    let rim = tint([22, 18, 18], dominant_light, 30);
    let hollow = [5, 4, 6, 255];
    let glass = tint([20, 24, 18], dominant_light, 44);
    let mut far = Canvas::new(brick);
    far.bricks(0, 330, mortar);
    far.rect(0, 330, WIDTH as i32, 70, mortar);
    far.rect(0, 328, WIDTH as i32, 2, rim);
    for bay in 0..6 {
        let x = 80 + bay * 160;
        let n = noise(seed, 100 + bay as u32);
        // An alcove of racks: bottle ends in rows, a few catching the light.
        far.arch(x, 150, 104, 328, rim);
        far.arch(x, 154, 96, 328, hollow);
        for row in 0..((328 - 210) / 7) {
            let y = 206 + row * 7;
            for col in 0..12 {
                let bx = x - 44 + col * 8;
                if noise(seed, (bay * 400 + row * 20 + col) as u32).is_multiple_of(9) {
                    continue;
                }
                far.rect(bx, y, 3, 3, mortar);
                if noise(n, (row * 20 + col) as u32).is_multiple_of(7) {
                    far.pixel(bx + 1, y + 1, glass);
                }
            }
        }
        // A crack down the wall between alcoves.
        let cx = x + 70;
        let mut y = 20 + (n % 60) as i32;
        let mut dx = 0;
        while y < 140 + (n % 50) as i32 {
            far.pixel(cx + dx, y, mortar);
            y += 1;
            if noise(n, y as u32).is_multiple_of(4) {
                dx = (dx
                    + if noise(n, y as u32 + 7).is_multiple_of(2) {
                        1
                    } else {
                        -1
                    })
                .clamp(-5, 5);
            }
        }
    }

    // The vault: piers with arches springing between them.
    let mut middle = Canvas::new([0; 4]);
    let dark = [8, 7, 9, 255];
    let edge = tint([16, 13, 14], dominant_light, 16);
    for pier in 0..3 {
        let x = 160 + pier * 320;
        middle.rect(x - 18, 60, 36, 300, dark);
        middle.rect(x - 22, 56, 44, 8, edge);
        middle.rect(x - 22, 340, 44, 20, edge);
        middle.rect(x - 15, 64, 2, 276, edge);
    }
    for span in 0..3 {
        let left = 160 + span * 320 + 18;
        let right = left + 284;
        let mid = (left + right) / 2;
        for x in left..right {
            let d = (x - mid) as f32 / 142.0;
            let springing = (60.0 + 50.0 * d * d).round() as i32;
            middle.rect(x, 0, 1, springing, dark);
            middle.pixel(x, springing, edge);
        }
    }
    // Barrels on their sides along the floor, two high in places.
    for i in 0..9 {
        let n = noise(seed, 200 + i);
        let x = 40 + i as i32 * 106 + (n % 20) as i32;
        let high = 1 + n.is_multiple_of(3) as i32;
        for level in 0..high {
            let cy = 344 - level * 26;
            middle.ellipse(x, cy, 16, 13, dark);
            middle.rect(x - 12, cy - 13, 1, 26, edge);
            middle.rect(x + 11, cy - 13, 1, 26, edge);
            middle.ellipse(x, cy, 3, 3, edge);
        }
    }

    let mut near = Canvas::new([0; 4]);
    let black = [5, 5, 7, 255];
    let iron = [11, 10, 12, 255];
    for x in [120, 600] {
        near.rect(x - 14, 0, 28, 400, black);
        near.rect(x - 18, 366, 36, 20, iron);
        near.rect(x - 6, 0, 2, 366, iron);
    }
    // A cage lamp on a chain, unlit.
    let lx = 360 + (noise(seed, 300) % 40) as i32;
    let ly = 96 + (noise(seed, 301) % 30) as i32;
    for y in (0..ly).step_by(3) {
        near.rect(lx, y, 1, 2, iron);
    }
    near.rect(lx - 6, ly, 13, 2, iron);
    for dx in [-6, -2, 2, 6] {
        near.rect(lx + dx, ly + 2, 1, 16, iron);
    }
    near.rect(lx - 6, ly + 18, 13, 2, iron);
    // Cobwebs in the vault's corners.
    for (cx, dir) in [(134, 1), (586, -1)] {
        for t in 0..14 {
            near.line((cx, 10 + t * 3), (cx + dir * (14 - t) * 3, 10), 1, iron);
        }
    }

    let mut front = Canvas::new([0; 4]);
    // A crate and a bottle, close and out of focus, in the bottom corner.
    front.rect(860, 330, 70, 70, [3, 3, 5, 255]);
    front.rect(864, 334, 62, 2, [6, 5, 8, 255]);
    front.rect(864, 360, 62, 2, [6, 5, 8, 255]);
    front.rect(838, 300, 12, 100, [3, 3, 5, 255]);
    front.rect(841, 280, 6, 20, [3, 3, 5, 255]);

    vec![
        far.layer("racks", 0.12, false),
        middle.layer("vault", 0.32, false),
        near.layer("pillars", 0.62, false),
        front.layer("foreground", 1.18, true),
    ]
}

/// Backstage: a brick wall hung with flats of painted scenery, fly lines and sandbags,
/// a catwalk truss, and the edge of a drape.
pub fn backstage(seed: u32, dominant_light: [u8; 3]) -> Vec<Layer> {
    let brick = tint([12, 12, 15], dominant_light, 16);
    let mortar = tint([8, 8, 10], dominant_light, 8);
    let canvas = tint([20, 18, 22], dominant_light, 34);
    let paint = tint([30, 30, 40], dominant_light, 46);
    let shade = tint([15, 14, 18], dominant_light, 22);
    let mut far = Canvas::new(brick);
    far.bricks(0, 340, mortar);
    far.rect(0, 340, WIDTH as i32, 60, mortar);
    // Flats leaning on the wall: a moon, a row of trees, a doorway that leads nowhere.
    for (i, x) in [90, 400, 720].into_iter().enumerate() {
        let n = noise(seed, 400 + i as u32);
        let (w, h) = (150 + (n % 40) as i32, 190 + (n % 50) as i32);
        let top = 340 - h;
        far.rect(x, top, w, h, canvas);
        far.rect(x, top, w, 2, shade);
        far.rect(x, top, 2, h, shade);
        match i {
            0 => {
                far.ellipse(x + w / 2, top + 70, 40, 40, paint);
                far.ellipse(x + w / 2 + 14, top + 62, 34, 36, canvas);
            }
            1 => {
                for t in 0..5 {
                    let tx = x + 15 + t * (w - 30) / 4;
                    let th = 60 + (noise(n, t as u32) % 50) as i32;
                    for row in 0..th {
                        let half = row * 14 / th;
                        far.rect(tx - half, top + h - 20 - th + row, half * 2 + 1, 1, paint);
                    }
                    far.rect(tx - 1, top + h - 20, 3, 14, paint);
                }
            }
            _ => {
                far.arch(x + w / 2, top + 30, 70, top + h - 10, paint);
                far.arch(x + w / 2, top + 36, 58, top + h - 10, shade);
            }
        }
        // A brace behind each flat, seen where it meets the floor.
        far.line((x + w - 10, top + 20), (x + w + 30, 340), 2, shade);
    }

    let mut middle = Canvas::new([0; 4]);
    let rope = tint([16, 15, 18], dominant_light, 14);
    let bag = [9, 8, 11, 255];
    // Fly lines down from the grid, each with a sandbag at its own height.
    for i in 0..18 {
        let n = noise(seed, 500 + i);
        let x = 26 + i as i32 * 53 + (n % 9) as i32;
        let y = 60 + (n % 200) as i32;
        middle.rect(x, 0, 1, y, rope);
        middle.ellipse(x, y + 8, 6, 9, bag);
        middle.rect(x - 2, y - 2, 5, 2, rope);
    }
    // A batten with a drop half flown in, folded.
    middle.rect(0, 40, WIDTH as i32, 3, bag);
    for x in 300i32..560 {
        let fold = ((x - 300) / 12) % 2;
        let hem = 150 + ((x - 430).abs() / 10);
        middle.rect(x, 43, 1, hem, if fold == 0 { bag } else { rope });
    }

    let mut near = Canvas::new([0; 4]);
    let steel = [6, 6, 8, 255];
    let lit = [11, 10, 13, 255];
    // A catwalk truss across the top: two chords and a zigzag web.
    near.rect(0, 70, WIDTH as i32, 3, steel);
    near.rect(0, 92, WIDTH as i32, 3, steel);
    near.rect(0, 69, WIDTH as i32, 1, lit);
    for x in (10..WIDTH as i32 - 10).step_by(20) {
        near.line((x, 73), (x + 10, 92), 1, steel);
        near.line((x + 10, 73), (x + 20, 92), 1, steel);
    }
    // A ladder up to it.
    for x in [250, 268] {
        near.rect(x, 95, 3, 305, steel);
    }
    for y in (110..400).step_by(14) {
        near.rect(250, y, 21, 2, steel);
    }

    let mut front = Canvas::new([0; 4]);
    // The edge of a drape, in deep folds, out of focus.
    for y in (0..400).step_by(2) {
        let sway = ((y as f32 / 40.0).sin() * 3.0).round() as i32;
        front.rect(900 + sway, y, 40, 2, [3, 3, 5, 255]);
        front.rect(900 + sway, y, 3, 2, [6, 4, 7, 255]);
        front.rect(918 + sway, y, 2, 2, [6, 4, 7, 255]);
    }

    vec![
        far.layer("flats", 0.12, false),
        middle.layer("fly lines", 0.32, false),
        near.layer("catwalk", 0.62, false),
        front.layer("foreground", 1.18, true),
    ]
}

/// The banquet: damask walls hung with still lifes, a long table receding with its diners,
/// swags and game birds close by, and the feast going off at the edge of the screen.
pub fn banquet(seed: u32, dominant_light: [u8; 3]) -> Vec<Layer> {
    let wall = tint([16, 8, 9], dominant_light, 18);
    let damask = tint([22, 11, 11], dominant_light, 24);
    let gilt = tint([34, 24, 12], dominant_light, 30);
    let shadow = tint([9, 5, 6], dominant_light, 8);
    let canvas = tint([12, 9, 7], dominant_light, 14);
    let fruit = tint([30, 14, 12], dominant_light, 40);
    let mut far = Canvas::new(wall);
    // Damask: a small diamond every 24 pixels, staggered by row.
    for row in 0..14 {
        let y = 40 + row * 20;
        let shift = if row % 2 == 0 { 12 } else { 0 };
        for x in (shift + 12..WIDTH as i32 - 12).step_by(24) {
            for d in 0..3 {
                far.rect(x - d, y - 3 + d, 1 + d * 2, 1, damask);
                far.rect(x - d, y + 3 - d, 1 + d * 2, 1, damask);
            }
        }
    }
    // A picture rail and a dark dado.
    far.rect(0, 30, WIDTH as i32, 3, gilt);
    far.rect(0, 33, WIDTH as i32, 2, shadow);
    far.rect(0, 320, WIDTH as i32, 80, shadow);
    far.rect(0, 318, WIDTH as i32, 2, gilt);
    // Still lifes in heavy frames: a dish, fruit heaped on it, something hanging over it.
    for bay in 0..4 {
        let n = noise(seed, 900 + bay as u32);
        let x = 120 + bay * 240;
        let (w, h) = (96 + (n % 3) as i32 * 12, 76 + (n / 3 % 3) as i32 * 10);
        let top = 70 + (n / 9 % 40) as i32;
        far.rect(x - w / 2 - 6, top - 6, w + 12, h + 12, gilt);
        far.rect(x - w / 2, top, w, h, canvas);
        let table = top + h - 18;
        far.rect(x - w / 2, table, w, 18, shadow);
        far.ellipse(x, table, w / 3, 5, damask);
        for i in 0..5 {
            let fx = x - 20 + i * 10;
            let fy = table - 6 - (i % 2) * 6;
            far.ellipse(fx, fy, 5, 5, fruit);
        }
        if n.is_multiple_of(2) {
            // A pheasant hung by its feet.
            far.line((x + w / 3, top + 4), (x + w / 3, top + 18), 1, shadow);
            far.ellipse(x + w / 3, top + 28, 5, 11, shadow);
        }
    }

    // The long table, receding, with diners and candelabra along it.
    let mut middle = Canvas::new([0; 4]);
    let dark = [10, 6, 7, 255];
    let cloth = tint([20, 14, 14], dominant_light, 20);
    let flame = tint([70, 44, 20], dominant_light, 90);
    middle.rect(0, 300, WIDTH as i32, 10, cloth);
    middle.rect(0, 310, WIDTH as i32, 50, dark);
    for i in 0..20 {
        let n = noise(seed, 700 + i as u32);
        let x = 24 + i * 48 + (n % 9) as i32;
        // A diner: shoulders and a head, some leaning in, some slumped.
        let slump = (n / 9 % 3) as i32 * 4;
        middle.ellipse(x, 288 + slump / 2, 14, 12, dark);
        middle.ellipse(x + slump / 2, 266 + slump, 7, 8, dark);
        if n.is_multiple_of(4) {
            // A raised glass.
            middle.rect(x + 10, 246 + slump, 2, 20, dark);
            middle.rect(x + 7, 242 + slump, 8, 5, dark);
        }
        if i % 4 == 2 {
            // A candelabrum between diners.
            let cx = x + 24;
            middle.rect(cx - 1, 248, 3, 52, dark);
            middle.rect(cx - 12, 250, 25, 2, dark);
            for c in [-12, 0, 12] {
                middle.rect(cx + c - 1, 238, 3, 12, dark);
                middle.pixel(cx + c, 236, flame);
                middle.pixel(cx + c, 235, flame);
            }
        } else if i % 4 == 0 {
            // A covered dish.
            middle.ellipse(x + 24, 298, 12, 9, dark);
            middle.rect(x + 23, 287, 3, 3, dark);
        }
    }

    // Swags of curtain from the top, and game hung from a rail.
    let mut near = Canvas::new([0; 4]);
    let drape = [8, 4, 6, 255];
    let fold = [14, 7, 9, 255];
    let rope = tint([30, 22, 12], dominant_light, 24);
    for x in [180, 660] {
        // A swag: a heavy curve between two tails.
        for dx in -110..=110 {
            let sag = 34 - (dx * dx) / 360;
            near.rect(x + dx, 0, 1, 20 + sag.max(0), drape);
        }
        for side in [-1, 1] {
            near.rect(x + side * 110 - 9, 0, 18, 220, drape);
            near.rect(x + side * 110 - 2, 10, 2, 200, fold);
        }
        near.rect(x - 2, 40, 4, 6, rope);
    }
    for x in [420, 900] {
        // A brace of birds hung by their feet from a hook.
        near.rect(x - 1, 0, 2, 90, rope);
        for (dx, len) in [(-8, 38), (0, 46), (8, 34)] {
            near.line((x, 90), (x + dx, 98), 1, rope);
            near.ellipse(x + dx, 98 + len / 2, 6, len / 2, drape);
            near.rect(x + dx - 1, 98 + len, 2, 10, drape);
        }
    }

    // Close, at the edge: the feast going off. A spilled goblet, fruit gone soft, a bone.
    let mut front = Canvas::new([0; 4]);
    let near_black = [4, 2, 3, 255];
    let soft = [9, 4, 6, 255];
    let (px, py) = (110, 368);
    front.ellipse(px, py + 20, 70, 26, near_black);
    for (dx, dy, r) in [
        (-40, -6, 12),
        (-18, -14, 14),
        (8, -10, 13),
        (30, -4, 11),
        (-4, -26, 10),
    ] {
        front.ellipse(px + dx, py + dy, r, r, soft);
        front.ellipse(px + dx - 2, py + dy - 2, r - 3, r - 3, near_black);
    }
    // The goblet on its side.
    front.rect(px + 40, py - 18, 26, 14, near_black);
    front.rect(px + 66, py - 14, 10, 6, near_black);
    front.rect(px + 76, py - 20, 3, 18, near_black);
    // A bone across the heap.
    front.line((px - 60, py - 30), (px - 20, py - 40), 4, soft);
    front.ellipse(px - 62, py - 30, 5, 5, soft);
    front.ellipse(px - 18, py - 41, 5, 5, soft);

    vec![
        far.layer("still lifes", 0.12, false),
        middle.layer("long table", 0.32, false),
        near.layer("swags", 0.62, false),
        front.layer("foreground", 1.18, true),
    ]
}

/// The garden under glass: the glasshouse's ironwork against a night sky, palms and trees in the
/// dark, vines hanging close with a few glowing buds, and great leaves at the edge of the screen.
pub fn garden(seed: u32, dominant_light: [u8; 3]) -> Vec<Layer> {
    let sky = tint([6, 10, 14], dominant_light, 14);
    let pane = tint([10, 16, 20], dominant_light, 20);
    let iron = tint([16, 22, 24], dominant_light, 24);
    let star = tint([60, 80, 90], dominant_light, 40);
    let moon = tint([90, 110, 110], dominant_light, 50);
    let mut far = Canvas::new(sky);
    // The glasshouse: a curved roof of panes on iron ribs, a bay every 120 pixels.
    for bay in 0..8 {
        let x = 60 + bay * 120;
        for dx in -56..=56 {
            let rise = 70 - (dx * dx) / 46;
            far.rect(x + dx, 40 + (70 - rise), 1, 300, pane);
        }
        far.rect(x - 1, 40, 3, 360, iron);
        for row in 0..6 {
            far.rect(x - 56, 110 + row * 40, 113, 1, iron);
        }
    }
    far.rect(0, 340, WIDTH as i32, 60, iron);
    // Stars through the glass, and the moon in one pane.
    for i in 0..60 {
        let n = noise(seed, 1100 + i);
        let (x, y) = (20 + (n % 920) as i32, 50 + (n / 920 % 250) as i32);
        far.pixel(x, y, star);
    }
    let mx = 200 + (noise(seed, 1200) % 560) as i32;
    far.ellipse(mx, 90, 14, 14, moon);
    far.ellipse(mx + 5, 86, 11, 11, pane);

    // Palms and trees in the dark, between beds.
    let mut middle = Canvas::new([0; 4]);
    let dark = [6, 10, 9, 255];
    let leaf = tint([10, 18, 14], dominant_light, 16);
    for i in 0..7 {
        let n = noise(seed, 1300 + i as u32);
        let x = 70 + i * 135 + (n % 30) as i32;
        if n.is_multiple_of(2) {
            // A palm: a leaning trunk and fronds falling from its crown.
            let lean = (n / 2 % 21) as i32 - 10;
            let top = 120 + (n / 50 % 40) as i32;
            middle.line((x, 350), (x + lean, top), 4, dark);
            for (dx, dy) in [
                (-50, 30),
                (-34, 50),
                (-12, 56),
                (12, 54),
                (36, 46),
                (52, 26),
            ] {
                for t in 0..=20 {
                    let px = x + lean + dx * t / 20;
                    let py = top + dy * t / 20 - 10 + (t * t) / 12;
                    let w = 5 - t / 6;
                    middle.rect(px - w / 2, py, w, 4, if t > 12 { leaf } else { dark });
                }
            }
        } else {
            // A tree in a tub, round and dense.
            middle.rect(x - 3, 250, 6, 100, dark);
            middle.ellipse(x, 226, 34, 36, dark);
            middle.ellipse(x - 22, 210, 22, 24, dark);
            middle.ellipse(x + 20, 200, 24, 26, dark);
            middle.ellipse(x - 26, 200, 10, 9, leaf);
            middle.ellipse(x + 14, 186, 12, 10, leaf);
            middle.rect(x - 16, 330, 32, 20, dark);
        }
    }
    middle.rect(0, 350, WIDTH as i32, 10, dark);

    // Vines hanging close, with buds that glow.
    let mut near = Canvas::new([0; 4]);
    let vine = [8, 14, 10, 255];
    let bud = tint([40, 90, 76], dominant_light, 30);
    for i in 0..9 {
        let n = noise(seed, 1400 + i as u32);
        let x = 50 + i * 104 + (n % 20) as i32;
        let len = 90 + (n / 20 % 140) as i32;
        for y in 0..len {
            let sway = ((y as f32 / 30.0).sin() * 4.0) as i32;
            near.rect(x + sway, y, 2, 1, vine);
            if y % 26 == 13 {
                near.rect(x + sway - 4, y, 4, 2, vine);
            }
        }
        let end = ((len as f32 / 30.0).sin() * 4.0) as i32;
        if n.is_multiple_of(3) {
            near.ellipse(x + end + 1, len + 2, 3, 3, bud);
        }
    }

    // Close, at the edge: great leaves out of focus.
    let mut front = Canvas::new([0; 4]);
    let near_black = [2, 4, 3, 255];
    let rib = [5, 9, 7, 255];
    for (cx, cy, rx, ry) in [(120, 380, 60, 30), (180, 350, 40, 22), (840, 390, 70, 26)] {
        front.ellipse(cx, cy, rx, ry, near_black);
        front.line((cx - rx + 8, cy + 4), (cx + rx - 8, cy - 4), 1, rib);
    }

    vec![
        far.layer("glasshouse", 0.12, false),
        middle.layer("palms", 0.32, false),
        near.layer("vines", 0.62, false),
        front.layer("foreground", 1.18, true),
    ]
}

/// Horizontal texture offset shared by the runtime and the preview, rounded to whole pixels.
pub fn scroll(camera_x: f32, parallax: f32) -> i32 {
    (camera_x * parallax).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn deterministic_seeded_and_tinted() {
        for name in SETS {
            let set = |seed, light| super::set(name, seed, light).unwrap();
            let a = set(DEFAULT_SEED, DEFAULT_LIGHT);
            assert_eq!(a, set(DEFAULT_SEED, DEFAULT_LIGHT));
            assert_ne!(a, set(DEFAULT_SEED + 1, DEFAULT_LIGHT), "{name}");
            assert_ne!(a, set(DEFAULT_SEED, [25, 65, 90]), "{name}");
            assert_eq!(a.iter().filter(|l| l.foreground).count(), 1);
            check_palette(a);
        }
        assert!(super::set("nowhere", DEFAULT_SEED, DEFAULT_LIGHT).is_none());
    }

    fn check_palette(layers: Vec<Layer>) {
        for layer in layers {
            let palette: HashSet<_> = layer.image.pixels().map(|p| p.0).collect();
            assert!(
                palette.len() <= 8,
                "{}: {} colours",
                layer.name,
                palette.len()
            );
            assert!(layer.image.pixels().all(|p| p[3] == 0 || p[3] == 255));
        }
    }

    #[test]
    fn horizontal_edges_match_for_every_layer_and_seed() {
        for (name, seed) in SETS
            .iter()
            .flat_map(|n| [0, 1, DEFAULT_SEED, u32::MAX].map(|s| (n, s)))
        {
            for layer in super::set(name, seed, DEFAULT_LIGHT).unwrap() {
                assert_eq!(layer.image.dimensions(), (WIDTH, HEIGHT));
                for y in 0..HEIGHT {
                    assert_eq!(
                        layer.image.get_pixel(0, y),
                        layer.image.get_pixel(WIDTH - 1, y),
                        "{} row {y}",
                        layer.name
                    );
                }
            }
        }
    }
}
