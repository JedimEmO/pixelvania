//! A tilted crescent, echo and sparks at the blade tip.
// The crescent, echo, trail, glint and spark recipe is adapted from Sprite Fusion's open-source slash
// (https://www.spritefusion.com/pixel-art-vfx/slash), MIT License, Copyright (c) 2026 Hugo Duprez.
use super::Options;
use crate::{
    color::{Color, WHITE},
    math::{PI, Rng, TAU, clamp, out_back, out_expo, round, smooth},
    painter::{Ink, Painter},
    sim::{Ground, Kind, Layer, Particles, sparkle},
};
#[derive(Clone)]
struct Slash {
    r: f64,
    w: f64,
    a0: f64,
    a1: f64,
    t0: f64,
    hold: f64,
    trail: f64,
    c: Vec<Color>,
}
impl Slash {
    fn point(&self, r: f64, a: f64) -> [f64; 2] {
        let (lx, ly) = (a.cos() * r, a.sin() * r * 0.6);
        let (c, s) = ((-0.38_f64).cos(), (-0.38_f64).sin());
        [6.0 + lx * c - ly * s, -26.0 + lx * s + ly * c]
    }
    fn crescent(&self, p: &mut Painter<'_>, ta: f64, l: f64, w: f64, age: f64) {
        if l.abs() < 0.02 || w < 0.3 {
            return;
        }
        let n = self.c.len();
        let dir = l.signum();
        let span = l.abs();
        let (c, s) = ((-0.38_f64).cos(), (-0.38_f64).sin());
        let b = self.r + 2.0;
        p.shade([6.0 - b, -26.0 - b, 6.0 + b, -26.0 + b], |x, y| {
            let (dx, dy) = (x - 6.0, y + 26.0);
            let (lx, ly) = (dx * c + dy * s, (-dx * s + dy * c) / 0.6);
            let r = lx.hypot(ly);
            if r > self.r + 0.5 || r < self.r - w - 1.0 {
                return None;
            }
            let mut u = (dir * (ly.atan2(lx) - ta)) % TAU;
            if u < 0.0 {
                u += TAU;
            }
            if u > span {
                return None;
            }
            let v = u / span;
            let w = w * (PI * v.powf(2.2)).sin().powf(0.6);
            if w < 0.35 {
                return None;
            }
            let q = (self.r - r) / w;
            if !(0.0..=1.0).contains(&q) {
                return None;
            }
            let ci = if q < 0.22 && age < 0.5 {
                0.0
            } else {
                round(q * (n - 1) as f64 * 0.75 + age * 2.4 - v * 0.8 + 0.4)
            };
            Some(Ink::new(
                self.c[n - 1 - ci.clamp(0.0, (n - 1) as f64) as usize],
                (v / 0.4).min(1.0),
            ))
        });
        let mut u = 0.0;
        while u <= span {
            let [x, y] = self.point(self.r - w * 0.4, ta + dir * u);
            p.glow(
                x,
                y,
                5.0 + w,
                self.c[n - 3],
                0.22 * (1.0 - age) * (u / span),
            );
            u += 0.3;
        }
    }
    fn draw(&self, p: &mut Painter<'_>, t: f64) {
        let u = t - self.t0;
        if u < 0.0 {
            return;
        }
        let span = self.a1 - self.a0;
        let head = out_expo(clamp(u / 0.06));
        let tail = smooth(0.06 + self.hold, 0.06 + self.hold + self.trail, u);
        let age = clamp(u / (0.06 + self.hold + self.trail));
        if tail < 1.0 {
            self.crescent(
                p,
                self.a0 + span * tail,
                span * (head - tail),
                self.w * (1.0 - tail * 0.55),
                age,
            );
        }
        let th = 0.7 * (1.0 - clamp((u - 0.06 - self.hold) / (self.trail + 0.2)));
        if tail > 0.02 && th > 0.0 {
            let mut k = 0.0;
            while k <= tail {
                let [x, y] = self.point(self.r - 0.5, self.a0 + span * k);
                p.dot(x, y, self.c[1], th * (0.4 + 0.6 * k));
                k += 0.6 / (span.abs() * self.r);
            }
        }
    }
}
#[derive(Clone)]
pub(super) struct SwordSlash {
    a: Slash,
    e: Slash,
    tip: [f64; 4],
    rng: Rng,
    ps: Particles,
    t: f64,
    landed: bool,
}
impl SwordSlash {
    pub fn new(seed: u32, o: &Options) -> Self {
        let a = Slash {
            r: o.radius,
            w: 12.0,
            a0: -2.55,
            a1: 0.95,
            t0: 0.0,
            hold: 0.06,
            trail: 0.24,
            c: o.palette.clone(),
        };
        let e = Slash {
            r: o.radius * (33.0 / 42.0),
            w: o.radius * (4.5 / 42.0),
            a0: a.a0 + (a.a1 - a.a0) * 0.1,
            a1: a.a1 - (a.a1 - a.a0) * (0.15 / 3.5),
            t0: 0.017,
            hold: 0.03,
            trail: 0.2,
            c: o.palette[..6].to_vec(),
        };
        let [x, y] = a.point(a.r - a.w * 0.3, a.a1);
        let [x2, y2] = a.point(a.r - a.w * 0.3, a.a1 + 0.05);
        let l = (x2 - x).hypot(y2 - y);
        Self {
            a,
            e,
            tip: [x, y, (x2 - x) / l, (y2 - y) / l],
            rng: Rng::new(seed),
            ps: Particles {
                sparks: o.sparks.clone(),
                ..Particles::default()
            },
            t: 0.0,
            landed: false,
        }
    }
    pub fn update(&mut self, dt: f64, g: &dyn Ground) {
        self.t += dt;
        let r = &mut self.rng;
        if !self.landed && self.t >= 0.06 {
            self.landed = true;
            let [x, y, dx, dy] = self.tip;
            for _ in 0..10 {
                let a = dy.atan2(dx) + r.range(-0.5, 0.5);
                let sp = r.range(150.0, 400.0);
                spawn!(
                    self.ps,
                    r,
                    Kind::Spark,
                    x,
                    y,
                    a.cos() * sp,
                    a.sin() * sp,
                    r.range(0.12, 0.35),
                    WHITE,
                    1.0,
                    0.0,
                    false
                );
            }
            for _ in 0..16 {
                let a = self.a.a0 + (self.a.a1 - self.a.a0) * r.range(0.3, 1.0);
                let [x, y] = self.a.point(self.a.r, a);
                let (ox, oy) = (x - 6.0, y + 26.0);
                let l = ox.hypot(oy);
                let sp = r.range(30.0, 100.0);
                spawn!(
                    self.ps,
                    r,
                    Kind::Mote,
                    x,
                    y,
                    ox / l * sp,
                    oy / l * sp - 10.0,
                    r.range(0.25, 0.55),
                    self.a.c[4 + r.int(0, 1) as usize],
                    0.6,
                    0.0,
                    false
                );
            }
        }
        self.ps.update(dt, r, g);
    }
    pub fn draw(&self, p: &mut Painter<'_>) {
        self.e.draw(p, self.t);
        self.a.draw(p, self.t);
        let g = self.t - 0.06 * 0.6;
        if g > 0.0 && g < 0.22 {
            sparkle(
                p,
                self.tip[0],
                self.tip[1],
                9.0 * out_back((g / 0.05).min(1.0), 1.70158) * (1.0 - g / 0.22),
                self.a.c[5],
                self.a.c[3],
                0.6,
            );
        }
        if self.t < 0.3 {
            let [x, y] = self.a.point(
                self.a.r - 4.0,
                self.a.a0 + (self.a.a1 - self.a.a0) * out_expo(clamp(self.t / 0.06)),
            );
            p.glow(x, y, 40.0, self.a.c[3], 0.5 * (1.0 - self.t / 0.3));
        }
        self.ps.draw(p, Layer::All);
    }
}
