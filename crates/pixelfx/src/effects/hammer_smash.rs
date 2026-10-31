//! Hammer motion, force dome, floor cracks and settling debris.
// The rocks, grit, rolling dust and floor ring are adapted from Sprite Fusion's open-source explosion, and the
// dome from his ice nova (https://www.spritefusion.com/pixel-art-vfx), MIT License, Copyright (c) 2026 Hugo Duprez.
use super::{Options, common::*};
use crate::{
    color::{Color, WHITE, hex},
    math::{PI, Rng, TAU, clamp, out_cubic, rand, round, smooth},
    painter::{Ink, Painter},
    sim::{Bit, Bits, Ground, Kind, Layer, Lights, Particles, band},
};
const HOT: [Color; 4] = [hex(0xfff3a6), hex(0xffc23d), hex(0xff8a2a), hex(0xc9281f)];
const DROP: f64 = 0.15;
fn on(r: f64, u: f64) -> [f64; 2] {
    let a = -2.25 + u;
    [5.0 + a.cos() * r, -25.0 + a.sin() * r]
}
#[derive(Clone)]
pub(super) struct HammerSmash {
    c: Vec<Color>,
    st: Stone,
    rng: Rng,
    ps: Particles,
    bits: Bits,
    chips: Bits,
    lights: Lights,
    lines: Vec<FloorCrack>,
    rays: Vec<(f64, f64)>,
    u1: f64,
    x: f64,
    t: f64,
    hit: bool,
}
impl HammerSmash {
    pub fn new(seed: u32, o: &Options) -> Self {
        let mut r = Rng::new(seed);
        let mut u1 = 0.0;
        while on(32.0, u1)[1] < -0.5 && u1 < 4.0 {
            u1 += 0.005;
        }
        let x = on(32.0 - 13.0 / 2.0, u1)[0];
        let lines = floor_cracks(&mut r, x);
        let rays = [-0.95, -0.6, -0.3, -0.05, 0.22, 0.5, 0.85]
            .into_iter()
            .enumerate()
            .map(|(i, d)| {
                let a = -PI / 2.0 + d + r.range(-0.06, 0.06);
                let l = if i == 3 {
                    34.0
                } else if i == 2 || i == 4 {
                    26.0
                } else if i == 1 || i == 5 {
                    18.0
                } else {
                    12.0
                };
                (a, l * r.range(0.9, 1.1))
            })
            .collect();
        Self {
            c: o.palette.clone(),
            st: stone(o.stone),
            rng: r,
            ps: Particles::default(),
            bits: Bits::default(),
            chips: Bits::default(),
            lights: Lights::default(),
            lines,
            rays,
            u1,
            x,
            t: 0.0,
            hit: false,
        }
    }
    pub fn update(&mut self, dt: f64, g: &dyn Ground) {
        self.t += dt;
        let r = &mut self.rng;
        let x = self.x;
        let c = &self.c;
        let st = self.st;
        if !self.hit && self.t >= DROP {
            self.hit = true;
            self.lights.add(x, -5.0, 16.0, WHITE, 0.35, 0.08);
            self.lights.add(x, -10.0, 64.0, c[4], 0.42, 0.6);
            for _ in 0..15 {
                let s = r.sign();
                self.bits.add(Bit {
                    x: x + s * r.range(0.0, 5.0),
                    y: -2.0,
                    vx: s * r.range(20.0, 105.0),
                    vy: -r.range(110.0, 300.0),
                    g: 1.0,
                    drag: 0.35,
                    bounce: 0.38,
                    life: r.range(1.25, 1.65),
                    a: r.range(0.0, TAU),
                    va: r.range(-16.0, 16.0),
                    size: r.pick(&[1.0, 1.0, 2.0, 2.0, 3.0]),
                    ..Bit::default()
                });
            }
            for _ in 0..11 {
                let a = -PI / 2.0 + r.range(-1.2, 1.2);
                let sp = r.range(90.0, 230.0);
                self.chips.add(Bit {
                    x: x + r.range(-4.0, 4.0),
                    y: -2.0,
                    vx: a.cos() * sp,
                    vy: a.sin() * sp,
                    g: 0.9,
                    drag: 0.5,
                    bounce: 0.35,
                    life: r.range(0.9, 1.5),
                    size: 1.0,
                    ..Bit::default()
                });
            }
            for _ in 0..40 {
                let a = -PI / 2.0 + r.range(-1.1, 1.1);
                let sp = r.range(50.0, 210.0);
                spawn!(
                    self.ps,
                    r,
                    Kind::Grit,
                    x + r.range(-4.0, 4.0),
                    -1.0,
                    a.cos() * sp,
                    a.sin() * sp,
                    r.range(0.9, 1.5),
                    r.pick(&[st.light, st.body, st.body, st.shade]),
                    if r.chance(0.4) { 2.0 } else { 1.0 },
                    0.0,
                    false
                );
            }
            for i in 0..14 {
                let s = if i % 2 != 0 { -1.0 } else { 1.0 };
                let sp = r.range(120.0, 260.0);
                spawn!(
                    self.ps,
                    r,
                    Kind::Mote,
                    x + s * 6.0,
                    -r.range(1.0, 4.0),
                    s * sp,
                    -r.range(0.0, 30.0),
                    r.range(0.18, 0.34),
                    if r.chance(0.5) { c[6] } else { c[5] },
                    0.8,
                    r.range(0.02, 0.05),
                    false
                );
            }
            for _ in 0..24 {
                let a = -PI / 2.0 + r.range(-1.4, 1.4);
                let sp = r.range(150.0, 380.0);
                spawn!(
                    self.ps,
                    r,
                    Kind::Spark,
                    x + a.cos() * 6.0,
                    -2.0 + a.sin() * 4.0,
                    a.cos() * sp,
                    a.sin() * sp - 30.0,
                    r.range(0.12, 0.42),
                    [0; 4],
                    1.0,
                    r.range(0.035, 0.08),
                    false
                );
            }
            for i in 0..20 {
                let s = if i % 2 != 0 { -1.0 } else { 1.0 };
                spawn!(
                    self.ps,
                    r,
                    Kind::Smoke,
                    x + s * r.range(2.0, 10.0),
                    -r.range(2.0, 7.0),
                    s * r.range(40.0, 125.0),
                    -r.range(2.0, 14.0),
                    r.range(0.95, 1.5),
                    st.dust,
                    r.range(6.0, 9.0),
                    r.range(0.02, 0.08),
                    true
                );
            }
            for _ in 0..7 {
                spawn!(
                    self.ps,
                    r,
                    Kind::Smoke,
                    x + r.range(-6.0, 6.0),
                    -r.range(3.0, 9.0),
                    r.range(-20.0, 20.0),
                    -r.range(30.0, 80.0),
                    r.range(1.0, 1.4),
                    st.dust,
                    r.range(5.0, 7.5),
                    r.range(0.04, 0.14),
                    true
                );
            }
            for _ in 0..8 {
                spawn!(
                    self.ps,
                    r,
                    Kind::Dust,
                    x + r.range(-6.0, 6.0),
                    -r.range(1.0, 4.0),
                    r.range(-70.0, 70.0),
                    -r.range(8.0, 30.0),
                    r.range(0.35, 0.7),
                    st.dust,
                    r.range(2.0, 3.5),
                    0.0,
                    false
                );
            }
        }
        self.bits.update(dt, g);
        self.chips.update(dt, g);
        if self.hit && self.t > DROP + 0.4 && self.t < DROP + 1.2 && r.chance(0.08) {
            let idx = (r.sample() * self.bits.list.len() as f64).floor() as usize;
            if let Some(b) = self.bits.list.get(idx).filter(|b| b.rest) {
                spawn!(
                    self.ps,
                    r,
                    Kind::Star,
                    b.x + r.range(-1.0, 1.0),
                    b.y - 1.5 - b.size,
                    0.0,
                    -2.0,
                    r.range(0.25, 0.45),
                    if r.chance(0.5) { HOT[0] } else { WHITE },
                    r.range(0.6, 1.6),
                    0.0,
                    false
                );
            }
        }
        for b in &mut self.bits.list {
            if b.rest && !b.dusted {
                b.dusted = true;
                for _ in 0..3 {
                    spawn!(
                        self.ps,
                        r,
                        Kind::Dust,
                        b.x + r.range(-1.5, 1.5),
                        -0.5,
                        r.sign() * r.range(8.0, 35.0),
                        -r.range(2.0, 14.0),
                        r.range(0.3, 0.6),
                        st.dust,
                        r.range(1.3, 2.6),
                        0.0,
                        false
                    );
                }
            }
        }
        self.ps.update(dt, r, g);
        self.lights.update(dt);
    }
    fn wedge(&self, p: &mut Painter<'_>) {
        let c = &self.c;
        let x = clamp((self.t + 0.03) / (DROP + 0.03));
        let h = 0.28 * x + 0.72 * x * x;
        let u = self.u1 * h;
        let after = smooth(DROP, DROP + 0.07, self.t);
        let lag = (u - (0.3 + 1.05 * h)).max(0.0);
        let tail = lag + (u - lag) * after * 0.8;
        if u - tail < 0.03 || after >= 1.0 {
            return;
        }
        let rm = 32.0 - 13.0 / 2.0;
        let len = (u - tail) * rm;
        p.shade(
            [5.0 - 35.0, -25.0 - 35.0, 5.0 + 35.0, -25.0 + 35.0],
            |x, y| {
                let (dx, dy) = (x - 5.0, y + 25.0);
                let r = dx.hypot(dy);
                if !(32.0 - 14.0 - 1.0..=33.0).contains(&r) {
                    return None;
                }
                let mut uu = (dy.atan2(dx) + 2.25) % TAU;
                if uu < 0.0 {
                    uu += TAU;
                }
                while uu < tail {
                    uu += TAU;
                }
                if uu > u {
                    return None;
                }
                let lead = (u - uu) * rm;
                let f = lead / len;
                let w =
                    13.0 * if lead < 3.0 {
                        1.0
                    } else {
                        (1.0 - (lead - 3.0) / (len - 3.0).max(1.0)).powf(0.85)
                    } * (1.0 - after * 0.5);
                let off = (r - rm) / (w / 2.0 + 0.01);
                if w < 0.6 || off.abs() > 1.0 {
                    return None;
                }
                let ci = if lead < 2.0 {
                    0.0
                } else {
                    round(
                        1.0 + f * 3.4 + off.abs() * 1.4 - if off > 0.55 { 1.0 } else { 0.0 }
                            + after * 3.0,
                    )
                };
                if ci > (c.len() - 3) as f64 {
                    return None;
                }
                Some(Ink::new(c[c.len() - 1 - ci.max(0.0) as usize], 1.0))
            },
        );
        if after < 0.6 {
            for (dr, l0, ll) in [
                (-13.0 / 2.0 - 2.0, 3.0, 26.0),
                (-13.0 / 2.0 - 5.0, 9.0, 16.0),
                (13.0 / 2.0 + 2.0, 2.0, 30.0),
                (13.0 / 2.0 + 5.0, 8.0, 18.0),
                (-1.0, len + 2.0, 14.0),
                (3.0, len + 4.0, 9.0),
            ] {
                let r = rm + dr;
                let mut d = l0;
                while d < l0 + ll * (1.0 - after) {
                    let uu = u - d / r;
                    let k = (d - l0) / ll;
                    if uu < 0.0 {
                        break;
                    }
                    let [x, y] = on(r, uu);
                    p.dot(
                        x,
                        y,
                        if k < 0.25 {
                            c[6]
                        } else if k < 0.6 {
                            c[5]
                        } else {
                            c[4]
                        },
                        (1.0 - k) * (1.0 - after),
                    );
                    d += 0.5;
                }
            }
        }
        let [hx, hy] = on(rm, u);
        p.glow(
            hx,
            hy,
            34.0,
            c[4],
            0.5 * (1.0 - smooth(DROP - 0.03, DROP, self.t)),
        );
    }
    pub fn back(&self, p: &mut Painter<'_>) {
        let q = self.t - DROP;
        if q >= 0.0 {
            let fade = 1.0 - smooth(1.3, 1.7, q);
            for x in -11_i32..=11 {
                let u = x as f64 / 10.0;
                let d = if x.abs() > 10 {
                    0
                } else {
                    round(3.2 * (1.0 - u * u) + (rand(&[x, 4]) - 0.5) * 0.8) as i32
                };
                for y in 0..d {
                    p.dot(
                        self.x + x as f64,
                        y as f64,
                        if y == 0 { SOOT } else { self.st.dark },
                        fade,
                    );
                }
                if d > 0 {
                    p.dot(
                        self.x + x as f64,
                        d as f64,
                        if x.abs() < 7 {
                            self.st.light
                        } else {
                            self.st.body
                        },
                        fade,
                    );
                }
                if x.abs() >= 8 {
                    p.dot(
                        self.x + x as f64,
                        -1.0,
                        if x.abs() >= 10 {
                            self.st.body
                        } else {
                            self.st.light
                        },
                        fade,
                    );
                }
                if x.abs() == 9 && rand(&[x, 7]) < 0.7 {
                    p.dot(self.x + x as f64, -2.0, self.st.body, fade);
                }
            }
            self.floor_ring(p, q, true);
        }
        self.ps.draw(p, Layer::Back);
    }
    fn floor_ring(&self, p: &mut Painter<'_>, q: f64, far: bool) {
        let k = q / 0.5;
        if k < 1.0 {
            band(
                p,
                self.x,
                0.5,
                6.0 + 50.0 * out_cubic(k),
                0.16,
                (2.0 * (1.0 - k)).max(1.0),
                1.15 - k,
                &[self.st.body, self.c[3], self.c[5], self.c[7]],
                far,
            );
        }
    }
    pub fn draw(&self, p: &mut Painter<'_>) {
        let q = self.t - DROP;
        let c = &self.c;
        let x = self.x;
        self.wedge(p);
        if q >= 0.0 {
            let fade = 1.0 - smooth(1.3, 1.7, q);
            for line in &self.lines {
                let reach = 30.0
                    * out_cubic(clamp((q - line.at * 0.06) / 0.08))
                    * if line.fork { 0.3 } else { 1.0 };
                if reach > 0.0 {
                    along(&line.pts, reach, |x0, y0, x1, y1| {
                        if q >= 0.12 {
                            p.line(x0, y0 + 3.0, x1, y1 + 3.0, self.st.shade, fade);
                        }
                        p.line(
                            x0,
                            y0 + 2.0,
                            x1,
                            y1 + 2.0,
                            if q < 0.05 {
                                WHITE
                            } else if q < 0.12 {
                                c[5]
                            } else {
                                SOOT
                            },
                            fade,
                        );
                    });
                }
            }
            self.floor_ring(p, q, false);
            let d = (q - 0.045) / 0.4;
            if (0.0..1.0).contains(&d) {
                let r = 12.0 + 30.0 * out_cubic(d);
                let th = (3.6 * (1.0 - d).powf(0.8)).max(1.0);
                let a = if d < 0.6 { 1.0 } else { (1.0 - d) / 0.4 };
                p.shade([x - r - 1.0, -r - 1.0, x + r + 1.0, 0.0], |xx, y| {
                    let e = r - (xx - x).hypot(y);
                    if e < 0.0 || e > th || y > -0.5 {
                        return None;
                    }
                    let v = e / th;
                    let col = if d < 0.1 {
                        if v < 0.6 { WHITE } else { c[6] }
                    } else if v < 0.34 {
                        if d < 0.45 { WHITE } else { c[6] }
                    } else if v < 0.67 {
                        c[5]
                    } else {
                        c[4]
                    };
                    Some(Ink::new(col, a))
                });
                for i in 0..7 {
                    let g = -PI * (0.08 + i as f64 / 6.0 * 0.84);
                    p.glow(x + g.cos() * r, g.sin() * r, 14.0, c[4], 0.36 * (1.0 - d));
                }
            }
            if q < 0.2 {
                for &(a, l) in &self.rays {
                    let d1 = l * out_cubic((q / 0.05).min(1.0));
                    let d0 = l * smooth(0.04, 0.2, q);
                    let (ca, sa) = (a.cos(), a.sin());
                    if d1 - d0 < 1.0 {
                        continue;
                    }
                    p.line_fn(
                        x + ca * (2.0 + d0),
                        -1.0 + sa * (2.0 + d0),
                        x + ca * d1,
                        -1.0 + sa * d1,
                        |u| {
                            Ink::new(
                                if u < 0.5 {
                                    WHITE
                                } else if u < 0.8 {
                                    c[6]
                                } else {
                                    c[5]
                                },
                                1.0,
                            )
                        },
                    );
                    p.glow_line(
                        x + ca * d0,
                        -1.0 + sa * d0,
                        x + ca * d1,
                        -1.0 + sa * d1,
                        4.0,
                        c[5],
                        0.18 * (1.0 - q / 0.2),
                    );
                }
            }
            if q < 0.07 {
                let hot = q < 0.035;
                let (rx, ry) = if hot { (16.0, 3.0) } else { (12.0, 2.0) };
                p.ellipse(
                    x,
                    0.0,
                    rx + 2.0,
                    ry + 1.0,
                    |_| if hot { c[6] } else { c[5] },
                    1.0,
                );
                p.ellipse(x, 0.0, rx, ry, |_| if hot { WHITE } else { c[7] }, 1.0);
            }
            if q < 0.2 {
                p.glow(x, -5.0, 26.0, c[5], 0.3 * (-q / 0.05).exp());
            }
            p.glow(x, -1.0, 16.0, HOT[2], 0.28 * (-q / 0.35).exp());
        }
        for b in &self.bits.list {
            melee_rock(p, b, self.st);
        }
        for b in &self.chips.list {
            let k = b.life / b.max;
            let c = if k > 0.7 {
                HOT[0]
            } else if k > 0.45 {
                HOT[1]
            } else if k > 0.25 {
                HOT[2]
            } else {
                HOT[3]
            };
            p.dot(b.x, b.y, c, (k * 4.0).min(1.0));
            if !b.rest {
                p.dot(
                    b.x - b.vx * 0.012,
                    b.y - b.vy * 0.012,
                    HOT[3],
                    (k * 4.0).min(1.0),
                );
            }
            p.glow(b.x, b.y, 4.0, c, 0.35 * k);
        }
        self.ps.draw(p, Layer::Front);
        self.lights.draw(p);
    }
}
