//! Furniture painted pixel by pixel: the lounge's chaise and its standing lamp.
//!
//! Each piece is a surface image, lit by the room like the tiles, and a glow image for
//! anything that shines by itself. Both are drawn with the feet of the piece on the bottom row.

use image::{Rgba, RgbaImage};

/// A painted piece of furniture.
pub struct Piece {
    pub color: RgbaImage,
    pub glow: RgbaImage,
}

const VELVET: [[u8; 3]; 4] = [[150, 56, 92], [112, 38, 70], [78, 26, 50], [48, 16, 32]];
const GILT: [[u8; 3]; 3] = [[214, 176, 104], [150, 114, 58], [92, 68, 34]];

fn put(img: &mut RgbaImage, x: i32, y: i32, c: [u8; 3]) {
    if x >= 0 && y >= 0 && (x as u32) < img.width() && (y as u32) < img.height() {
        img.put_pixel(x as u32, y as u32, Rgba([c[0], c[1], c[2], 255]));
    }
}

/// A velvet chaise longue, 36 by 18 pixels, with a scrolled back at its left end.
pub fn chaise() -> Piece {
    let (w, h) = (36, 18);
    let mut color = RgbaImage::new(w as u32, h as u32);
    // The seat: a long cushion, lit along its top edge.
    for x in 2..w - 2 {
        for y in 9..14 {
            let shade = match y {
                9 => 0,
                10 => 1,
                13 => 3,
                _ => 2,
            };
            put(&mut color, x, y, VELVET[shade]);
        }
    }
    // Buttons tufted into the cushion.
    for x in (6..w - 4).step_by(6) {
        put(&mut color, x, 11, VELVET[3]);
    }
    // The back rises from the left and curls over; the right end has a low scrolled arm.
    for y in 1..9 {
        let reach = (9 - y) / 2;
        for x in 2..6 + reach + (8 - y) {
            let shade = if x == 2 || y == 1 {
                0
            } else {
                1 + (y % 2) as usize
            };
            put(&mut color, x, y, VELVET[shade.min(3)]);
        }
    }
    for (dx, dy) in [(0, 0), (1, 0), (2, 0), (0, 1), (-1, 1), (-1, 2), (0, 3)] {
        put(&mut color, 1 + dx, 1 + dy, GILT[0]);
    }
    for y in 6..9 {
        for x in w - 5..w - 1 {
            put(&mut color, x, y, VELVET[if y == 6 { 0 } else { 1 }]);
        }
    }
    put(&mut color, w - 1, 6, GILT[0]);
    put(&mut color, w - 1, 7, GILT[1]);
    // A gilt rail under the seat, and four turned legs.
    for x in 1..w - 1 {
        put(&mut color, x, 14, GILT[1]);
    }
    for lx in [3, 12, w - 13, w - 4] {
        put(&mut color, lx, 15, GILT[0]);
        put(&mut color, lx, 16, GILT[1]);
        put(&mut color, lx, 17, GILT[2]);
    }
    let glow = RgbaImage::new(w as u32, h as u32);
    Piece { color, glow }
}

/// A standing lamp, 9 by 40 pixels: a gilt pole on a foot, a shade, and a flame inside it.
/// `step` picks the flame's flicker.
pub fn lamp(step: u32) -> Piece {
    let (w, h) = (9, 40);
    let mut color = RgbaImage::new(w as u32, h as u32);
    let mut glow = RgbaImage::new(w as u32, h as u32);
    // The foot and the pole.
    for x in 1..8 {
        put(&mut color, x, h - 1, GILT[2]);
    }
    for x in 2..7 {
        put(&mut color, x, h - 2, GILT[1]);
    }
    for y in 12..h - 2 {
        put(&mut color, 4, y, GILT[if y % 7 == 0 { 0 } else { 1 }]);
    }
    // The shade: a pleated cone, lit from inside, so it glows as well as taking light.
    for y in 2..12 {
        let half = 2 + (y - 2) / 3;
        for x in 4 - half..=4 + half {
            put(&mut color, x, y, [200, 150, 110]);
            let k = if (x - 4).abs() == half { 0.35 } else { 0.6 };
            let warm = [255.0 * k, 170.0 * k, 90.0 * k].map(|v| v as u8);
            put(&mut glow, x, y, warm);
        }
    }
    for x in 2..7 {
        put(&mut color, x, 1, GILT[0]);
    }
    // The flame shows through the bottom of the shade, taller on some steps.
    put(&mut glow, 4, 12, [255, 240, 200]);
    if !step.is_multiple_of(3) {
        put(&mut glow, 4, 11, [255, 220, 140]);
    }
    Piece { color, glow }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pieces_stand_on_their_bottom_row() {
        for piece in [chaise(), lamp(0)] {
            let bottom = piece.color.height() - 1;
            let feet = (0..piece.color.width())
                .filter(|&x| piece.color.get_pixel(x, bottom)[3] > 0)
                .count();
            assert!(feet > 0);
        }
        assert!(lamp(1).glow.pixels().any(|p| p[3] > 0));
    }
}
