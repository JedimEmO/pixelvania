//! Props: the things that say what a room is for, painted pixel by pixel.
//!
//! Each kind is painted from a few shapes into the same two images as the furniture:
//! a surface image, dark and flat, that the room's light falls on, and a glow image
//! for whatever shines by itself, such as flames, bulbs and champagne. The feet of a
//! piece are on the bottom row. Surfaces keep to dark, low-saturation tones with a
//! one-pixel lit edge on top, so the light in the room does the work.

use image::{Rgba, RgbaImage};

pub use crate::furniture::Piece;

/// Every prop kind, by its name in room files.
pub const KINDS: &[&str] = &[
    "coat_rack",
    "counter",
    "card_table",
    "chair",
    "champagne_tower",
    "bottle_rack",
    "barrel",
    "cask",
    "piano",
    "harp",
    "music_stand",
    "drum",
    "double_bass",
    "vanity",
    "mirror",
    "portrait",
    "curtain",
    "banquet_table",
    "cake",
    "sofa",
    "potted_palm",
    "statue",
    "urn",
    "fallen_glass",
    "shoes",
    "gloves_on_banister",
    "heap_of_coats",
    "ashtray_stand",
    "birdcage",
    "clock",
    "footlight",
    "balustrade",
    "candlestand",
    "screen",
    "crate",
    "roast",
    "fruit_pyramid",
    "game_rack",
    "range",
];

/// The light a prop casts.
pub struct PropLight {
    /// Offset in pixels from the bottom-centre of the image, x right, y up.
    pub x: f32,
    pub y: f32,
    /// sRGB.
    pub color: [u8; 3],
    pub radius: f32,
    pub strength: f32,
}

type C = [u8; 3];
/// A ramp from the lit edge down to the deepest shade.
type Ramp = [C; 4];

const WOOD: Ramp = [[84, 64, 48], [56, 42, 33], [40, 30, 25], [26, 20, 18]];
const DARK_WOOD: Ramp = [[64, 50, 42], [42, 33, 29], [30, 24, 22], [20, 16, 16]];
const VELVET: Ramp = [[128, 50, 82], [92, 34, 60], [66, 24, 44], [42, 15, 29]];
const GILT: Ramp = [[214, 176, 104], [150, 114, 58], [100, 74, 38], [62, 46, 26]];
const STONE: Ramp = [[104, 100, 110], [72, 69, 78], [52, 49, 58], [34, 32, 40]];
const LACQUER: Ramp = [[78, 72, 86], [36, 33, 42], [24, 22, 28], [14, 13, 17]];
const LINEN: Ramp = [[132, 126, 118], [96, 91, 86], [70, 66, 64], [48, 45, 46]];
const BRASS: Ramp = [[196, 160, 92], [138, 104, 56], [92, 68, 36], [58, 42, 24]];
const LEAF: Ramp = [[76, 108, 66], [46, 74, 48], [32, 54, 38], [22, 36, 28]];
const BOTTLE: Ramp = [[78, 104, 84], [38, 58, 46], [26, 40, 32], [16, 26, 22]];
const GLASS: C = [150, 142, 170];
const COATS: [Ramp; 5] = [
    [[70, 76, 112], [42, 46, 74], [30, 33, 54], [20, 22, 36]],
    [[112, 60, 86], [74, 36, 56], [52, 24, 40], [34, 16, 26]],
    [[64, 96, 78], [36, 60, 48], [26, 44, 36], [18, 30, 24]],
    [[86, 82, 92], [52, 50, 58], [36, 35, 42], [24, 23, 28]],
    [[132, 104, 76], [94, 72, 52], [66, 50, 38], [44, 34, 26]],
];

const FLAME: [C; 3] = [[255, 250, 220], [255, 214, 120], [255, 150, 60]];

/// Two images being painted, with the feet on the bottom row.
struct Canvas {
    color: RgbaImage,
    glow: RgbaImage,
}

impl Canvas {
    fn new(w: u32, h: u32) -> Canvas {
        Canvas {
            color: RgbaImage::new(w, h),
            glow: RgbaImage::new(w, h),
        }
    }

    fn w(&self) -> i32 {
        self.color.width() as i32
    }

    fn h(&self) -> i32 {
        self.color.height() as i32
    }

    fn inside(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w() && y < self.h()
    }

    fn px(&mut self, x: i32, y: i32, c: C) {
        if self.inside(x, y) {
            self.color
                .put_pixel(x as u32, y as u32, Rgba([c[0], c[1], c[2], 255]));
        }
    }

    /// Clears a surface pixel, for cutting shapes out.
    fn cut(&mut self, x: i32, y: i32) {
        if self.inside(x, y) {
            self.color.put_pixel(x as u32, y as u32, Rgba([0, 0, 0, 0]));
        }
    }

    fn gl(&mut self, x: i32, y: i32, c: C) {
        if self.inside(x, y) {
            self.glow
                .put_pixel(x as u32, y as u32, Rgba([c[0], c[1], c[2], 255]));
        }
    }

    /// A surface pixel that also shines.
    fn both(&mut self, x: i32, y: i32, c: C, g: C) {
        self.px(x, y, c);
        self.gl(x, y, g);
    }

    fn solid(&self, x: i32, y: i32) -> bool {
        self.inside(x, y) && self.color.get_pixel(x as u32, y as u32)[3] > 0
    }

    /// A filled rectangle, corners inclusive.
    fn rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: C) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.px(x, y, c);
            }
        }
    }

    /// A rectangle in a ramp: lit top row, mid body, a deep bottom row.
    fn block(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, r: Ramp) {
        self.rect(x0, y0, x1, y1, r[1]);
        self.rect(x0, y0, x1, y0, r[0]);
        if y1 > y0 + 1 {
            self.rect(x0, y1, x1, y1, r[3]);
        }
    }

    fn line(&mut self, a: (i32, i32), b: (i32, i32), c: C) {
        for (x, y) in line_points(a, b) {
            self.px(x, y, c);
        }
    }

    /// A filled ellipse, its colour picked by position within it (-1..1 each way).
    fn disc(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, shade: impl Fn(f32, f32) -> C) {
        let (x0, x1) = ((cx - rx).floor() as i32, (cx + rx).ceil() as i32);
        let (y0, y1) = ((cy - ry).floor() as i32, (cy + ry).ceil() as i32);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let dx = (x as f32 + 0.5 - cx) / rx;
                let dy = (y as f32 + 0.5 - cy) / ry;
                if dx * dx + dy * dy <= 1.0 {
                    self.px(x, y, shade(dx, dy));
                }
            }
        }
    }

    /// Lights the top edge of everything painted: every solid pixel with nothing above it
    /// takes the lit tone of its ramp, picked by `lit`.
    fn top_edge(&mut self, lit: impl Fn(C) -> Option<C>) {
        for y in 0..self.h() {
            for x in 0..self.w() {
                if self.solid(x, y) && !self.solid(x, y - 1) {
                    let p = self.color.get_pixel(x as u32, y as u32);
                    if let Some(c) = lit([p[0], p[1], p[2]]) {
                        self.px(x, y, c);
                    }
                }
            }
        }
    }

    /// A small candle flame, flickering on `step`, its wick at `(x, y)`.
    fn flame(&mut self, x: i32, y: i32, step: u32, seed: u32) {
        let tall = !(step + seed).is_multiple_of(3);
        self.gl(x, y, FLAME[0]);
        self.gl(x, y - 1, FLAME[1]);
        if tall {
            self.gl(x, y - 2, FLAME[2]);
        }
    }

    fn done(self) -> Piece {
        Piece {
            color: self.color,
            glow: self.glow,
        }
    }
}

fn line_points(a: (i32, i32), b: (i32, i32)) -> Vec<(i32, i32)> {
    let (dx, dy) = ((b.0 - a.0).abs(), -(b.1 - a.1).abs());
    let (sx, sy) = ((b.0 - a.0).signum(), (b.1 - a.1).signum());
    let (mut x, mut y, mut err) = (a.0, a.1, dx + dy);
    let mut out = Vec::new();
    loop {
        out.push((x, y));
        if (x, y) == b {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
    out
}

fn scale(c: C, k: f32) -> C {
    c.map(|v| (v as f32 * k).round().clamp(0.0, 255.0) as u8)
}

/// Paints a prop; `step` is the animation step (12 per second) for flames, smoke and glints.
/// `None` for an unknown kind.
pub fn paint(kind: &str, step: u32) -> Option<Piece> {
    Some(match kind {
        "coat_rack" => coat_rack(),
        "counter" => counter(),
        "card_table" => card_table(),
        "chair" => chair(),
        "champagne_tower" => champagne_tower(step),
        "bottle_rack" => bottle_rack(step),
        "barrel" => barrel(),
        "cask" => cask(),
        "piano" => piano(),
        "harp" => harp(),
        "music_stand" => music_stand(),
        "drum" => drum(),
        "double_bass" => double_bass(),
        "vanity" => vanity(step),
        "mirror" => mirror(),
        "portrait" => portrait(),
        "curtain" => curtain(),
        "banquet_table" => banquet_table(step),
        "cake" => cake(),
        "sofa" => sofa(),
        "potted_palm" => potted_palm(),
        "statue" => statue(),
        "urn" => urn(),
        "fallen_glass" => fallen_glass(),
        "shoes" => shoes(step),
        "gloves_on_banister" => gloves_on_banister(),
        "heap_of_coats" => heap_of_coats(),
        "ashtray_stand" => ashtray_stand(step),
        "birdcage" => birdcage(),
        "clock" => clock(),
        "footlight" => footlight(),
        "balustrade" => balustrade(),
        "candlestand" => candlestand(step),
        "screen" => screen(),
        "crate" => crate_of_bottles(),
        "roast" => roast(),
        "fruit_pyramid" => fruit_pyramid(),
        "game_rack" => game_rack(),
        "range" => range(step),
        _ => return None,
    })
}

/// Whether the glow image changes with `step`, so the game repaints it.
pub fn animated(kind: &str) -> bool {
    matches!(
        kind,
        "champagne_tower"
            | "bottle_rack"
            | "vanity"
            | "banquet_table"
            | "shoes"
            | "ashtray_stand"
            | "candlestand"
            | "range"
    )
}

/// The light a prop casts, if any.
pub fn light(kind: &str) -> Option<PropLight> {
    let l = |x: f32, y: f32, color: C, radius: f32, strength: f32| PropLight {
        x,
        y,
        color,
        radius,
        strength,
    };
    Some(match kind {
        "champagne_tower" => l(0.0, 18.0, [255, 200, 120], 64.0, 0.7),
        "vanity" => l(0.0, 30.0, [255, 196, 150], 70.0, 0.8),
        "banquet_table" => l(0.0, 22.0, [255, 176, 96], 90.0, 0.9),
        "candlestand" => l(0.0, 40.0, [255, 170, 90], 80.0, 0.9),
        "footlight" => l(0.0, 10.0, [255, 190, 120], 70.0, 1.0),
        "mirror" => l(0.0, 30.0, [200, 190, 255], 40.0, 0.4),
        "ashtray_stand" => l(0.0, 22.0, [255, 120, 60], 30.0, 0.4),
        "range" => l(0.0, 8.0, [255, 120, 50], 80.0, 0.9),
        _ => return None,
    })
}

/// A coat hanging from a hook at `(cx, top)`, `len` long.
fn coat(c: &mut Canvas, cx: i32, top: i32, len: i32, r: Ramp) {
    for y in top..top + len {
        let t = (y - top) as f32 / len as f32;
        let half = (2.0 + t * 3.5).round() as i32;
        for x in cx - half..=cx + half {
            let shade = if y == top + len - 1 {
                3
            } else if x == cx + half || x == cx {
                2
            } else {
                1
            };
            c.px(x, y, r[shade]);
        }
    }
    // Shoulders and a collar.
    for x in cx - 2..=cx + 2 {
        c.px(x, top, r[0]);
    }
    c.px(cx - 1, top + 1, r[0]);
    c.px(cx + 1, top + 1, r[2]);
    // A sleeve hanging down the near side.
    for y in top + 2..top + len - 4 {
        c.px(cx - 3 - (y - top) / 6, y, r[2]);
    }
}

fn coat_rack() -> Piece {
    let mut c = Canvas::new(22, 48);
    let mid = 11;
    // The pole, its finial, and the hooks.
    for y in 3..45 {
        c.px(mid, y, DARK_WOOD[if y % 9 == 0 { 0 } else { 1 }]);
        c.px(mid + 1, y, DARK_WOOD[2]);
    }
    c.rect(mid, 1, mid + 1, 2, GILT[1]);
    c.px(mid, 1, GILT[0]);
    for (dx, dy) in [(-1, 4), (-2, 5), (-3, 5), (2, 4), (3, 5), (4, 5)] {
        c.px(mid + dx, dy, GILT[1]);
    }
    // Coats on the hooks, and a hat on the top.
    coat(&mut c, mid - 4, 6, 26, COATS[0]);
    coat(&mut c, mid + 5, 6, 22, COATS[4]);
    coat(&mut c, mid + 1, 8, 28, COATS[1]);
    for x in mid - 3..=mid + 4 {
        c.px(x, 4, LACQUER[1]);
    }
    c.rect(mid - 1, 1, mid + 2, 3, LACQUER[1]);
    c.rect(mid - 1, 1, mid + 2, 1, LACQUER[0]);
    // Three feet.
    for (dx, dy) in [(-1, 0), (-3, 1), (-5, 2), (2, 0), (4, 1), (6, 2)] {
        c.px(mid + dx, 45 + dy, DARK_WOOD[1]);
    }
    c.rect(mid, 45, mid + 1, 47, DARK_WOOD[1]);
    c.done()
}

fn counter() -> Piece {
    let mut c = Canvas::new(48, 22);
    // The top, overhanging, lit along its edge.
    c.block(0, 8, 47, 9, WOOD);
    c.rect(0, 8, 47, 8, WOOD[0]);
    // The front: panels in a frame.
    c.rect(2, 10, 45, 21, WOOD[2]);
    for x0 in [4, 18, 32] {
        c.rect(x0, 12, x0 + 11, 18, WOOD[1]);
        c.rect(x0, 12, x0 + 11, 12, WOOD[3]);
        c.rect(x0 + 11, 12, x0 + 11, 18, WOOD[3]);
    }
    c.rect(2, 20, 45, 21, WOOD[3]);
    c.rect(2, 10, 45, 10, GILT[2]);
    // A brass bell on the counter.
    c.rect(38, 7, 42, 7, BRASS[2]);
    c.disc(40.5, 6.5, 2.2, 2.0, |dx, dy| {
        if dy < -0.2 && dx < 0.3 {
            BRASS[0]
        } else {
            BRASS[1]
        }
    });
    c.rect(38, 7, 42, 7, BRASS[2]);
    c.px(40, 3, BRASS[0]);
    // A stack of cloakroom tickets and a ledger.
    c.rect(8, 6, 13, 7, LINEN[1]);
    c.rect(8, 6, 13, 6, LINEN[0]);
    c.rect(16, 7, 19, 7, LINEN[2]);
    c.px(16, 6, LINEN[1]);
    c.done()
}

fn card_table() -> Piece {
    let mut c = Canvas::new(32, 18);
    let baize: Ramp = [[60, 96, 72], [34, 62, 46], [26, 46, 36], [18, 32, 26]];
    c.block(1, 8, 30, 9, baize);
    c.rect(0, 10, 31, 10, WOOD[0]);
    c.rect(1, 11, 30, 11, WOOD[2]);
    // A turned pedestal on splayed feet.
    c.rect(15, 12, 16, 15, DARK_WOOD[1]);
    c.px(15, 13, DARK_WOOD[0]);
    c.rect(10, 16, 21, 17, DARK_WOOD[2]);
    c.rect(10, 16, 21, 16, DARK_WOOD[1]);
    // Cards face up, a fanned hand, and stacks of chips.
    let card = [196, 186, 166];
    c.rect(4, 7, 6, 7, card);
    c.rect(8, 7, 10, 7, card);
    c.px(12, 6, card);
    c.px(13, 6, card);
    c.px(14, 5, card);
    c.px(13, 7, [150, 40, 50]);
    let chips: [C; 3] = [[150, 40, 50], [196, 186, 166], [40, 38, 46]];
    for (i, (x, n)) in [(19, 3), (21, 2), (24, 4), (26, 1)].into_iter().enumerate() {
        for k in 0..n {
            c.px(x, 7 - k, chips[(i + k as usize) % 3]);
            c.px(x + 1, 7 - k, scale(chips[(i + k as usize) % 3], 0.75));
        }
    }
    c.done()
}

fn chair() -> Piece {
    let mut c = Canvas::new(14, 24);
    // The back: two posts with a velvet panel, the top rail lit.
    c.rect(2, 1, 3, 23, DARK_WOOD[1]);
    c.rect(2, 0, 4, 0, DARK_WOOD[0]);
    c.rect(4, 3, 4, 12, VELVET[2]);
    c.rect(3, 3, 4, 3, VELVET[1]);
    // The seat with its cushion, and the front legs.
    c.rect(2, 12, 12, 13, VELVET[1]);
    c.rect(2, 12, 12, 12, VELVET[0]);
    c.rect(2, 14, 12, 14, DARK_WOOD[2]);
    c.rect(11, 15, 12, 23, DARK_WOOD[1]);
    c.px(11, 23, DARK_WOOD[0]);
    c.rect(4, 18, 11, 18, DARK_WOOD[3]);
    c.done()
}

fn champagne_tower(step: u32) -> Piece {
    let mut c = Canvas::new(28, 34);
    // A small round table under it, in a cloth.
    c.block(2, 25, 25, 26, LINEN);
    c.rect(4, 27, 23, 28, LINEN[2]);
    c.rect(13, 29, 14, 32, DARK_WOOD[1]);
    c.rect(9, 33, 18, 33, DARK_WOOD[2]);
    // Coupes stacked four, three, two, one; each bowl holds champagne that glows.
    let rows = [4, 3, 2, 1];
    for (r, &n) in rows.iter().enumerate() {
        let base = 24 - r as i32 * 6;
        let x0 = 14 - n * 3;
        for i in 0..n {
            let gx = x0 + i * 6 + 1;
            c.rect(gx + 1, base, gx + 3, base, GLASS);
            c.px(gx + 2, base - 1, GLASS);
            c.px(gx + 2, base - 2, GLASS);
            // The bowl: rim and wine.
            for x in gx..=gx + 4 {
                c.px(x, base - 3, scale(GLASS, 0.8));
                c.px(x, base - 4, GLASS);
            }
            let fizz = (step + (i + r as i32) as u32 * 5) % 7;
            for x in gx + 1..=gx + 3 {
                c.gl(x, base - 3, [150, 104, 40]);
            }
            c.gl(gx + 1 + (fizz as i32 % 3), base - 4, [255, 224, 150]);
            if fizz == 0 {
                c.gl(gx + 2, base - 5, [200, 170, 110]);
            }
        }
    }
    // Wine runs down from the top glass.
    c.gl(14, 5, [120, 84, 34]);
    c.done()
}

fn bottle_rack(step: u32) -> Piece {
    let mut c = Canvas::new(32, 34);
    // The frame.
    c.rect(0, 0, 31, 31, DARK_WOOD[2]);
    c.rect(0, 0, 31, 0, DARK_WOOD[0]);
    c.rect(1, 1, 30, 30, DARK_WOOD[3]);
    c.rect(1, 32, 3, 33, DARK_WOOD[2]);
    c.rect(28, 32, 30, 33, DARK_WOOD[2]);
    // Shelves of bottles lying with their bottoms out, a glint on some.
    let mut n = 0u32;
    for row in 0..4 {
        let y = 4 + row * 7;
        c.rect(1, y + 3, 30, y + 3, DARK_WOOD[1]);
        for col in 0..5 {
            let x = 4 + col * 6;
            n += 1;
            if (n * 7) % 11 == 3 {
                continue;
            }
            c.disc(x as f32 + 0.5, y as f32, 2.4, 2.4, |dx, dy| {
                if dx < -0.2 && dy < -0.2 {
                    BOTTLE[1]
                } else {
                    BOTTLE[2]
                }
            });
            c.px(x - 1, y - 1, BOTTLE[0]);
            if (n + step / 4).is_multiple_of(5) {
                c.gl(x - 1, y - 1, [120, 150, 110]);
            }
        }
    }
    c.done()
}

fn barrel() -> Piece {
    let mut c = Canvas::new(16, 20);
    for y in 0..20 {
        let t = y as f32 / 19.0;
        let half = 5.0 + 2.0 * (std::f32::consts::PI * t).sin();
        let (x0, x1) = ((8.0 - half).round() as i32, (7.0 + half).round() as i32);
        for x in x0..=x1 {
            let stave = if (x - x0) % 3 == 2 { 2 } else { 1 };
            c.px(x, y, WOOD[stave]);
        }
        c.px(x1, y, WOOD[3]);
    }
    for y in [3, 4, 15, 16] {
        for x in 0..16 {
            if c.solid(x, y) {
                c.px(
                    x,
                    y,
                    if y == 3 || y == 15 {
                        [70, 66, 70]
                    } else {
                        [44, 42, 46]
                    },
                );
            }
        }
    }
    c.rect(3, 0, 12, 0, WOOD[0]);
    c.done()
}

fn cask() -> Piece {
    let mut c = Canvas::new(34, 26);
    // The cask on its side: bulging staves, hoops near each end, and a brass tap.
    for x in 2..30 {
        let t = (x - 2) as f32 / 27.0;
        let half = 7.0 + 2.0 * (std::f32::consts::PI * t).sin();
        let (y0, y1) = ((10.0 - half).round() as i32, (9.0 + half).round() as i32);
        for y in y0..=y1 {
            let stave = if (y - y0) % 4 == 3 { 2 } else { 1 };
            c.px(x, y, WOOD[stave]);
        }
        c.px(x, y0, WOOD[0]);
        c.px(x, y1, WOOD[3]);
    }
    for x in [5, 6, 25, 26] {
        for y in 0..20 {
            if c.solid(x, y) {
                c.px(
                    x,
                    y,
                    if x == 5 || x == 25 {
                        [70, 66, 70]
                    } else {
                        [44, 42, 46]
                    },
                );
            }
        }
    }
    // The head at the right end.
    c.rect(30, 3, 31, 16, WOOD[2]);
    c.rect(30, 3, 31, 3, WOOD[0]);
    c.rect(32, 12, 33, 13, BRASS[1]);
    c.px(32, 12, BRASS[0]);
    c.px(33, 14, BRASS[2]);
    // The cradle: two trestles.
    for x0 in [6, 22] {
        c.line((x0, 25), (x0 + 3, 18), DARK_WOOD[1]);
        c.line((x0 + 6, 25), (x0 + 3, 18), DARK_WOOD[2]);
        c.rect(x0 - 1, 18, x0 + 7, 19, DARK_WOOD[1]);
        c.rect(x0 - 1, 18, x0 + 7, 18, DARK_WOOD[0]);
    }
    c.done()
}

fn piano() -> Piece {
    let mut c = Canvas::new(58, 38);
    // The case, its tail curving down at the right.
    for x in 4..56 {
        let tail = if x > 44 {
            ((x - 44) as f32 / 11.0).powi(2) * 4.0
        } else {
            0.0
        };
        let y0 = 15 + tail.round() as i32;
        for y in y0..=22 {
            c.px(x, y, LACQUER[if y == 22 { 3 } else { 1 }]);
        }
        c.px(x, y0, LACQUER[0]);
    }
    c.rect(4, 20, 55, 20, LACQUER[2]);
    // The keys at the left.
    c.rect(0, 15, 4, 16, LACQUER[1]);
    c.rect(0, 15, 3, 15, [190, 182, 168]);
    c.px(1, 15, [40, 38, 44]);
    c.px(3, 15, [40, 38, 44]);
    // The lid, propped open on its stick: a dark wedge with a lit edge.
    for y in 3..15 {
        let k = (14 - y) as f32 / 11.0;
        let x0 = (10.0 + k * 40.0).round() as i32;
        let x1 = (54.0 - k * 4.0).round() as i32;
        for x in x0..=x1 {
            c.px(x, y, LACQUER[if x == x0 { 0 } else { 2 }]);
        }
    }
    c.line((10, 14), (50, 3), LACQUER[0]);
    c.line((50, 3), (54, 14), LACQUER[1]);
    c.line((30, 14), (32, 9), GILT[2]);
    // The music desk.
    c.line((6, 14), (9, 9), LACQUER[1]);
    c.rect(7, 10, 8, 11, LINEN[1]);
    // Three legs with brass castors.
    for x in [7, 30, 50] {
        c.rect(x, 23, x + 1, 35, LACQUER[1]);
        c.px(x, 23, LACQUER[0]);
        c.rect(x - 1, 36, x + 2, 36, LACQUER[2]);
        c.px(x, 37, BRASS[1]);
        c.px(x + 1, 37, BRASS[2]);
    }
    // The lyre and pedals.
    c.rect(20, 23, 21, 32, LACQUER[2]);
    c.rect(19, 33, 22, 33, BRASS[2]);
    c.done()
}

fn harp() -> Piece {
    let mut c = Canvas::new(22, 46);
    // The pillar, with a crown.
    c.rect(3, 4, 4, 41, GILT[1]);
    c.rect(3, 4, 3, 41, GILT[0]);
    c.rect(2, 1, 5, 3, GILT[1]);
    c.rect(2, 1, 5, 1, GILT[0]);
    // The neck, curving from the crown to the top of the soundboard.
    let neck: Vec<(i32, i32)> = (0..=16)
        .map(|i| {
            let t = i as f32 / 16.0;
            let x = 4.0 + t * 15.0;
            let y = 5.0 + 6.0 * (t * std::f32::consts::PI).sin() * 0.6 + t * 4.0;
            (x.round() as i32, y.round() as i32)
        })
        .collect();
    for &(x, y) in &neck {
        c.px(x, y, GILT[1]);
        c.px(x, y - 1, GILT[0]);
    }
    // The soundboard, from the base up to the end of the neck.
    c.line((7, 42), (19, 10), WOOD[1]);
    c.line((8, 42), (20, 10), WOOD[2]);
    c.line((9, 42), (20, 12), WOOD[2]);
    // Strings from the neck to the soundboard.
    for &(x, y) in neck.iter().skip(2).step_by(2) {
        let bottom = 42 - ((x - 7) as f32 * 32.0 / 12.0).round() as i32;
        if bottom > y + 1 {
            c.line((x, y + 1), (x, bottom - 1), [104, 98, 92]);
        }
    }
    // The base.
    c.rect(1, 42, 10, 43, GILT[2]);
    c.rect(1, 42, 10, 42, GILT[1]);
    c.rect(2, 44, 9, 45, DARK_WOOD[2]);
    c.done()
}

fn music_stand() -> Piece {
    let mut c = Canvas::new(14, 30);
    // The desk, tilted back, with a page of music on it.
    c.rect(1, 2, 12, 8, [34, 33, 38]);
    c.rect(1, 2, 12, 2, [70, 66, 72]);
    c.rect(2, 3, 11, 7, LINEN[1]);
    c.rect(2, 3, 11, 3, LINEN[0]);
    for y in [4, 6] {
        for x in (3..11).step_by(2) {
            c.px(x, y, LINEN[3]);
        }
    }
    c.rect(1, 9, 12, 9, [44, 42, 48]);
    // The pole and three feet.
    c.rect(6, 10, 7, 26, [44, 42, 48]);
    c.px(6, 18, [70, 66, 72]);
    c.line((6, 26), (1, 29), [44, 42, 48]);
    c.line((7, 26), (12, 29), [44, 42, 48]);
    c.rect(6, 27, 7, 29, [34, 33, 38]);
    c.done()
}

fn drum() -> Piece {
    let mut c = Canvas::new(22, 22);
    let shell: Ramp = [[132, 52, 64], [92, 32, 44], [64, 22, 32], [40, 14, 20]];
    c.disc(11.0, 10.0, 10.0, 10.0, |dx, dy| {
        let d = (dx * dx + dy * dy).sqrt();
        if d > 0.86 {
            if dy < -0.3 { shell[0] } else { shell[1] }
        } else if d > 0.8 {
            shell[3]
        } else if d < 0.35 {
            [124, 116, 104]
        } else {
            [104, 98, 88]
        }
    });
    // A painted ring on the head, and tension rods round the rim.
    c.disc(11.0, 10.0, 4.2, 4.2, |_, _| [86, 80, 72]);
    c.disc(11.0, 10.0, 3.2, 3.2, |_, _| [116, 108, 98]);
    for i in 0..10 {
        let a = i as f32 / 10.0 * std::f32::consts::TAU;
        let (x, y) = (11.0 + a.cos() * 9.0, 10.0 + a.sin() * 9.0);
        c.px(x.floor() as i32, y.floor() as i32, BRASS[1]);
    }
    // Its stand.
    c.line((5, 18), (3, 21), [44, 42, 48]);
    c.line((16, 18), (18, 21), [44, 42, 48]);
    c.done()
}

fn double_bass() -> Piece {
    let mut c = Canvas::new(26, 52);
    let bass: Ramp = [[126, 82, 52], [88, 54, 34], [62, 38, 24], [40, 24, 16]];
    // Painted upright, then leaned: each row slides right as it rises.
    let lean = |y: i32| ((51 - y) as f32 * 0.22).round() as i32;
    for y in 22..48 {
        let t = (y - 22) as f32 / 25.0;
        // Upper bout, waist, lower bout.
        let half = if t < 0.35 {
            4.0 + 3.0 * (t / 0.35 * std::f32::consts::FRAC_PI_2).sin()
        } else if t < 0.5 {
            6.0
        } else {
            6.0 + 3.0 * ((t - 0.5) / 0.5 * std::f32::consts::PI).sin().max(0.0)
        };
        let half = half.round() as i32;
        let cx = 8 + lean(y);
        for x in cx - half..=cx + half {
            let shade = if x == cx - half || y == 22 {
                0
            } else if x == cx + half || y == 47 {
                3
            } else {
                1
            };
            c.px(x, y, bass[shade]);
        }
        // The f-holes.
        if (31..38).contains(&y) {
            c.px(cx - 3, y, bass[3]);
            c.px(cx + 3, y, bass[3]);
        }
    }
    // The bridge and tailpiece.
    for y in 36..46 {
        c.px(8 + lean(y), y, LACQUER[2]);
    }
    // The neck, the scroll, and the strings down the fingerboard.
    for y in 3..22 {
        let cx = 8 + lean(y);
        c.px(cx, y, LACQUER[1]);
        c.px(cx + 1, y, LACQUER[2]);
    }
    let top = 8 + lean(2);
    c.rect(top - 1, 0, top + 1, 2, bass[1]);
    c.px(top - 1, 0, bass[0]);
    c.px(top + 2, 1, bass[2]);
    // The endpin.
    c.px(8 + lean(48), 48, [70, 66, 72]);
    c.px(8 + lean(49), 49, [70, 66, 72]);
    c.px(8 + lean(50), 50, [70, 66, 72]);
    c.px(8 + lean(51), 51, [44, 42, 48]);
    c.done()
}

fn vanity(step: u32) -> Piece {
    let mut c = Canvas::new(38, 42);
    // The table: a top, a drawer, and slender legs.
    c.block(1, 24, 36, 25, WOOD);
    c.rect(3, 26, 34, 30, WOOD[2]);
    c.rect(13, 27, 24, 29, WOOD[1]);
    c.px(18, 28, BRASS[0]);
    c.rect(3, 31, 4, 41, WOOD[1]);
    c.rect(33, 31, 34, 41, WOOD[1]);
    c.rect(3, 31, 34, 31, WOOD[3]);
    // The mirror in its frame, dark glass with a sheen.
    c.rect(7, 2, 30, 23, GILT[2]);
    c.rect(7, 2, 30, 2, GILT[1]);
    c.rect(9, 4, 28, 22, [30, 32, 44]);
    for i in 0..8 {
        c.px(12 + i, 20 - i * 2, [52, 56, 74]);
        c.px(13 + i, 20 - i * 2, [52, 56, 74]);
    }
    // Bulbs round the frame, lit.
    let mut bulbs = Vec::new();
    for x in (9..=28).step_by(4) {
        bulbs.push((x, 2));
    }
    for y in (6..=22).step_by(4) {
        bulbs.push((7, y));
        bulbs.push((30, y));
    }
    for (i, &(x, y)) in bulbs.iter().enumerate() {
        let dim = (i as u32 * 7 + step / 6).is_multiple_of(23);
        c.both(
            x,
            y,
            [220, 206, 180],
            if dim { [150, 110, 70] } else { [255, 226, 170] },
        );
    }
    // Scent bottles and a powder puff.
    c.rect(11, 21, 12, 23, [140, 100, 120]);
    c.px(11, 20, GILT[0]);
    c.rect(15, 22, 17, 23, [120, 130, 150]);
    c.rect(24, 22, 27, 23, [196, 180, 176]);
    c.done()
}

fn mirror() -> Piece {
    let mut c = Canvas::new(20, 54);
    // A tall glass in a gilt frame, round at the top, on a stand.
    for y in 1..46 {
        for x in 2..18 {
            let dy = y as f32 - 9.0;
            let dx = x as f32 - 9.5;
            if y < 9 && dx * dx + dy * dy > 64.0 {
                continue;
            }
            c.px(x, y, GILT[1]);
        }
    }
    for y in 3..44 {
        for x in 4..16 {
            let dy = y as f32 - 9.5;
            let dx = x as f32 - 9.5;
            if y < 9 && dx * dx + dy * dy > 36.0 {
                continue;
            }
            c.px(x, y, [28, 30, 42]);
        }
    }
    c.top_edge(|p| (p == GILT[1]).then_some(GILT[0]));
    // A faint sheen across the glass, and a cold glint.
    for i in 0..12 {
        let (x, y) = (5 + i, 38 - i * 3);
        if c.solid(x, y) && c.color.get_pixel(x as u32, y as u32)[0] == 28 {
            c.both(x, y, [56, 60, 80], [26, 28, 44]);
        }
    }
    c.gl(6, 8, [90, 90, 130]);
    // The stand.
    c.rect(9, 46, 10, 50, GILT[2]);
    c.rect(4, 51, 15, 51, GILT[2]);
    c.rect(3, 52, 5, 53, GILT[3]);
    c.rect(14, 52, 16, 53, GILT[3]);
    c.done()
}

fn portrait() -> Piece {
    let mut c = Canvas::new(26, 32);
    c.rect(0, 0, 25, 31, GILT[1]);
    c.rect(1, 1, 24, 30, GILT[2]);
    c.rect(0, 0, 25, 0, GILT[0]);
    c.rect(0, 31, 25, 31, GILT[3]);
    // Corner ornaments.
    for (x, y) in [(1, 1), (24, 1), (1, 30), (24, 30)] {
        c.px(x, y, GILT[0]);
    }
    c.rect(3, 3, 22, 28, [30, 24, 22]);
    // A sitter in the gloom: dark shoulders with a pale collar, a face framed by dark hair.
    c.disc(12.5, 27.5, 8.5, 6.0, |dx, _| {
        if dx < -0.5 {
            [56, 42, 54]
        } else {
            [42, 32, 42]
        }
    });
    c.line((9, 22), (12, 24), [120, 104, 92]);
    c.line((16, 22), (13, 24), [120, 104, 92]);
    c.rect(11, 17, 14, 21, [92, 76, 68]);
    c.disc(12.5, 12.0, 5.0, 5.6, |_, _| [40, 28, 26]);
    c.disc(12.6, 13.4, 3.3, 4.0, |dx, dy| {
        if dx < 0.1 && dy > -0.6 {
            [138, 114, 100]
        } else {
            [104, 86, 78]
        }
    });
    c.px(11, 13, [70, 56, 52]);
    c.px(14, 13, [70, 56, 52]);
    c.px(12, 16, [92, 60, 60]);
    c.px(13, 16, [92, 60, 60]);
    for y in 22..=28 {
        c.cut(3, y);
        c.px(3, y, [30, 24, 22]);
    }
    c.rect(3, 29, 22, 29, GILT[2]);
    c.done()
}

fn curtain() -> Piece {
    let mut c = Canvas::new(26, 64);
    // The drape, tied back to the left: full at the rod, pinched at the tie, pooled at the floor.
    for y in 2..64 {
        let (x0, x1) = if y < 40 {
            let t = (y - 2) as f32 / 38.0;
            (0, (23.0 - t * t * 15.0).round() as i32)
        } else {
            let t = (y - 40) as f32 / 23.0;
            (0, (8.0 + t * 7.0).round() as i32)
        };
        for x in x0..=x1 {
            let fold = (x + y / 12) % 5;
            let shade = match fold {
                0 => 0,
                1 | 2 => 1,
                3 => 2,
                _ => 3,
            };
            let shade = if x == x1 { 3 } else { shade.max(1) };
            c.px(x, y, VELVET[shade]);
        }
        // A highlight down the crest of each fold.
        for x in (x0..x1).filter(|x| (x + y / 12) % 5 == 0) {
            c.px(x, y, VELVET[0]);
        }
    }
    // The rod and its finial.
    c.rect(0, 0, 25, 1, GILT[1]);
    c.rect(0, 0, 25, 0, GILT[0]);
    c.rect(24, 0, 25, 2, GILT[0]);
    // The tie-back and its tassel.
    c.rect(0, 39, 9, 41, GILT[1]);
    c.rect(0, 39, 9, 39, GILT[0]);
    c.rect(9, 42, 10, 46, GILT[2]);
    c.px(9, 42, GILT[0]);
    c.done()
}

fn banquet_table(step: u32) -> Piece {
    let mut c = Canvas::new(64, 24);
    // The cloth, hanging in scallops, and the legs under it.
    c.rect(0, 10, 63, 10, LINEN[0]);
    for x in 0..64 {
        let drop = 17 + if (x % 8) < 4 { 1 } else { 0 };
        for y in 11..=drop {
            c.px(x, y, LINEN[if (x % 8) == 7 || y == drop { 2 } else { 1 }]);
        }
    }
    for x in [3, 31, 59] {
        c.rect(x, 19, x + 1, 23, DARK_WOOD[1]);
    }
    // Candlesticks with their flames.
    for (i, x) in [10, 32, 54].into_iter().enumerate() {
        c.rect(x - 1, 9, x + 1, 9, BRASS[1]);
        c.px(x, 8, BRASS[1]);
        c.rect(x, 3, x, 7, [226, 212, 186]);
        c.flame(x, 2, step, i as u32);
    }
    // Plates, a cloche, glasses and a tipped bottle.
    for x in [5, 17, 25, 40, 47] {
        c.rect(x, 9, x + 3, 9, [150, 146, 150]);
    }
    c.disc(21.0, 9.0, 3.5, 3.0, |_, dy| {
        if dy < -0.5 {
            [170, 168, 172]
        } else {
            [110, 108, 116]
        }
    });
    c.rect(17, 9, 25, 9, [150, 146, 150]);
    for x in [14, 37, 44, 58] {
        c.px(x, 9, GLASS);
        c.px(x, 8, GLASS);
        c.px(x - 1, 7, GLASS);
        c.px(x + 1, 7, GLASS);
        c.gl(x, 7, [120, 90, 40]);
    }
    c.line((27, 9), (30, 8), BOTTLE[1]);
    c.px(31, 8, GILT[0]);
    c.done()
}

fn cake() -> Piece {
    let mut c = Canvas::new(18, 22);
    let icing: Ramp = [[160, 148, 140], [124, 112, 106], [94, 84, 82], [66, 58, 58]];
    let sponge = [124, 92, 64];
    let jam = [138, 40, 60];
    // The stand.
    c.rect(1, 18, 16, 18, [150, 146, 150]);
    c.rect(1, 18, 16, 18, [170, 168, 172]);
    c.rect(8, 19, 9, 20, [110, 108, 116]);
    c.rect(5, 21, 12, 21, [110, 108, 116]);
    // Three tiers; the lower two half eaten from the right, their layers showing.
    for (y0, y1, x0, x1, eaten) in [(12, 17, 2, 15, 9), (7, 11, 4, 13, 10), (3, 6, 6, 11, 99)] {
        for y in y0..=y1 {
            for x in x0..=x1 {
                let cut = x >= eaten;
                let col = if cut {
                    if (y - y0) % 3 == 1 { jam } else { sponge }
                } else if y == y0 {
                    icing[0]
                } else if x == x1 || y == y1 {
                    icing[2]
                } else {
                    icing[1]
                };
                if cut && y == y0 && x > eaten + 2 {
                    continue;
                }
                c.px(x, y, col);
            }
        }
    }
    // Crumbs on the stand, and a cherry on top.
    c.px(15, 17, sponge);
    c.px(12, 17, sponge);
    c.px(8, 2, jam);
    c.px(8, 1, icing[2]);
    c.done()
}

/// A roast pig on a silver platter, an apple in its mouth, carved into from the back.
fn roast() -> Piece {
    let mut c = Canvas::new(34, 16);
    let crackling: Ramp = [[150, 92, 52], [112, 64, 36], [80, 44, 26], [52, 28, 18]];
    let meat = [150, 96, 90];
    // The platter.
    c.rect(1, 13, 32, 14, [150, 146, 150]);
    c.rect(0, 13, 33, 13, [180, 178, 182]);
    c.rect(3, 15, 30, 15, [96, 94, 102]);
    // The pig: a long body, the head forward, the back carved away.
    c.disc(16.0, 9.0, 11.0, 4.5, |dx, dy| {
        if dy < -0.55 {
            crackling[0]
        } else if dx < -0.55 && dy < 0.2 {
            meat
        } else {
            crackling[if dy > 0.5 { 2 } else { 1 }]
        }
    });
    c.disc(28.0, 9.5, 3.5, 3.0, |_, dy| {
        crackling[if dy < -0.4 { 0 } else { 1 }]
    });
    c.px(31, 9, crackling[2]);
    c.px(26, 7, crackling[3]);
    // The apple, and the legs tucked under.
    c.disc(31.5, 10.5, 1.6, 1.6, |_, _| [150, 36, 40]);
    for x in [9, 13, 20, 24] {
        c.px(x, 12, crackling[2]);
    }
    c.done()
}

/// Fruit heaped on a gilt stand; the far side of the heap has gone soft and brown.
fn fruit_pyramid() -> Piece {
    let mut c = Canvas::new(18, 26);
    // The stand.
    c.block(3, 17, 14, 18, GILT);
    c.rect(8, 19, 9, 23, GILT[1]);
    c.block(4, 24, 13, 25, GILT);
    // Rows of fruit, narrowing to one; the right of each row is spoiling.
    let fresh = [
        [176, 52, 58],
        [190, 140, 60],
        [120, 140, 60],
        [150, 60, 120],
    ];
    let rotten: C = [70, 54, 34];
    let mut i = 0;
    for (row, (x0, count)) in [(2, 5), (4, 4), (6, 3), (8, 2), (10, 1)]
        .into_iter()
        .enumerate()
    {
        let y = 14.5 - row as f32 * 3.0;
        for k in 0..count {
            let x = x0 as f32 - 0.5 + k as f32 * 3.0 + 1.5;
            let col = if k == count - 1 && count > 1 {
                rotten
            } else {
                fresh[i % 4]
            };
            i += 1;
            c.disc(x, y, 1.7, 1.7, |dx, dy| {
                if dx < -0.2 && dy < -0.2 {
                    scale(col, 1.25)
                } else {
                    col
                }
            });
        }
    }
    // A stalk, and a fly over the soft side.
    c.px(9, 1, [60, 80, 40]);
    c.px(15, 4, [30, 30, 34]);
    c.done()
}

/// A wooden rack with game birds hung from it by the feet.
fn game_rack() -> Piece {
    let mut c = Canvas::new(26, 36);
    // The frame.
    c.block(2, 0, 23, 1, WOOD);
    c.rect(2, 2, 3, 35, WOOD[1]);
    c.rect(22, 2, 23, 35, WOOD[1]);
    c.rect(1, 34, 4, 35, WOOD[2]);
    c.rect(21, 34, 24, 35, WOOD[2]);
    // Three birds, head down: feet tied at the bar, a fat body, a long neck, a drooping head.
    let feather: Ramp = [[118, 88, 58], [84, 60, 40], [58, 40, 28], [36, 26, 20]];
    for (x, len) in [(8, 14), (13, 18), (18, 12)] {
        c.line((x, 2), (x, 5), [40, 30, 24]);
        c.disc(x as f32 + 0.5, 9.0, 2.6, 4.2, |dx, dy| {
            feather[if dx > 0.3 {
                2
            } else if dy < -0.5 {
                0
            } else {
                1
            }]
        });
        c.line((x, 13), (x, 13 + len - 10), feather[2]);
        c.disc(x as f32 + 0.5, (13 + len - 9) as f32, 1.5, 1.5, |_, _| {
            feather[3]
        });
        // A tail feather sticking up past the tie.
        c.px(x + 1, 3, feather[0]);
    }
    c.done()
}

/// An iron range: an oven door, the firebox glowing, and pots on the hob.
fn range(step: u32) -> Piece {
    let mut c = Canvas::new(36, 26);
    let iron: Ramp = [[70, 66, 70], [42, 40, 44], [30, 28, 32], [18, 17, 20]];
    c.block(0, 10, 35, 25, iron);
    c.rect(0, 9, 35, 9, iron[0]);
    // The oven door with its handle.
    c.rect(3, 13, 16, 22, iron[2]);
    c.rect(4, 14, 15, 21, iron[1]);
    c.rect(6, 15, 13, 15, BRASS[1]);
    // The firebox: a grate with coals glowing behind it.
    c.rect(20, 14, 32, 22, iron[3]);
    for x in 21..=31 {
        for y in 16..=21 {
            let hot = (x + y + step as i32 / 3) % 4;
            let g = match hot {
                0 => [255, 150, 60],
                1 => [220, 90, 40],
                _ => [150, 50, 30],
            };
            c.both(x, y, scale(g, 0.6), g);
        }
    }
    for x in (21..=31).step_by(3) {
        c.rect(x, 15, x, 22, iron[3]);
    }
    // Pots on the hob, one with its lid rattling.
    c.block(4, 4, 13, 8, iron);
    c.rect(3, 4, 14, 4, iron[0]);
    c.block(
        19,
        2,
        30,
        8,
        [[124, 84, 60], [90, 58, 42], [66, 42, 32], [44, 28, 22]],
    );
    let lift = if (step / 2).is_multiple_of(4) { 1 } else { 0 };
    c.rect(18, 1 - lift, 31, 1 - lift, iron[0]);
    c.px(24, 0, iron[0]);
    c.done()
}

fn sofa() -> Piece {
    let mut c = Canvas::new(42, 20);
    let plum: Ramp = [[104, 70, 104], [70, 44, 72], [50, 30, 52], [32, 18, 34]];
    // The back, a long rolling curve.
    for x in 3..39 {
        let lift = (((x - 21) as f32 / 18.0).powi(2) * 3.0).round() as i32;
        for y in 1 + lift..10 {
            c.px(x, y, plum[if y == 1 + lift { 0 } else { 1 }]);
        }
    }
    // Buttons, seat, arms.
    for x in (7..36).step_by(6) {
        c.px(x, 6, plum[3]);
    }
    c.rect(3, 10, 38, 13, plum[1]);
    c.rect(3, 10, 38, 10, plum[0]);
    c.rect(3, 13, 38, 13, plum[3]);
    for x0 in [0, 37] {
        c.rect(x0, 6, x0 + 4, 13, plum[2]);
        c.rect(x0, 6, x0 + 4, 6, plum[0]);
        c.px(x0 + if x0 == 0 { 0 } else { 4 }, 7, GILT[1]);
    }
    c.rect(1, 14, 40, 15, GILT[2]);
    c.rect(1, 14, 40, 14, GILT[1]);
    for x in [2, 13, 28, 39] {
        c.rect(x, 16, x, 19, GILT[1]);
        c.px(x, 19, GILT[2]);
    }
    c.done()
}

fn potted_palm() -> Piece {
    let mut c = Canvas::new(28, 42);
    // Fronds arching out from the crown, leaflets hanging from each.
    let crown = (14.0f32, 16.0f32);
    for (k, (ex, ey, lift)) in [
        (-13.0, 10.0, -9.0),
        (-11.0, 2.0, -12.0),
        (-3.0, -14.0, -2.0),
        (5.0, -13.0, -3.0),
        (12.0, 1.0, -12.0),
        (13.0, 11.0, -8.0),
        (-6.0, 14.0, -4.0),
    ]
    .into_iter()
    .enumerate()
    {
        let mut last = None;
        for i in 0..=24 {
            let t = i as f32 / 24.0;
            let x = crown.0 + ex * t;
            let y = crown.1 + ey * t + lift * 4.0 * t * (1.0 - t);
            let q = (x.round() as i32, y.round() as i32);
            if last == Some(q) {
                continue;
            }
            last = Some(q);
            c.px(q.0, q.1, LEAF[1]);
            if i > 3 && i % 2 == (k % 2) {
                let side = if ex < 0.0 { -1 } else { 1 };
                c.px(q.0, q.1 + 1, LEAF[2]);
                c.px(q.0 - side, q.1 + 2, LEAF[3]);
                c.px(q.0 + side, q.1 + 1, LEAF[2]);
            }
        }
    }
    c.top_edge(|p| (p == LEAF[1]).then_some(LEAF[0]));
    // The trunk.
    c.rect(13, 16, 14, 28, WOOD[2]);
    for y in (17..28).step_by(2) {
        c.px(13, y, WOOD[1]);
    }
    // The pot: a fluted urn of dark terracotta.
    let pot: Ramp = [[120, 76, 56], [84, 52, 40], [62, 38, 30], [42, 26, 22]];
    c.rect(7, 28, 20, 29, pot[0]);
    for y in 30..39 {
        let half = 6 - (y - 30) / 3;
        for x in 14 - half..=13 + half {
            c.px(x, y, pot[if (x - 14 + half) % 3 == 2 { 2 } else { 1 }]);
        }
    }
    c.rect(10, 39, 17, 39, pot[2]);
    c.rect(8, 40, 19, 41, pot[3]);
    c.rect(8, 40, 19, 40, pot[1]);
    c.done()
}

fn statue() -> Piece {
    let mut c = Canvas::new(20, 52);
    // The plinth.
    c.block(1, 38, 18, 39, STONE);
    c.rect(3, 40, 16, 49, STONE[2]);
    c.rect(3, 40, 4, 49, STONE[1]);
    c.block(1, 50, 18, 51, STONE);
    // A draped figure, one arm raised with a torch long gone out.
    c.disc(9.5, 4.5, 2.4, 2.8, |dx, _| {
        if dx < 0.2 { STONE[0] } else { STONE[1] }
    });
    c.rect(9, 7, 10, 8, STONE[1]);
    for y in 9..38 {
        let half = 3 + (y - 9) / 7;
        for x in 10 - half..=9 + half {
            let fold = (x + (y - 9) / 4) % 3;
            let shade = if x == 9 + half {
                3
            } else if fold == 0 {
                0
            } else {
                1
            };
            c.px(x, y, STONE[shade.min(2)]);
        }
    }
    c.line((13, 10), (15, 4), STONE[1]);
    c.line((14, 10), (16, 4), STONE[2]);
    c.rect(15, 1, 16, 3, STONE[2]);
    c.px(15, 1, STONE[0]);
    c.line((6, 11), (5, 20), STONE[1]);
    c.top_edge(|p| (p == STONE[1] || p == STONE[2]).then_some(STONE[0]));
    c.done()
}

fn urn() -> Piece {
    let mut c = Canvas::new(18, 24);
    // Lip, neck, a full bowl, a stem and a square foot.
    c.rect(4, 0, 13, 1, STONE[1]);
    c.rect(4, 0, 13, 0, STONE[0]);
    c.rect(6, 2, 11, 4, STONE[2]);
    c.disc(9.0, 11.0, 7.0, 6.5, |dx, dy| {
        if dx < -0.4 && dy < 0.2 {
            STONE[0]
        } else if dx > 0.6 || dy > 0.7 {
            STONE[2]
        } else {
            STONE[1]
        }
    });
    c.rect(2, 9, 15, 9, STONE[3]);
    c.rect(7, 18, 10, 19, STONE[2]);
    c.rect(5, 20, 12, 21, STONE[1]);
    c.rect(4, 22, 13, 23, STONE[2]);
    c.rect(4, 22, 13, 22, STONE[1]);
    // Handles.
    for (x, dir) in [(1, -1), (16, 1)] {
        c.px(x, 6, STONE[1]);
        c.px(x, 7, STONE[1]);
        c.px(x + dir, 8, STONE[2]);
        c.px(x - dir, 5, STONE[1]);
    }
    c.done()
}

fn fallen_glass() -> Piece {
    let mut c = Canvas::new(14, 8);
    // A coupe on its side, its stem in the air, and a spill that still catches the light.
    c.line((2, 6), (7, 4), GLASS);
    c.line((2, 7), (7, 7), scale(GLASS, 0.8));
    c.px(2, 5, GLASS);
    c.line((8, 5), (11, 4), GLASS);
    c.rect(11, 3, 11, 5, GLASS);
    for x in 0..9 {
        c.gl(
            x,
            7,
            if x % 3 == 1 {
                [200, 160, 80]
            } else {
                [110, 80, 34]
            },
        );
    }
    c.gl(4, 6, [255, 224, 150]);
    c.done()
}

fn shoes(step: u32) -> Piece {
    let mut c = Canvas::new(16, 8);
    let satin: Ramp = [[176, 64, 96], [128, 40, 66], [92, 28, 48], [60, 18, 32]];
    // Two heeled dancing shoes, one tipped over, with buckles that glint.
    for (x0, tipped) in [(1, false), (8, true)] {
        if tipped {
            c.rect(x0, 5, x0 + 6, 6, satin[1]);
            c.rect(x0, 5, x0 + 6, 5, satin[0]);
            c.rect(x0 + 1, 7, x0 + 5, 7, satin[2]);
            c.rect(x0 + 6, 3, x0 + 6, 4, satin[2]);
            c.px(x0 + 2, 4, GILT[0]);
        } else {
            c.rect(x0, 4, x0 + 1, 4, satin[0]);
            c.rect(x0, 5, x0 + 1, 7, satin[1]);
            c.rect(x0 + 2, 6, x0 + 6, 7, satin[1]);
            c.rect(x0 + 2, 6, x0 + 6, 6, satin[0]);
            c.px(x0 + 6, 7, satin[2]);
            c.px(x0, 7, satin[3]);
            c.px(x0 + 4, 5, GILT[0]);
        }
    }
    let glint = if step % 18 < 2 {
        [255, 240, 200]
    } else {
        [120, 90, 50]
    };
    c.gl(5, 5, glint);
    c.gl(10, 4, [120, 90, 50]);
    c.done()
}

fn gloves_on_banister() -> Piece {
    let mut c = Canvas::new(24, 26);
    // The newel post with its cap, and the rail running off to the right over balusters.
    c.rect(2, 4, 5, 25, DARK_WOOD[1]);
    c.rect(2, 4, 2, 25, DARK_WOOD[0]);
    c.rect(1, 3, 6, 3, DARK_WOOD[0]);
    c.disc(3.5, 1.5, 2.0, 1.6, |dx, _| {
        if dx < 0.0 { DARK_WOOD[0] } else { DARK_WOOD[1] }
    });
    c.rect(6, 6, 23, 7, DARK_WOOD[1]);
    c.rect(6, 6, 23, 6, DARK_WOOD[0]);
    for x in [10, 15, 20] {
        c.rect(x, 8, x, 25, DARK_WOOD[2]);
        c.px(x - 1, 14, DARK_WOOD[2]);
        c.px(x + 1, 14, DARK_WOOD[2]);
    }
    c.rect(6, 24, 23, 25, DARK_WOOD[2]);
    // A pair of long silk gloves draped over the rail, fingers hanging.
    let silk: Ramp = [
        [214, 206, 196],
        [176, 168, 160],
        [134, 128, 124],
        [96, 92, 92],
    ];
    for (x0, len) in [(11, 13), (14, 10)] {
        c.rect(x0 - 1, 5, x0 + 2, 5, silk[0]);
        for y in 6..6 + len {
            c.px(x0, y, silk[1]);
            c.px(x0 + 1, y, silk[2]);
        }
        let tip = 6 + len;
        c.px(x0 - 1, tip, silk[2]);
        c.px(x0, tip, silk[1]);
        c.px(x0, tip + 1, silk[2]);
        c.px(x0 + 1, tip, silk[3]);
    }
    c.done()
}

fn heap_of_coats() -> Piece {
    let mut c = Canvas::new(34, 16);
    // Coats thrown on coats: overlapping mounds, a sleeve trailing, a hat on top.
    for (i, (cx, cy, rx, ry)) in [
        (8.0, 12.0, 8.0, 5.0),
        (24.0, 12.0, 9.0, 5.0),
        (16.0, 10.0, 9.0, 6.0),
        (11.0, 8.0, 6.0, 4.0),
        (21.0, 7.0, 6.0, 3.5),
    ]
    .into_iter()
    .enumerate()
    {
        let r = COATS[i % COATS.len()];
        c.disc(cx, cy, rx, ry, |dx, dy| {
            if dy < -0.6 {
                r[0]
            } else if dx > 0.6 || dy > 0.6 {
                r[2]
            } else {
                r[1]
            }
        });
    }
    for y in 4..16 {
        for x in 0..34 {
            if c.solid(x, y) && y > 13 {
                c.px(x, y, [24, 22, 28]);
            }
        }
    }
    c.line((26, 9), (32, 13), COATS[3][1]);
    c.line((26, 10), (32, 14), COATS[3][2]);
    c.rect(32, 14, 33, 15, [150, 140, 128]);
    c.rect(14, 3, 19, 4, LACQUER[1]);
    c.rect(15, 1, 18, 2, LACQUER[1]);
    c.rect(15, 1, 18, 1, LACQUER[0]);
    c.rect(15, 2, 18, 2, [92, 34, 46]);
    c.done()
}

fn ashtray_stand(step: u32) -> Piece {
    let mut c = Canvas::new(14, 34);
    // A brass bowl on a pole, on a weighted foot.
    c.rect(2, 18, 11, 19, BRASS[1]);
    c.rect(2, 18, 11, 18, BRASS[0]);
    c.rect(3, 20, 10, 20, BRASS[2]);
    c.rect(6, 21, 7, 31, BRASS[2]);
    c.px(6, 25, BRASS[1]);
    c.rect(3, 32, 10, 32, BRASS[1]);
    c.rect(2, 33, 11, 33, BRASS[2]);
    // A cigarette left burning, and its smoke curling up.
    c.rect(8, 17, 11, 17, [196, 190, 180]);
    c.gl(11, 17, [255, 120, 50]);
    for i in 0..16 {
        let y = 16 - i;
        let wave = ((i as f32 * 0.5) - step as f32 * 0.35).sin() * (1.0 + i as f32 * 0.12);
        let x = 11 + wave.round() as i32 - i / 6;
        let fade = 1.0 - i as f32 / 18.0;
        if i % 5 != 4 {
            c.gl(x, y, scale([70, 66, 78], fade));
        }
    }
    c.done()
}

fn birdcage() -> Piece {
    let mut c = Canvas::new(18, 38);
    // The dome of bars, rings, and the hook at the top.
    for y in 3..17 {
        let t = (y - 3) as f32 / 13.0;
        let half = if t < 0.4 {
            (7.0 * (t / 0.4 * std::f32::consts::FRAC_PI_2).sin()).max(1.0)
        } else {
            7.0
        };
        let half = half.round() as i32;
        for x in [9 - half, 8 + half] {
            c.px(x, y, GILT[1]);
        }
        for x in (9 - half..=8 + half).filter(|x| (x - 9).rem_euclid(3) == 0) {
            c.px(x, y, GILT[2]);
        }
    }
    c.px(9, 1, GILT[0]);
    c.px(8, 2, GILT[1]);
    c.px(9, 0, GILT[1]);
    for y in [8, 16] {
        c.rect(2, y, 15, y, GILT[1]);
    }
    c.rect(1, 17, 16, 17, GILT[2]);
    c.top_edge(|p| (p == GILT[1]).then_some(GILT[0]));
    // A small bird on its perch, quite still.
    c.rect(5, 13, 12, 13, DARK_WOOD[1]);
    c.disc(8.5, 11.0, 2.0, 1.6, |_, dy| {
        if dy < 0.0 { [86, 82, 72] } else { [60, 56, 50] }
    });
    c.px(10, 10, [86, 82, 72]);
    c.px(11, 10, BRASS[1]);
    c.px(6, 12, [60, 56, 50]);
    // The stand.
    c.rect(8, 18, 9, 34, DARK_WOOD[1]);
    c.px(8, 24, DARK_WOOD[0]);
    c.line((8, 34), (3, 37), DARK_WOOD[1]);
    c.line((9, 34), (14, 37), DARK_WOOD[1]);
    c.rect(8, 35, 9, 37, DARK_WOOD[2]);
    c.done()
}

fn clock() -> Piece {
    let mut c = Canvas::new(18, 50);
    // The hood with its pediment.
    c.rect(2, 3, 15, 18, WOOD[1]);
    c.rect(2, 3, 15, 3, WOOD[0]);
    for (i, y) in [0, 1, 2].into_iter().enumerate() {
        c.rect(
            4 + i as i32 * 2,
            y,
            13 - i as i32 * 2,
            y,
            WOOD[if y == 0 { 0 } else { 1 }],
        );
    }
    c.px(8, 0, GILT[0]);
    c.px(9, 0, GILT[1]);
    // The dial, its hands stopped at a minute to midnight.
    c.disc(9.0, 10.5, 5.0, 5.0, |dx, dy| {
        if dx * dx + dy * dy > 0.72 {
            GILT[1]
        } else {
            [170, 162, 146]
        }
    });
    for i in 0..12 {
        let a = i as f32 / 12.0 * std::f32::consts::TAU;
        let (x, y) = (9.0 + a.sin() * 3.2, 10.5 - a.cos() * 3.2);
        if i % 3 == 0 {
            c.px(x.floor() as i32, y.floor() as i32, [60, 54, 50]);
        }
    }
    c.line((9, 10), (9, 7), [30, 26, 26]);
    c.line((9, 10), (8, 8), [30, 26, 26]);
    // The waist with its window and a still pendulum.
    c.rect(4, 19, 13, 39, WOOD[2]);
    c.rect(4, 19, 4, 39, WOOD[1]);
    c.rect(6, 22, 11, 36, [26, 24, 30]);
    c.rect(9, 22, 9, 32, BRASS[2]);
    c.disc(9.0, 33.5, 2.0, 2.0, |dx, _| {
        if dx < 0.0 { BRASS[0] } else { BRASS[1] }
    });
    // The base.
    c.block(2, 40, 15, 41, WOOD);
    c.rect(3, 42, 14, 49, WOOD[2]);
    c.rect(5, 44, 12, 47, WOOD[1]);
    c.rect(3, 49, 14, 49, WOOD[3]);
    c.done()
}

fn footlight() -> Piece {
    let mut c = Canvas::new(12, 8);
    // A brass hood, open towards the stage, with the lamp shining up out of it.
    c.rect(1, 6, 10, 7, BRASS[2]);
    c.rect(1, 6, 10, 6, BRASS[1]);
    c.line((1, 5), (4, 2), BRASS[1]);
    c.line((2, 5), (4, 3), BRASS[2]);
    c.rect(4, 2, 4, 5, BRASS[2]);
    c.both(6, 4, [230, 220, 190], [255, 236, 190]);
    c.both(7, 4, [230, 220, 190], [255, 210, 140]);
    c.both(6, 5, [200, 190, 160], [255, 200, 120]);
    c.gl(7, 3, [200, 140, 70]);
    c.gl(8, 2, [120, 80, 40]);
    c.done()
}

fn balustrade() -> Piece {
    let mut c = Canvas::new(32, 14);
    c.block(0, 0, 31, 1, STONE);
    c.rect(0, 12, 31, 13, STONE[2]);
    c.rect(0, 12, 31, 12, STONE[1]);
    // Vase-shaped balusters.
    for x0 in (2..30).step_by(5) {
        for y in 2..12 {
            let w = match y {
                2 | 11 => 1,
                3 | 10 => 0,
                6..=8 => 1,
                _ => 0,
            };
            for x in x0 - w..=x0 + 1 + w {
                c.px(x, y, STONE[if x == x0 - w { 1 } else { 2 }]);
            }
        }
    }
    c.done()
}

fn candlestand(step: u32) -> Piece {
    let mut c = Canvas::new(18, 42);
    // Three candles on curling arms, a tall stem, and a tripod foot.
    c.rect(8, 10, 9, 37, BRASS[2]);
    c.rect(8, 10, 8, 37, BRASS[1]);
    for y in [16, 26] {
        c.rect(7, y, 10, y, BRASS[1]);
    }
    c.line((8, 12), (3, 9), BRASS[1]);
    c.line((9, 12), (14, 9), BRASS[1]);
    for (i, x) in [3, 9, 14].into_iter().enumerate() {
        let top = if i == 1 { 3 } else { 5 };
        let cup = top + 4;
        c.rect(x - 1, cup, x + 1, cup, BRASS[0]);
        c.rect(x, top + 1, x, cup - 1, [226, 212, 186]);
        c.flame(x, top, step, i as u32 * 2);
    }
    c.line((8, 38), (3, 41), BRASS[2]);
    c.line((9, 38), (14, 41), BRASS[2]);
    c.rect(8, 38, 9, 41, BRASS[2]);
    c.done()
}

fn screen() -> Piece {
    let mut c = Canvas::new(32, 38);
    let silk: [Ramp; 2] = [
        [[140, 104, 112], [100, 72, 82], [74, 52, 62], [50, 34, 42]],
        [[96, 72, 80], [66, 48, 56], [50, 36, 44], [34, 24, 30]],
    ];
    // Three panels folded in a zigzag, the middle one turned away from the light.
    for p in 0..3 {
        let x0 = 1 + p * 10;
        let r = silk[(p % 2) as usize];
        c.rect(x0, 2, x0 + 9, 35, DARK_WOOD[1]);
        c.rect(x0, 2, x0 + 9, 2, DARK_WOOD[0]);
        c.rect(x0 + 2, 4, x0 + 7, 33, r[1]);
        c.rect(x0 + 2, 4, x0 + 7, 4, r[0]);
        // A painted branch climbing across the silk.
        for i in 0..8 {
            let y = 30 - i * 3;
            let x = x0 + 3 + (i + p) % 3;
            c.px(x, y, r[3]);
            c.px(x + 1, y - 1, r[3]);
        }
        c.rect(x0 + 1, 36, x0 + 1, 37, DARK_WOOD[2]);
        c.rect(x0 + 8, 36, x0 + 8, 37, DARK_WOOD[2]);
    }
    // A stocking flung over the top.
    c.rect(19, 0, 22, 1, [60, 52, 58]);
    c.rect(22, 2, 23, 12, [60, 52, 58]);
    c.px(23, 13, [44, 38, 44]);
    c.done()
}

fn crate_of_bottles() -> Piece {
    let mut c = Canvas::new(22, 16);
    // Bottle necks with foil tops, standing in the crate.
    for (i, x) in [3, 7, 11, 15, 18].into_iter().enumerate() {
        let top = if i % 2 == 0 { 0 } else { 2 };
        c.rect(x, top + 2, x + 1, 6, BOTTLE[2]);
        c.px(x, top + 3, BOTTLE[0]);
        c.rect(x, top, x + 1, top + 1, GILT[if i % 2 == 0 { 0 } else { 1 }]);
    }
    // The crate: planks, a stencilled mark.
    c.rect(0, 6, 21, 15, WOOD[2]);
    c.rect(0, 6, 21, 6, WOOD[0]);
    for y in [10, 14] {
        c.rect(0, y, 21, y, WOOD[3]);
    }
    c.rect(0, 7, 21, 9, WOOD[1]);
    c.rect(0, 11, 21, 13, WOOD[1]);
    c.rect(0, 6, 0, 15, WOOD[3]);
    c.rect(21, 6, 21, 15, WOOD[3]);
    for x in [8, 10, 12] {
        c.px(x, 12, WOOD[3]);
    }
    c.px(9, 11, WOOD[3]);
    c.px(11, 11, WOOD[3]);
    c.done()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_paints_and_stands_on_its_bottom_row() {
        for &kind in KINDS {
            let p = paint(kind, 0).unwrap_or_else(|| panic!("{kind} does not paint"));
            let (w, h) = p.color.dimensions();
            assert_eq!(p.glow.dimensions(), (w, h), "{kind}");
            assert!(
                (8..=64).contains(&w) && (8..=64).contains(&h),
                "{kind}: {w}x{h}"
            );
            let feet = (0..w).any(|x| p.color.get_pixel(x, h - 1)[3] > 0);
            assert!(feet, "{kind} has nothing on its bottom row");
            if animated(kind) {
                let changes = (1..24).any(|s| paint(kind, s).unwrap().glow != p.glow);
                assert!(changes, "{kind} is animated but its glow never changes");
            } else {
                assert!(paint(kind, 5).unwrap().glow == p.glow, "{kind} changes");
            }
            if light(kind).is_some() || animated(kind) {
                assert!(p.glow.pixels().any(|g| g[3] > 0), "{kind} glows nowhere");
            }
        }
        assert!(paint("spaceship", 0).is_none());
        assert!(light("spaceship").is_none());
    }
}
