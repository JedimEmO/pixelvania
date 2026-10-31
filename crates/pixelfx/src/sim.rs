//! Particles, tumbling debris and fading lights in art pixels and seconds.
// Adapted from pixelfx core/sim.js and Sprite Fusion's open-source pixel art VFX
// (https://www.spritefusion.com/pixel-art-vfx), MIT License, Copyright (c) 2026 Hugo Duprez.
use crate::{
    color::{self, Color, WHITE, hex, mix, scale},
    math::{Rng, TAU, round},
    painter::Painter,
};
/// Returns floor y at local x. Positive y points down; infinity means no floor.
pub trait Ground {
    fn height(&self, x: f64) -> f64;
}
impl<F: Fn(f64) -> f64> Ground for F {
    fn height(&self, x: f64) -> f64 {
        self(x)
    }
}
pub const GRAVITY: f64 = 950.0;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Spark,
    Mote,
    Dust,
    Star,
    Smoke,
    Grit,
}
#[derive(Clone, Debug)]
pub struct Particle {
    pub kind: Kind,
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub life: f64,
    pub max: f64,
    pub color: Color,
    pub size: f64,
    pub seed: f64,
    pub delay: f64,
    pub big: bool,
}
#[derive(Clone, Debug)]
pub struct Particles {
    pub list: Vec<Particle>,
    pub time: f64,
    pub sparks: Vec<Color>,
}
impl Default for Particles {
    fn default() -> Self {
        Self {
            list: vec![],
            time: 0.0,
            sparks: vec![hex(0xfff3a6), hex(0xffe26b), hex(0xff8a2a), hex(0xf2561d)],
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Back,
    Front,
    All,
}
impl Particles {
    pub fn spawn(
        &mut self,
        rng: &mut Rng,
        kind: Kind,
        x: f64,
        y: f64,
        vx: f64,
        vy: f64,
        life: f64,
        color: Color,
        size: f64,
        delay: f64,
        big: bool,
    ) {
        self.list.push(Particle {
            kind,
            x,
            y,
            vx,
            vy,
            life,
            max: life,
            color,
            size,
            seed: rng.sample() * 100.0,
            delay,
            big,
        });
    }
    pub fn update(&mut self, dt: f64, rng: &mut Rng, ground: &dyn Ground) {
        self.time += dt;
        for i in (0..self.list.len()).rev() {
            let p = &mut self.list[i];
            if p.delay > 0.0 {
                p.delay -= dt;
                continue;
            }
            p.life -= dt;
            if p.life <= 0.0 {
                self.list.remove(i);
                continue;
            }
            match p.kind {
                Kind::Grit | Kind::Spark => {
                    if p.kind == Kind::Grit {
                        p.vy += GRAVITY * dt;
                        p.vx -= p.vx * 0.8 * dt;
                    } else {
                        p.vy += GRAVITY * 0.45 * dt;
                        p.vx -= p.vx * 2.2 * dt;
                        p.vy -= p.vy * 2.2 * dt;
                    }
                    let g = ground.height(p.x);
                    let ny = p.y + p.vy * dt;
                    p.x += p.vx * dt;
                    if p.vy > 0.0 && p.y <= g && ny >= g {
                        p.y = g - 0.25;
                        if p.kind == Kind::Spark {
                            p.vy = -p.vy * 0.45;
                            p.vx *= 0.7;
                        } else if p.vy > 130.0 && rng.chance(0.4) {
                            p.vy = -p.vy * 0.25;
                            p.vx = p.vx * 0.5 + rng.range(-10.0, 10.0);
                        } else {
                            p.vx = 0.0;
                            p.vy = 0.0;
                            p.life = p.life.min(0.5);
                        }
                    } else {
                        p.y = ny;
                    }
                    continue;
                }
                Kind::Smoke => {
                    p.vy -= if p.big { 23.0 * dt } else { 16.0 * dt };
                    p.vx += (self.time * 1.3 + p.seed).sin() * 8.0 * dt;
                    p.vx -= p.vx * 1.6 * dt;
                    p.vy -= p.vy * 1.2 * dt;
                }
                Kind::Mote => {
                    p.vx -= p.vx * 0.6 * dt;
                    p.vy -= p.vy * 0.6 * dt;
                }
                _ => {
                    let drag = if p.kind == Kind::Dust { 4.0 } else { 1.6 };
                    p.vx -= p.vx * drag * dt;
                    p.vy -= p.vy * drag * dt + 5.0 * dt;
                }
            }
            p.x += p.vx * dt;
            p.y += p.vy * dt;
        }
    }
    pub fn draw(&self, p: &mut Painter<'_>, layer: Layer) {
        for q in &self.list {
            if q.delay > 0.0 {
                continue;
            }
            let soft = matches!(q.kind, Kind::Dust | Kind::Smoke);
            if layer != Layer::All && soft != (layer == Layer::Back) {
                continue;
            }
            let k = q.life / q.max;
            let (x, y) = (q.x, q.y);
            let sp = q.vx.hypot(q.vy);
            let sp = if sp == 0.0 { 1.0 } else { sp };
            let (ux, uy) = (q.vx / sp, q.vy / sp);
            match q.kind {
                Kind::Spark => {
                    let s = &self.sparks;
                    let head = if k > 0.6 {
                        s[0]
                    } else if k > 0.3 {
                        s[1]
                    } else {
                        s[2]
                    };
                    let len = (1.0 + sp * 0.022).min(9.0);
                    p.line(
                        x - ux * len,
                        y - uy * len,
                        x - ux * len * 0.4,
                        y - uy * len * 0.4,
                        if k > 0.5 { head } else { s[s.len() - 1] },
                        1.0,
                    );
                    p.line(x - ux * len * 0.45, y - uy * len * 0.45, x, y, head, 1.0);
                    p.glow(x, y, 3.0, head, 0.45 * k);
                }
                Kind::Mote => {
                    let len = (0.5 + sp * 0.026).min(16.0) * q.size;
                    let a = if k < 0.35 { k / 0.35 } else { 1.0 };
                    p.line(
                        x - ux * len,
                        y - uy * len,
                        x - ux * len * 0.35,
                        y - uy * len * 0.35,
                        scale(q.color, 0.55),
                        a,
                    );
                    p.line(x - ux * len * 0.4, y - uy * len * 0.4, x, y, q.color, a);
                    p.glow(x, y, 2.5 + q.size, q.color, 0.4 * k);
                }
                Kind::Dust => p.disc(
                    x,
                    y,
                    q.size * (0.6 + (1.0 - k) * 0.9),
                    q.color,
                    k.powf(1.3) * 0.7,
                ),
                Kind::Smoke => {
                    let age = 1.0 - k;
                    let r = q.size
                        * (0.65
                            + (age * 2.5).min(1.0) * 0.45
                            + age * if q.big { 0.5 } else { 0.4 });
                    let a =
                        (age * 12.0).min(1.0) * (k * 2.4).min(1.0) * if q.big { 0.7 } else { 0.58 };
                    let ramp = [
                        hex(0x7b6f72),
                        hex(0x5d5660),
                        hex(0x433d48),
                        hex(0x2b2630),
                        hex(0x1a171e),
                    ];
                    let c = if q.color != [0; 4] {
                        mix(q.color, color::at(&ramp, age), age)
                    } else {
                        color::at(&ramp, age * 1.05)
                    };
                    p.disc(x, y, r, c, a * 0.45);
                    p.disc(x + r * 0.05, y + r * 0.06, r * 0.74, c, a * 0.62);
                    if age < 0.5 {
                        p.disc(
                            x - r * 0.22,
                            y - r * 0.3,
                            r * 0.5,
                            mix(c, WHITE, 0.12),
                            a * 0.4 * (1.0 - age * 2.0),
                        );
                    }
                }
                Kind::Grit => {
                    if k > 0.25 || k * 4.0 > 0.5 {
                        p.px(x, y, q.color);
                        if q.size >= 2.0 {
                            p.px(x + 1.0, y, q.color);
                            p.px(x, y + 1.0, scale(q.color, 0.8));
                            p.px(x + 1.0, y + 1.0, scale(q.color, 0.7));
                        }
                    }
                }
                Kind::Star => {
                    let tw = 0.6 + 0.4 * (self.time * (8.0 + q.seed % 5.0) + q.seed).sin();
                    if q.size < 1.0 {
                        if tw * k > 0.25 {
                            p.px(x, y, if tw > 0.8 { WHITE } else { q.color });
                        }
                        p.glow(x, y, 3.0, q.color, 0.4 * k * tw);
                    } else {
                        sparkle(
                            p,
                            x,
                            y,
                            q.size * tw * (k * 3.0).min(1.0),
                            q.color,
                            scale(q.color, 0.75),
                            0.5 * k * tw,
                        );
                    }
                }
            }
        }
    }
}
/// A rigid chunk or chip. `points` holds optional facet radii for impact rocks.
#[derive(Clone, Debug)]
pub struct Bit {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub life: f64,
    pub max: f64,
    pub g: f64,
    pub drag: f64,
    pub bounce: f64,
    pub a: f64,
    pub va: f64,
    pub size: f64,
    pub rest: bool,
    pub dusted: bool,
    pub previous_vy: f64,
    pub points: Vec<f64>,
}
impl Default for Bit {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            life: 1.0,
            max: 1.0,
            g: 1.0,
            drag: 0.0,
            bounce: 0.0,
            a: 0.0,
            va: 0.0,
            size: 1.0,
            rest: false,
            dusted: false,
            previous_vy: 0.0,
            points: vec![],
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct Bits {
    pub list: Vec<Bit>,
}
impl Bits {
    pub fn add(&mut self, mut b: Bit) {
        b.max = b.life;
        self.list.push(b);
    }
    pub fn update(&mut self, dt: f64, ground: &dyn Ground) {
        for i in (0..self.list.len()).rev() {
            let b = &mut self.list[i];
            b.life -= dt;
            if b.life <= 0.0 {
                self.list.remove(i);
                continue;
            }
            if b.rest {
                continue;
            }
            b.vy += GRAVITY * b.g * dt;
            b.vx -= b.vx * b.drag * dt;
            b.vy -= b.vy * b.drag * dt;
            b.x += b.vx * dt;
            b.y += b.vy * dt;
            b.a += b.va * dt;
            let floor = ground.height(b.x);
            let sp = b.vy;
            if b.g <= 0.0 || b.y < floor || sp <= 0.0 {
                continue;
            }
            b.y = floor - 0.01;
            if b.bounce > 0.0 && sp > 70.0 {
                b.vy = -sp * b.bounce;
                b.vx *= 0.7;
                b.va *= 0.6;
            } else {
                b.rest = true;
                b.vx = 0.0;
                b.vy = 0.0;
            }
        }
    }
}
#[derive(Clone, Debug)]
struct Light {
    x: f64,
    y: f64,
    r: f64,
    color: Color,
    k: f64,
    life: f64,
    max: f64,
}
#[derive(Clone, Debug, Default)]
pub struct Lights {
    list: Vec<Light>,
}
impl Lights {
    pub fn add(&mut self, x: f64, y: f64, r: f64, color: Color, k: f64, life: f64) {
        self.list.push(Light {
            x,
            y,
            r,
            color,
            k,
            life,
            max: life,
        });
    }
    pub fn update(&mut self, dt: f64) {
        for l in &mut self.list {
            l.life -= dt;
        }
        self.list.retain(|l| l.life > 0.0);
    }
    pub fn draw(&self, p: &mut Painter<'_>) {
        for l in &self.list {
            let k = l.life / l.max;
            p.glow(l.x, l.y, l.r * (0.55 + 0.45 * k), l.color, l.k * k * k);
        }
    }
}
pub fn sparkle(p: &mut Painter<'_>, x: f64, y: f64, arm: f64, c: Color, tip: Color, k: f64) {
    let n = round(arm) as i32;
    if n < 1 {
        return;
    }
    let (x0, y0) = (x.floor(), y.floor());
    for d in 1..=n {
        let cc = if d as f64 > n as f64 * 0.6 { tip } else { c };
        for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            p.px(x0 + dx * d as f64, y0 + dy * d as f64, cc);
        }
    }
    for (dx, dy) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        if n >= 3 {
            p.px(x0 + dx, y0 + dy, c);
        }
        if n >= 6 {
            p.px(x0 + dx * 2.0, y0 + dy * 2.0, tip);
        }
    }
    p.px(x0, y0, WHITE);
    p.glow(x, y, 4.0 + n as f64 * 2.0, c, k);
}
pub fn flare(p: &mut Painter<'_>, x: f64, y: f64, length: f64, k: f64, ramp: &[Color]) {
    if length < 1.0 {
        return;
    }
    let n = ramp.len();
    for i in 0..8 {
        let an = i as f64 * std::f64::consts::PI / 4.0;
        let len = length
            * if i % 4 == 0 {
                1.0
            } else if i % 2 == 0 {
                0.6
            } else {
                0.3
            };
        let mut d = 0.0;
        while d <= len {
            let u = d / len;
            p.dot(
                x + an.cos() * d,
                y + an.sin() * d,
                if u < 0.35 {
                    WHITE
                } else if u < 0.7 {
                    ramp[n - 2]
                } else {
                    ramp[n - 3]
                },
                if u < 0.6 {
                    1.0
                } else {
                    k * 1.5 * (1.0 - u) + 0.2
                },
            );
            d += 0.5;
        }
    }
}
pub fn band(
    p: &mut Painter<'_>,
    cx: f64,
    cy: f64,
    r: f64,
    sy: f64,
    th: f64,
    k: f64,
    ramp: &[Color],
    far: bool,
) {
    if r <= 0.0 || k <= 0.0 {
        return;
    }
    let n = ramp.len();
    let m = (r * TAU * 1.2).ceil() as usize;
    for j in 0..m {
        let ang = j as f64 / m as f64 * TAU;
        let (ca, sa) = (ang.cos(), ang.sin());
        if (sa < 0.0) != far {
            continue;
        }
        let mut o = th;
        while o >= 0.0 {
            let u = o / th;
            let c = if u < 0.15 {
                ramp[n - 1]
            } else if u < 0.4 {
                ramp[n - 2]
            } else if u < 0.7 {
                ramp[n - 3]
            } else {
                ramp[n - 4]
            };
            p.dot(
                cx + ca * (r - o),
                cy + sa * (r - o) * sy,
                c,
                k * if u < 0.4 { 1.2 } else { 1.15 - u },
            );
            o -= 0.5;
        }
    }
}
