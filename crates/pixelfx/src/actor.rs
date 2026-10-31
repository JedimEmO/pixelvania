//! Stock actor sprites and masks, ported from pixelfx core/actor.js.
use crate::{
    color::{Color, hex},
    painter::Painter,
};
const HERO: &[&str] = &[
    "....kkkkk...",
    "..kkhhhhhk..",
    ".khhhhhhhhk.",
    ".khhhhhhhhhk",
    ".khhssssshhk",
    ".khsessesshk",
    ".khsessesshk",
    "..ksssssssk.",
    "...kkSSSkk..",
    "..kttttttk..",
    ".kttTttTttk.",
    ".ksttTTttsk.",
    ".kstttttttk.",
    "..kbbbbbbk..",
    "..kppkkppk..",
    "..kppk.kppk.",
    "..kppk.kppk.",
    "..koook.kook",
    "..kkkkk.kkkk",
];
const HERO2: &[&str] = &[
    "...........kkk..........",
    "..........kAABk.........",
    ".........kAAABJk........",
    ".........kABBJJk........",
    ".........kBBBJCDk.......",
    ".........kBCJDDDk.......",
    ".........kkJJJDk........",
    ".......kkkkABBJCkk......",
    "......kAABCJBCCCkDk.....",
    "......kABBCJCCCmDkDk....",
    "......kBBCDDJCDDDkDk....",
    "......kABCDJDDDDDkk.....",
    ".....kAABDkDDDDJkk......",
    ".....kABCDkDDJJGkk......",
    ".....kABCkJJJGGGkk......",
    "....kJJCDkFFFGGGk.......",
    "....kAJJDkJJJGGGk.......",
    "....kJCJJkABBJGGk.......",
    ".....kJCCkBBCDJJJk......",
    ".....kCCCJBBCDJJDk......",
    ".....kCCCDJCCDkJDDk.....",
    "......kCDDJBCDkDDDk.....",
    ".......kDJJJCDkDDDkk....",
    ".......kJBCJCDkDDDkk....",
    ".......kBDJBCDkCDDkk....",
    "........kJJBCDkCDDk.....",
    "........kBBBCDkCCDDk....",
    "........kBBBCDkCCDDk....",
    ".........kBBCDkJDDDk....",
    "........kBJJJJkCJJJk....",
    "........kBBCCDkCCDDk....",
    "........kBBCDDkCDDk.....",
    "........kBBCDkCCDDk.....",
    "........kBCCDkCCDDk.....",
    "........kBCDDkCDDDk.....",
    ".......kJBCJkkkDJkkkk...",
    ".......kAJJAAAAkCCCCCk..",
    "........kkkkkkkkkkkkk...",
];
fn hero_pal(ch: char) -> Color {
    match ch {
        'k' => hex(0x17121f),
        'h' => hex(0x7a4128),
        's' => hex(0xf3c49d),
        'S' => hex(0xc98b6c),
        'e' => hex(0x17121f),
        't' => hex(0x5f6f94),
        'T' => hex(0x46536f),
        'b' => hex(0x8a5a32),
        'p' => hex(0x38334a),
        'o' => hex(0x6a4030),
        'H' => hex(0xa35d34),
        'w' => hex(0xffffff),
        'm' => hex(0xa3564a),
        'g' => hex(0xe8c060),
        'L' => hex(0x7f90b6),
        'O' => hex(0x8f5a3f),
        'P' => hex(0x4b4560),
        'c' => hex(0xf53cd1),
        'C' => hex(0xff8fe6),
        'D' => hex(0xa31f8a),
        _ => [0; 4],
    }
}
fn dummy_pal(ch: char) -> Color {
    match ch {
        'k' => hex(0x0e0c14),
        'A' => hex(0xe4e6ec),
        'B' => hex(0xb6b9c3),
        'C' => hex(0x868995),
        'D' => hex(0x5c5e6a),
        'E' => hex(0x6e707c),
        'F' => hex(0x4b4d58),
        'G' => hex(0x34353f),
        'J' => hex(0x2a2b33),
        'm' => hex(0xf53cd1),
        _ => [0; 4],
    }
}
/// A stock actor at 12 × 19 or 24 × 38 pixels, with its feet at the origin.
#[derive(Clone, Debug)]
pub struct Actor {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<Color>,
    pub mask: Vec<bool>,
    pub origin: [f64; 2],
    pub chest: [f64; 2],
}
impl Actor {
    pub fn stock(large: bool) -> Self {
        let rows = if large { HERO2 } else { HERO };
        let pal = if large { dummy_pal } else { hero_pal };
        let width = rows[0].len();
        let height = rows.len();
        let pixels: Vec<_> = rows.iter().flat_map(|row| row.chars().map(pal)).collect();
        let mask = pixels.iter().map(|c| c[3] != 0).collect();
        Self {
            width,
            height,
            pixels,
            mask,
            origin: [-(width as f64 / 2.0).floor(), -(height as f64)],
            chest: [
                0.0,
                if large {
                    -26.0
                } else {
                    -(height as f64) + 11.0
                },
            ],
        }
    }
    pub fn draw(&self, p: &mut Painter<'_>, alpha: f64) {
        for (i, &c) in self.pixels.iter().enumerate() {
            if c[3] != 0 {
                p.dot(
                    self.origin[0] + (i % self.width) as f64,
                    self.origin[1] + (i / self.width) as f64,
                    c,
                    alpha,
                );
            }
        }
    }
}
