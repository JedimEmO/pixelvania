//! Hanging lights: chandeliers and lanterns on their chains, painted pixel by pixel.
//!
//! A light is a set of pixels around its grab point, where the Lure's rope takes hold,
//! hung from a chain whose top is the pivot. [`paint`] draws it swung by an angle about
//! that pivot, into two images: the gilt and wax that the room's light falls on,
//! and the flames and crystal glints that shine by themselves, for the glow buffer.
//! Swaying repaints the points at the new angle rather than rotating a finished sprite,
//! so every pixel stays a whole pixel.

use image::{Rgba, RgbaImage};

/// Which hanging light.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Chandelier,
    Lantern,
}

/// Width of the painted images, with the pivot on the centre column.
pub const WIDTH: u32 = 72;
/// Pixels below the grab point the lowest part can reach.
const BELOW: i32 = 22;
/// Pixels from the top of the fixture's crown to its grab point.
pub const CROWN: i32 = 10;

/// A painted hanging light, both images the same size, with the pivot at `(WIDTH / 2, 0)`.
pub struct Painted {
    pub color: RgbaImage,
    pub glow: RgbaImage,
}

/// The height of the images for a chain of `chain` pixels.
pub fn height(chain: u32) -> u32 {
    chain + (CROWN + BELOW) as u32 + 2
}

const DEEP: [u8; 3] = [52, 38, 20];
const DARK: [u8; 3] = [92, 68, 34];
const MID: [u8; 3] = [150, 114, 58];
const HI: [u8; 3] = [214, 176, 104];
const WAX: [u8; 3] = [226, 212, 186];
const GLASS: [u8; 3] = [150, 140, 170];

/// What a point is made of, which decides the image and colour it lands in.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Ink {
    Metal([u8; 3]),
    Wax,
    /// Crystal: pale on the surface, with a glint in the glow on some steps.
    Crystal,
    /// A flame, by its height in the flame: 0 at the wick.
    Flame(u8),
    /// The lantern's glass, lit from inside.
    Pane,
}

type Points = Vec<(i32, i32, Ink)>;

/// Adds the pixels of a quadratic curve from `a` to `c`, bent towards `b`.
fn bezier(p: &mut Points, a: (f32, f32), b: (f32, f32), c: (f32, f32), ink: &dyn Fn(f32) -> Ink) {
    let mut last = None;
    for i in 0..=64 {
        let t = i as f32 / 64.0;
        let u = 1.0 - t;
        let x = u * u * a.0 + 2.0 * u * t * b.0 + t * t * c.0;
        let y = u * u * a.1 + 2.0 * u * t * b.1 + t * t * c.1;
        let q = (x.round() as i32, y.round() as i32);
        if last != Some(q) {
            p.push((q.0, q.1, ink(t)));
            last = Some(q);
        }
    }
}

/// The points of a fixture, relative to its grab point, y down.
fn points(kind: Kind) -> Vec<(i32, i32, Ink)> {
    let mut p = Vec::new();
    let candle = |p: &mut Points, x: i32, y: i32| {
        // A cup, a candle, and its flame.
        for dx in -1..=1 {
            p.push((x + dx, y, Ink::Metal(if dx == 0 { MID } else { HI })));
        }
        p.push((x, y + 1, Ink::Metal(DARK)));
        for dy in 1..=3 {
            p.push((x, y - dy, Ink::Wax));
        }
        for (i, dy) in (4..=6).enumerate() {
            p.push((x, y - dy, Ink::Flame(i as u8)));
        }
    };
    match kind {
        Kind::Chandelier => {
            // The crown and the stem, with a knob at the grab point.
            for (y, half) in [(-10, 1), (-9, 2), (-8, 3), (-7, 2)] {
                for x in -half..=half {
                    let ink = if y == -8 { DARK } else { HI };
                    p.push((x, y, Ink::Metal(ink)));
                }
            }
            for y in -6..=12 {
                p.push((-1, y, Ink::Metal(HI)));
                p.push((0, y, Ink::Metal(MID)));
                p.push((1, y, Ink::Metal(DARK)));
            }
            for y in -1..=1 {
                for x in -2..=2 {
                    let ink = if x < 0 { HI } else { MID };
                    p.push((x, y, Ink::Metal(ink)));
                }
            }
            // Two tiers of arms on each side, curling up to their candles.
            for side in [-1.0f32, 1.0] {
                let s = side;
                let shade = |t: f32| Ink::Metal(if t < 0.5 { MID } else { HI });
                bezier(&mut p, (s, 2.0), (s * 8.0, 7.0), (s * 11.0, -4.0), &shade);
                bezier(&mut p, (s, 7.0), (s * 17.0, 15.0), (s * 21.0, 2.0), &shade);
                // A short curl under each lower arm.
                bezier(
                    &mut p,
                    (s * 14.0, 11.0),
                    (s * 15.0, 15.0),
                    (s * 12.0, 15.0),
                    &|_| Ink::Metal(DARK),
                );
            }
            for side in [-1, 1] {
                candle(&mut p, side * 11, -5);
                candle(&mut p, side * 21, 1);
            }
            // Strings of crystal drops between the lower arms, and below the stem.
            for side in [-1.0f32, 1.0] {
                let mut last = None;
                for i in 0..=12 {
                    let t = i as f32 / 12.0;
                    let x = side * (21.0 - 19.0 * t);
                    let y = 3.0 + 9.0 * (std::f32::consts::PI * t * 0.5).sin();
                    let q = (x.round() as i32, y.round() as i32);
                    if i % 2 == 0 && last != Some(q) {
                        p.push((q.0, q.1, Ink::Crystal));
                        last = Some(q);
                    }
                }
                p.push(((side * 21.0) as i32, 4, Ink::Crystal));
                p.push(((side * 21.0) as i32, 6, Ink::Crystal));
            }
            for (y, half) in [(13, 2), (14, 1), (15, 1), (16, 0), (17, 0)] {
                for x in -half..=half {
                    p.push((x, y, Ink::Metal(if x < 0 { HI } else { DARK })));
                }
            }
            p.push((0, 19, Ink::Crystal));
            p.push((0, 21, Ink::Crystal));
        }
        Kind::Lantern => {
            // A small cage of gilt round a lit pane, with a cap and a finial.
            for (y, half) in [(-10, 0), (-9, 1), (-8, 3), (-7, 4)] {
                for x in -half..=half {
                    p.push((x, y, Ink::Metal(if y == -7 { DARK } else { HI })));
                }
            }
            for y in -6..=6 {
                for x in -3i32..=3 {
                    let edge = x.abs() == 3 || y == -6 || y == 6;
                    let bar = x == 0 && y % 4 == 0;
                    let ink = if edge || bar {
                        Ink::Metal(if x < 0 { HI } else { MID })
                    } else {
                        Ink::Pane
                    };
                    p.push((x, y, ink));
                }
            }
            for x in -4..=4 {
                p.push((x, 7, Ink::Metal(DARK)));
            }
            for (y, half) in [(8, 2), (9, 1), (10, 0)] {
                for x in -half..=half {
                    p.push((x, y, Ink::Metal(MID)));
                }
            }
            for dy in 0..3 {
                p.push((0, 3 - dy, Ink::Flame(dy as u8)));
            }
        }
    }
    p
}

/// A small integer hash, for flicker and glints that change every animation step.
fn hash(a: u32, b: u32) -> u32 {
    let mut h = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0xC2B2_AE3D);
    h ^ (h >> 13)
}

/// Paints a hanging light on a chain of `chain` pixels, swung by `angle` radians
/// (positive swings the fixture to the right), at animation step `step`, which picks
/// the flames' flicker and the crystals' glints. `flare` from 0 to 1 brightens every flame,
/// for a moment after the Lure takes hold.
pub fn paint(kind: Kind, chain: u32, angle: f32, step: u32, flare: f32) -> Painted {
    let h = height(chain);
    let mut color = RgbaImage::new(WIDTH, h);
    let mut glow = RgbaImage::new(WIDTH, h);
    let (sin, cos) = angle.sin_cos();
    let pivot = WIDTH as i32 / 2;
    // From fixture coordinates to image pixels: the grab point hangs `chain + CROWN` below the pivot.
    let hang = (chain as i32 + CROWN) as f32;
    // Every point of the chain and the fixture, by position in fixture coordinates.
    let mut at = std::collections::HashMap::new();
    for y in 0..chain as i32 {
        // Links alternating face-on and edge-on.
        let local = y - hang as i32;
        if (y / 2) % 2 == 0 {
            at.insert((-1, local), (0, Ink::Metal(DARK)));
            at.insert((1, local), (0, Ink::Metal(DEEP)));
        } else {
            at.insert((0, local), (0, Ink::Metal(MID)));
        }
    }
    for (i, (x, y, ink)) in points(kind).into_iter().enumerate() {
        at.insert((x, y), (i, ink));
    }
    let flare = flare.clamp(0.0, 1.0);
    // Each image pixel looks back to the point that swings onto it, so filled shapes stay filled.
    // Swinging right moves the bottom right: the hanging vector turns clockwise on screen.
    for py in 0..h {
        for px in 0..WIDTH {
            let (rx, ry) = ((px as i32 - pivot) as f32, py as f32);
            let lx = rx * cos - ry * sin;
            let ly = rx * sin + ry * cos;
            let (x, y) = (lx.round() as i32, (ly - hang).round() as i32);
            let Some(&(i, ink)) = at.get(&(x, y)) else {
                continue;
            };
            let surface = |c: [u8; 3]| Rgba([c[0], c[1], c[2], 255]);
            match ink {
                Ink::Metal(c) => color.put_pixel(px, py, surface(c)),
                Ink::Wax => color.put_pixel(px, py, surface(WAX)),
                Ink::Crystal => {
                    color.put_pixel(px, py, surface(GLASS));
                    if hash(i as u32, step).is_multiple_of(5) {
                        glow.put_pixel(px, py, Rgba([200, 190, 255, 255]));
                    }
                }
                Ink::Pane => {
                    color.put_pixel(px, py, surface([120, 84, 40]));
                    let k = 0.45 + 0.1 * (hash(x as u32, step) % 3) as f32 + 0.3 * flare;
                    glow.put_pixel(px, py, Rgba(scale([255, 170, 80], k)));
                }
                Ink::Flame(level) => {
                    // The tip flickers: some steps it is a pixel short, some a pixel tall.
                    let flick = hash(x as u32 ^ (y as u32).wrapping_mul(31), step) % 4;
                    let tall = flare > 0.3;
                    if level == 2 && flick == 0 && !tall {
                        continue;
                    }
                    let c = match level {
                        0 => [255, 250, 220],
                        1 => [255, 214, 120],
                        _ => [255, 150, 60],
                    };
                    glow.put_pixel(px, py, Rgba([c[0], c[1], c[2], 255]));
                    if level == 2 && (flick == 3 || tall) && py > 0 {
                        glow.put_pixel(px, py - 1, Rgba([255, 120, 50, 255]));
                    }
                }
            }
        }
    }
    Painted { color, glow }
}

fn scale(c: [u8; 3], k: f32) -> [u8; 4] {
    let s = |v: u8| (v as f32 * k).round().clamp(0.0, 255.0) as u8;
    [s(c[0]), s(c[1]), s(c[2]), 255]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hangs_straight_from_its_pivot_at_rest() {
        let p = paint(Kind::Chandelier, 30, 0.0, 0, 0.0);
        assert_eq!(p.color.height(), height(30));
        // The chain starts at the pivot, on the centre column.
        let top: Vec<u32> = (0..WIDTH)
            .filter(|&x| p.color.get_pixel(x, 0)[3] > 0)
            .collect();
        assert!(!top.is_empty() && top.iter().all(|&x| x.abs_diff(WIDTH / 2) <= 1));
        // At rest the fixture is mirror-symmetric in shape.
        for y in 0..p.color.height() {
            for x in 0..WIDTH / 2 - 2 {
                let l = p.color.get_pixel(WIDTH / 2 - 1 - x, y)[3] > 0;
                let r = p.color.get_pixel(WIDTH / 2 + 1 + x, y)[3] > 0;
                assert_eq!(l, r, "asymmetric at ({x}, {y})");
            }
        }
    }

    #[test]
    fn swinging_right_moves_the_fixture_right() {
        let centre = |angle: f32| {
            let p = paint(Kind::Chandelier, 40, angle, 0, 0.0);
            let (mut sum, mut n) = (0.0, 0.0);
            for (x, y, px) in p.color.enumerate_pixels() {
                if px[3] > 0 && y > 40 {
                    sum += x as f32;
                    n += 1.0;
                }
            }
            sum / n
        };
        assert!(centre(0.15) > centre(0.0) + 4.0);
        assert!(centre(-0.15) < centre(0.0) - 4.0);
    }

    #[test]
    fn flames_burn_in_the_glow_only() {
        for kind in [Kind::Chandelier, Kind::Lantern] {
            let p = paint(kind, 20, 0.0, 3, 0.0);
            let lit = p.glow.pixels().filter(|px| px[3] > 0).count();
            assert!(lit >= 3, "{kind:?} has {lit} glowing pixels");
        }
    }
}
