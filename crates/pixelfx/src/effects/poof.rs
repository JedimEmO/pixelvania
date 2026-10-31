//! A pop, smoke balls and stars, followed by a slow lift.
// The soft, top-lit smoke puffs (a body, a denser core, a lit cap) follow Sprite Fusion's open-source explosion
// (https://www.spritefusion.com/pixel-art-vfx/explosion), MIT License, Copyright (c) 2026 Hugo Duprez.
use super::Options;
use crate::{
    color::{Color, WHITE},
    math::{PI, Rng, TAU, clamp, in_back, out_back, out_cubic, out_expo, smooth},
    painter::{Ink, Painter},
    sim::{Ground, Kind, Layer, Lights, Particles, flare},
};
const CY: f64 = -17.0;
#[derive(Clone)]
struct Ball {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    r0: f64,
    r1: f64,
    life: f64,
    max: f64,
    delay: f64,
}
impl Ball {
    fn radius(&self) -> f64 {
        let age = self.max - self.life;
        let u = age / self.max;
        let grow = self.r0 + (self.r1 - self.r0) * out_back(clamp(age / 0.22), 2.0);
        let breathe = 1.0 + 0.06 * (age * 14.0 + self.r1).sin();
        grow * breathe
            * if u < 0.62 {
                1.0
            } else {
                1.0 - in_back(clamp((u - 0.62) / 0.38), 1.4)
            }
    }
}
#[derive(Clone)]
pub(super) struct Poof {
    c: Vec<Color>,
    rng: Rng,
    ps: Particles,
    lights: Lights,
    balls: Vec<Ball>,
    lines: Vec<(f64, f64, f64)>,
    t: f64,
}
impl Poof {
    pub fn new(seed: u32, o: &Options) -> Self {
        let c = o.palette.clone();
        let s = &o.stars;
        let mut r = Rng::new(seed);
        let mut ps = Particles {
            sparks: vec![WHITE, s[1], s[0], c[2]],
            ..Particles::default()
        };
        let mut lights = Lights::default();
        let mut balls = vec![];
        let mut lines = vec![];
        for i in 0..9 {
            let a = i as f64 / 9.0 * TAU + r.range(-0.2, 0.2);
            let sp = r.range(95.0, 140.0);
            let rr = r.range(5.0, 7.0);
            balls.push(Ball {
                x: a.cos() * 3.0,
                y: CY + a.sin() * 3.0,
                vx: a.cos() * sp,
                vy: a.sin() * sp * 0.8 - 12.0,
                r0: rr * 0.5,
                r1: rr,
                life: r.range(0.7, 1.05),
                max: 0.0,
                delay: r.range(0.0, 0.03),
            });
        }
        for _ in 0..5 {
            let a = r.range(0.0, TAU);
            let sp = r.range(20.0, 60.0);
            balls.push(Ball {
                x: a.cos() * 2.0,
                y: CY + a.sin() * 2.0,
                vx: a.cos() * sp,
                vy: a.sin() * sp - 16.0,
                r0: 4.0,
                r1: r.range(6.5, 8.5),
                life: r.range(0.85, 1.2),
                max: 0.0,
                delay: r.range(0.02, 0.06),
            });
        }
        for _ in 0..7 {
            let a = r.range(0.0, TAU);
            let sp = r.range(170.0, 240.0);
            let rr = r.range(2.2, 3.6);
            balls.push(Ball {
                x: a.cos() * 4.0,
                y: CY + a.sin() * 4.0,
                vx: a.cos() * sp,
                vy: a.sin() * sp * 0.75 - 10.0,
                r0: rr * 0.6,
                r1: rr,
                life: r.range(0.32, 0.5),
                max: 0.0,
                delay: r.range(0.01, 0.04),
            });
        }
        for b in &mut balls {
            b.max = b.life;
        }
        for i in 0..12 {
            let a = i as f64 / 12.0 * TAU + r.range(-0.12, 0.12) + 0.13;
            let sa = a.sin();
            lines.push((
                a,
                r.range(24.0, 34.0)
                    .min(if sa > 0.1 { (-2.0 - CY) / sa } else { 99.0 }),
                r.range(0.0, 0.04),
            ));
        }
        for _ in 0..7 {
            let a = r.range(-PI, 0.3);
            let sp = r.range(70.0, 125.0);
            spawn!(
                ps,
                &mut r,
                Kind::Star,
                a.cos() * 6.0,
                CY + a.sin() * 6.0,
                a.cos() * sp,
                a.sin() * sp - 30.0,
                r.range(0.55, 0.95),
                r.pick(&[s[2], s[1], s[1], s[0]]),
                r.range(1.3, 2.4),
                r.range(0.02, 0.1),
                false
            );
        }
        for _ in 0..10 {
            let a = r.range(0.0, TAU);
            let sp = r.range(40.0, 110.0);
            spawn!(
                ps,
                &mut r,
                Kind::Star,
                a.cos() * 5.0,
                CY + a.sin() * 5.0,
                a.cos() * sp,
                a.sin() * sp - 20.0,
                r.range(0.3, 0.6),
                r.pick(&[WHITE, s[1]]),
                0.6,
                r.range(0.0, 0.08),
                false
            );
        }
        for _ in 0..6 {
            let s = r.sign();
            spawn!(
                ps,
                &mut r,
                Kind::Dust,
                s * r.range(1.0, 6.0),
                -1.0,
                s * r.range(25.0, 70.0),
                -r.range(2.0, 10.0),
                r.range(0.4, 0.7),
                c[2],
                r.range(1.6, 3.0),
                r.range(0.02, 0.08),
                false
            );
        }
        lights.add(0.0, CY, 46.0, c[4], 0.8, 0.4);
        lights.add(0.0, CY, 22.0, WHITE, 0.8, 0.1);
        Self {
            c,
            rng: r,
            ps,
            lights,
            balls,
            lines,
            t: 0.0,
        }
    }
    pub fn update(&mut self, dt: f64, g: &dyn Ground) {
        self.t += dt;
        for i in (0..self.balls.len()).rev() {
            let b = &mut self.balls[i];
            if b.delay > 0.0 {
                b.delay -= dt;
                continue;
            }
            b.life -= dt;
            if b.life <= 0.0 {
                if b.r1 > 4.0 {
                    spawn!(
                        self.ps,
                        &mut self.rng,
                        Kind::Star,
                        b.x,
                        b.y,
                        0.0,
                        -8.0,
                        0.22,
                        WHITE,
                        0.6,
                        0.0,
                        false
                    );
                }
                self.balls.remove(i);
                continue;
            }
            b.vx -= b.vx * 7.0 * dt;
            b.vy -= b.vy * 7.0 * dt + 30.0 * dt;
            b.x += b.vx * dt;
            b.y = (b.y + b.vy * dt).min(-b.radius() * 0.7);
        }
        self.ps.update(dt, &mut self.rng, g);
        self.lights.update(dt);
    }
    pub fn back(&self, p: &mut Painter<'_>) {
        let s = 1.0 - smooth(0.1, 0.9, self.t);
        if s > 0.0 {
            p.glow(0.0, -1.0, 26.0, self.c[4], 0.25 * s);
        }
        self.ps.draw(p, Layer::Back);
    }
    pub fn draw(&self, p: &mut Painter<'_>) {
        let c = &self.c;
        for layer in 0..3 {
            for b in &self.balls {
                if b.delay > 0.0 {
                    continue;
                }
                let age = b.max - b.life;
                let r = b.radius();
                let a = (age * 20.0).min(1.0);
                let lit = age < 0.12;
                if r < 0.6 {
                    continue;
                }
                let hy = (b.y - CY + 6.0) / 14.0;
                let lo = hy > 0.4;
                let hi = hy < -0.3;
                if layer == 0 {
                    p.disc(
                        b.x + r * 0.16,
                        b.y + r * 0.2,
                        r,
                        if lit {
                            c[3]
                        } else if lo {
                            c[1]
                        } else {
                            c[2]
                        },
                        0.8 * a,
                    );
                } else if layer == 1 {
                    p.disc(
                        b.x - r * 0.06,
                        b.y - r * 0.08,
                        r * 0.86,
                        if lit {
                            c[4]
                        } else if lo {
                            c[2]
                        } else {
                            c[3]
                        },
                        0.88 * a,
                    );
                } else if !lo || lit {
                    p.disc(
                        b.x - r * 0.28,
                        b.y - r * 0.34,
                        r * 0.42,
                        if lit || hi { WHITE } else { c[4] },
                        0.85 * a,
                    );
                    if age < 0.25 {
                        p.disc(
                            b.x - r * 0.3,
                            b.y - r * 0.38,
                            r * 0.2,
                            WHITE,
                            0.9 * a * (1.0 - age / 0.25),
                        );
                    }
                }
            }
        }
        let t = self.t;
        if t < 0.07 {
            let k = t / 0.07;
            p.circle(0.0, CY, 9.0 + 3.0 * k, c[4], 1.0);
            p.circle(0.0, CY, 7.0 + 3.0 * k, WHITE, 1.0);
            flare(p, 0.0, CY, 20.0 * (1.0 - k * 0.5), 1.0 - k, c);
        }
        let rk = (t - 0.03) / 0.2;
        if rk > 0.0 && rk < 1.0 {
            p.ring(
                0.0,
                CY,
                8.0 + 18.0 * out_cubic(rk),
                (2.5 * (1.0 - rk)).max(1.0),
                if rk < 0.3 { WHITE } else { c[4] },
                1.1 - rk,
            );
        }
        for &(a, r1, d) in &self.lines {
            let u = (t - d) / 0.2;
            if u <= 0.0 || u >= 1.0 {
                continue;
            }
            let head = 8.0 + (r1 - 8.0) * out_expo(clamp(u / 0.6));
            let tail = 8.0 + (r1 - 8.0) * u * u * u;
            let (ca, sa) = (a.cos(), a.sin());
            p.line_fn(ca * tail, CY + sa * tail, ca * head, CY + sa * head, |v| {
                Ink::new(if v > 0.6 { WHITE } else { c[4] }, 1.0)
            });
            p.glow_line(
                ca * tail,
                CY + sa * tail,
                ca * head,
                CY + sa * head,
                3.0,
                c[4],
                0.25,
            );
        }
        self.ps.draw(p, Layer::Front);
        self.lights.draw(p);
    }
}
