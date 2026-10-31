//! Pixel cells, dithered shapes, translucent discs and additive lights.
//! Ported from pixelfx core/painter.js; transforms operate on pixel centers.
use crate::{
    Frame,
    color::{CLEAR, Color},
    math::clamp,
};
const BAYER: [f64; 16] = [
    0.5, 8.5, 2.5, 10.5, 12.5, 4.5, 14.5, 6.5, 3.5, 11.5, 1.5, 9.5, 15.5, 7.5, 13.5, 5.5,
];
#[derive(Clone, Copy)]
pub struct Ink {
    pub color: Color,
    pub alpha: f64,
}
impl Ink {
    pub fn new(color: Color, alpha: f64) -> Self {
        Self { color, alpha }
    }
}
/// A view of a frame with local placement, mirroring, rotation and fade.
pub struct Painter<'a> {
    pub frame: &'a mut Frame,
    pub fade: f64,
    pub light_strength: f64,
    tx: f64,
    ty: f64,
    fx: f64,
    cs: f64,
    sn: f64,
}
impl<'a> Painter<'a> {
    pub fn new(frame: &'a mut Frame) -> Self {
        Self {
            frame,
            fade: 1.0,
            light_strength: 1.0,
            tx: 0.0,
            ty: 0.0,
            fx: 1.0,
            cs: 1.0,
            sn: 0.0,
        }
    }
    pub fn place(&mut self, x: f64, y: f64, flip: bool, angle: f64) {
        self.tx = x;
        self.ty = y;
        self.fx = if flip { -1.0 } else { 1.0 };
        self.cs = angle.cos();
        self.sn = angle.sin();
    }
    fn to(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.tx + x * self.fx * self.cs - y * self.sn,
            self.ty + x * self.fx * self.sn + y * self.cs,
        )
    }
    fn from(&self, x: f64, y: f64) -> (f64, f64) {
        let (dx, dy) = (x - self.tx, y - self.ty);
        (
            (dx * self.cs + dy * self.sn) * self.fx,
            -dx * self.sn + dy * self.cs,
        )
    }
    fn cell(&self, x: f64, y: f64) -> (i32, i32) {
        let (x, y) = self.to(x.floor() + 0.5, y.floor() + 0.5);
        (x.floor() as i32, y.floor() as i32)
    }
    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.frame.width as i32 || y >= self.frame.height as i32 {
            None
        } else {
            Some(y as usize * self.frame.width as usize + x as usize)
        }
    }
    pub fn put(&mut self, x: i32, y: i32, c: Color, a: f64) {
        let a = a * self.fade;
        if c == CLEAR || (a < 1.0 && a <= BAYER[(((y & 3) << 2) | (x & 3)) as usize] / 16.0) {
            return;
        }
        if let Some(i) = self.index(x, y) {
            self.frame.color[i] = c;
        }
    }
    fn add(&mut self, x: i32, y: i32, c: Color, k: f64) {
        let k = k * self.fade * self.light_strength;
        if k <= 0.004 {
            return;
        }
        if let Some(i) = self.index(x, y) {
            let d = &mut self.frame.light[i];
            for j in 0..3 {
                d[j] = (d[j] as f64 + c[j] as f64 * k).min(255.0) as u8;
            }
            d[3] = 255;
        }
    }
    fn blend_px(&mut self, x: i32, y: i32, c: Color, a: f64) {
        let a = a * self.fade;
        if c == CLEAR || a <= 0.004 {
            return;
        }
        if let Some(i) = self.index(x, y) {
            let d = &mut self.frame.color[i];
            let da = d[3] as f64 / 255.0;
            let oa = a + da * (1.0 - a);
            let k = a / oa;
            for j in 0..3 {
                d[j] = (c[j] as f64 * k + d[j] as f64 * (1.0 - k)) as u8;
            }
            d[3] = (oa * 255.0) as u8;
        }
    }
    pub fn px(&mut self, x: f64, y: f64, c: Color) {
        self.dot(x, y, c, 1.0);
    }
    pub fn dot(&mut self, x: f64, y: f64, c: Color, a: f64) {
        if a > 0.0 {
            let (x, y) = self.cell(x, y);
            self.put(x, y, c, a);
        }
    }
    pub fn blend(&mut self, x: f64, y: f64, c: Color, a: f64) {
        let (x, y) = self.cell(x, y);
        self.blend_px(x, y, c, a);
    }
    pub fn light(&mut self, x: f64, y: f64, c: Color, k: f64) {
        let (x, y) = self.cell(x, y);
        self.add(x, y, c, k);
    }
    fn each(&mut self, b: [f64; 4], mut f: impl FnMut(&mut Self, f64, f64, i32, i32)) {
        let (mut ax, mut ay, mut bx, mut by) = (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        );
        for i in 0..4 {
            let (x, y) = self.to(
                if i & 1 != 0 { b[2] } else { b[0] },
                if i & 2 != 0 { b[3] } else { b[1] },
            );
            ax = ax.min(x);
            ay = ay.min(y);
            bx = bx.max(x);
            by = by.max(y);
        }
        for sy in (ay.floor() as i32).max(0)..=(by.ceil() as i32).min(self.frame.height as i32 - 1)
        {
            for sx in
                (ax.floor() as i32).max(0)..=(bx.ceil() as i32).min(self.frame.width as i32 - 1)
            {
                let (x, y) = self.from(sx as f64 + 0.5, sy as f64 + 0.5);
                f(self, x, y, sx, sy);
            }
        }
    }
    pub fn shade(&mut self, b: [f64; 4], mut f: impl FnMut(f64, f64) -> Option<Ink>) {
        self.each(b, |p, x, y, sx, sy| {
            if let Some(ink) = f(x, y) {
                p.put(sx, sy, ink.color, ink.alpha);
            }
        });
    }
    pub fn disc(&mut self, x: f64, y: f64, r: f64, c: Color, a: f64) {
        if r < 0.5 {
            self.blend(x, y, c, a);
            return;
        }
        self.each([x - r, y - r, x + r, y + r], |p, lx, ly, sx, sy| {
            let d = (lx - x) * (lx - x) + (ly - y) * (ly - y);
            if d <= r * r {
                p.blend_px(sx, sy, c, a);
            }
        });
    }
    pub fn line_fn(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, mut f: impl FnMut(f64) -> Ink) {
        let (mut sx, mut sy) = self.cell(x0, y0);
        let (ex, ey) = self.cell(x1, y1);
        let (dx, dy) = ((ex - sx).abs(), -(ey - sy).abs());
        let stepx = if sx < ex { 1 } else { -1 };
        let stepy = if sy < ey { 1 } else { -1 };
        let n = dx.max(-dy).max(1) as f64;
        let (mut err, mut i) = (dx + dy, 0);
        loop {
            let ink = f(i as f64 / n);
            i += 1;
            self.put(sx, sy, ink.color, ink.alpha);
            if sx == ex && sy == ey {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                sx += stepx;
            }
            if e2 <= dx {
                err += dx;
                sy += stepy;
            }
        }
    }
    pub fn line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, c: Color, a: f64) {
        self.line_fn(x0, y0, x1, y1, |_| Ink::new(c, a));
    }
    pub fn circle(&mut self, x: f64, y: f64, r: f64, c: Color, a: f64) {
        if r < 0.5 {
            self.dot(x, y, c, a);
        } else {
            self.ellipse(x, y, r, r, |_| c, a);
        }
    }
    pub fn ellipse(
        &mut self,
        x: f64,
        y: f64,
        rx: f64,
        ry: f64,
        mut color: impl FnMut(f64) -> Color,
        a: f64,
    ) {
        if rx <= 0.0 || ry <= 0.0 {
            return;
        }
        self.shade([x - rx, y - ry, x + rx, y + ry], |lx, ly| {
            let dx = (lx - x) / rx;
            let dy = (ly - y) / ry;
            let d = dx * dx + dy * dy;
            if d > 1.0 {
                None
            } else {
                Some(Ink::new(color(d.sqrt()), a))
            }
        });
    }
    pub fn ring(&mut self, x: f64, y: f64, r: f64, w: f64, c: Color, a: f64) {
        let r0 = (r - w).max(0.0);
        self.shade([x - r, y - r, x + r, y + r], |lx, ly| {
            let d = (lx - x) * (lx - x) + (ly - y) * (ly - y);
            if d > r * r || d < r0 * r0 {
                None
            } else {
                Some(Ink::new(c, a))
            }
        });
    }
    pub fn glow(&mut self, x: f64, y: f64, r: f64, c: Color, k: f64) {
        if r < 1.0 || k <= 0.004 {
            return;
        }
        let (cx, cy) = self.to(x, y);
        for sy in (cy - r).floor() as i32..=(cy + r).ceil() as i32 {
            let dy = (sy as f64 + 0.5 - cy) / r;
            for sx in (cx - r).floor() as i32..=(cx + r).ceil() as i32 {
                let dx = (sx as f64 + 0.5 - cx) / r;
                let d = dx * dx + dy * dy;
                if d >= 1.0 {
                    continue;
                }
                let f = 1.0 - d.sqrt();
                self.add(sx, sy, c, f * f * k);
            }
        }
    }
    pub fn glow_line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, r: f64, c: Color, k: f64) {
        if r < 1.0 || k <= 0.004 {
            return;
        }
        let (ax, ay) = self.to(x0, y0);
        let (bx, by) = self.to(x1, y1);
        let (dx, dy) = (bx - ax, by - ay);
        let l2 = dx * dx + dy * dy;
        let l2 = if l2 == 0.0 { 1e-9 } else { l2 };
        for sy in (ay.min(by) - r).floor() as i32..=(ay.max(by) + r).ceil() as i32 {
            for sx in (ax.min(bx) - r).floor() as i32..=(ax.max(bx) + r).ceil() as i32 {
                let (px, py) = (sx as f64 + 0.5, sy as f64 + 0.5);
                let u = clamp(((px - ax) * dx + (py - ay) * dy) / l2);
                let (ex, ey) = (ax + dx * u - px, ay + dy * u - py);
                let d = (ex * ex + ey * ey).sqrt() / r;
                if d >= 1.0 {
                    continue;
                }
                let f = 1.0 - d;
                self.add(sx, sy, c, f * f * k);
            }
        }
    }
}
