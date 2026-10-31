//! Pixel-exact movement against a room's tiles.
//!
//! A [`Body`] sits on whole pixels and keeps the fraction it has not moved yet.
//! Each move is walked one pixel at a time, so a body never tunnels through a tile,
//! never ends up inside one, and always stops flush against what it hit.

use crate::room::{Room, TILE};

/// An axis-aligned box that moves through a room. Pixels, y down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Body {
    /// Top-left corner, in whole pixels.
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    /// Movement not yet applied, always within half a pixel.
    pub rem_x: f32,
    pub rem_y: f32,
}

impl Body {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            x,
            y,
            w,
            h,
            rem_x: 0.0,
            rem_y: 0.0,
        }
    }

    /// True when the box, shifted by `(dx, dy)`, overlaps a solid tile.
    pub fn collides_at(&self, room: &Room, dx: i32, dy: i32) -> bool {
        let x0 = (self.x + dx).div_euclid(TILE);
        let y0 = (self.y + dy).div_euclid(TILE);
        let x1 = (self.x + dx + self.w - 1).div_euclid(TILE);
        let y1 = (self.y + dy + self.h - 1).div_euclid(TILE);
        (y0..=y1).any(|ty| (x0..=x1).any(|tx| room.is_solid(tx, ty)))
    }

    /// Standing on something: one pixel down is blocked.
    pub fn on_ground(&self, room: &Room) -> bool {
        self.collides_at(room, 0, 1)
    }

    /// Moves horizontally by `amount` pixels. Returns true if a tile stopped the move.
    pub fn move_x(&mut self, room: &Room, amount: f32) -> bool {
        self.rem_x += amount;
        let steps = self.rem_x.round() as i32;
        self.rem_x -= steps as f32;
        let dir = steps.signum();
        for _ in 0..steps.abs() {
            if self.collides_at(room, dir, 0) {
                self.rem_x = 0.0;
                return true;
            }
            self.x += dir;
        }
        false
    }

    /// Moves vertically by `amount` pixels, positive is down. Returns true if a tile stopped the move.
    pub fn move_y(&mut self, room: &Room, amount: f32) -> bool {
        self.rem_y += amount;
        let steps = self.rem_y.round() as i32;
        self.rem_y -= steps as f32;
        let dir = steps.signum();
        for _ in 0..steps.abs() {
            if self.collides_at(room, 0, dir) {
                self.rem_y = 0.0;
                return true;
            }
            self.y += dir;
        }
        false
    }

    /// Exact position including the unapplied fraction, for smooth rendering.
    pub fn exact_pos(&self) -> (f32, f32) {
        (self.x as f32 + self.rem_x, self.y as f32 + self.rem_y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room() -> Room {
        // 8 x 5 tiles, walls all round, a floor at row 4.
        Room::parse(
            "[legend]\n# stone\n[tiles]\n\
             ########\n\
             #......#\n\
             #......#\n\
             #......#\n\
             ########\n",
        )
        .unwrap()
    }

    #[test]
    fn falls_and_lands_flush_on_the_floor() {
        let room = room();
        let mut b = Body::new(32, 20, 12, 30);
        let mut landed = false;
        for _ in 0..200 {
            landed |= b.move_y(&room, 3.7);
        }
        assert!(landed);
        // Floor row starts at y = 64; the body's bottom pixel is one above it.
        assert_eq!(b.y + b.h, 64);
        assert!(b.on_ground(&room));
    }

    #[test]
    fn stops_flush_against_a_wall() {
        let room = room();
        let mut b = Body::new(32, 20, 12, 30);
        assert!(!b.move_x(&room, -10.0));
        assert!(b.move_x(&room, -100.0));
        assert_eq!(b.x, 16);
    }

    #[test]
    fn keeps_fractions_between_moves() {
        let room = room();
        let mut b = Body::new(32, 20, 12, 30);
        for _ in 0..4 {
            b.move_x(&room, 0.25);
        }
        assert_eq!(b.x, 33);
        assert!(b.rem_x.abs() < 1e-6);
    }

    #[test]
    fn never_tunnels_through_a_thin_wall() {
        let room = Room::parse("[legend]\n# stone\n[tiles]\n.....#.....\n").unwrap();
        let mut b = Body::new(0, 0, 8, 8);
        assert!(b.move_x(&room, 500.0));
        assert_eq!(b.x + b.w, 80);
    }
}
