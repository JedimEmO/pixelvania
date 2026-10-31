//! Seeded ballroom silhouettes. Pixels carry their distant lighting, in sRGB.

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
        let a = ballroom(DEFAULT_SEED, DEFAULT_LIGHT);
        assert_eq!(a, ballroom(DEFAULT_SEED, DEFAULT_LIGHT));
        assert_ne!(a, ballroom(DEFAULT_SEED + 1, DEFAULT_LIGHT));
        assert_ne!(a, ballroom(DEFAULT_SEED, [25, 65, 90]));
        assert_eq!(a.iter().filter(|l| l.foreground).count(), 1);
        for layer in a {
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
        for seed in [0, 1, DEFAULT_SEED, u32::MAX] {
            for layer in ballroom(seed, DEFAULT_LIGHT) {
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
