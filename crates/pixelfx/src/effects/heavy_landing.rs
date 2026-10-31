//! Floor dent, expanding dust rings and bouncing pebbles.
// The rocks, grit and dust are adapted from Sprite Fusion's open-source explosion
// (https://www.spritefusion.com/pixel-art-vfx/explosion), MIT License, Copyright (c) 2026 Hugo Duprez.
use super::{Options, common::*};
use crate::{
    color::{Color, WHITE, mix},
    math::{PI, Rng, TAU, out_cubic, round, smooth},
    painter::Painter,
    sim::{Bit, Bits, Ground, Kind, Layer, Particles, band},
};
#[derive(Clone)]
pub(super) struct HeavyLanding {
    c: Vec<Color>,
    st: Stone,
    rng: Rng,
    back: Particles,
    front: Particles,
    bits: Bits,
    cracks: Vec<Crack>,
    dust: Color,
    lit: Color,
    phase: f64,
    t: f64,
}
impl HeavyLanding {
    pub fn new(seed: u32, o: &Options) -> Self {
        let c = o.palette.clone();
        let st = stone(o.stone);
        let mut r = Rng::new(seed);
        let mut back = Particles::default();
        let mut front = Particles::default();
        let mut bits = Bits::default();
        let cracks = make_cracks(&mut r);
        let dust = mix(st.dust, c[5], 0.4);
        let lit = mix(dust, WHITE, 0.3);
        let phase = r.range(0.0, TAU);
        for (n, v0, v1, size, delay) in [
            (20, 140.0, 190.0, [2.2, 3.4], 0.01),
            (18, 70.0, 110.0, [3.4, 5.4], 0.12),
        ] {
            for i in 0..n {
                let g = i as f64 / n as f64 * TAU + r.range(-0.1, 0.1);
                let v = r.range(v0, v1);
                let ps = if g.sin() < 0.0 { &mut back } else { &mut front };
                spawn!(
                    ps,
                    &mut r,
                    Kind::Dust,
                    g.cos() * 4.0,
                    g.sin() * 0.7 - 1.0,
                    g.cos() * v,
                    g.sin() * v * 0.16 - r.range(2.0, 10.0),
                    r.range(0.45, 0.8),
                    if r.chance(0.45) { lit } else { dust },
                    r.range(size[0], size[1]),
                    delay + r.range(0.0, 0.03),
                    false
                );
            }
        }
        for i in 0..12 {
            let s = r.sign();
            let up = i < 6;
            let ps = if i % 2 != 0 { &mut front } else { &mut back };
            spawn!(
                ps,
                &mut r,
                Kind::Dust,
                s * r.range(2.0, 10.0),
                -r.range(1.0, 4.0),
                s * r.range(10.0, 35.0) * if up { 0.6 } else { 1.0 },
                -if up {
                    r.range(25.0, 60.0)
                } else {
                    r.range(4.0, 12.0)
                },
                r.range(0.55, 0.9),
                if r.chance(0.5) { lit } else { dust },
                r.range(3.0, 5.0),
                r.range(0.02, 0.1),
                false
            );
        }
        for _ in 0..12 {
            let s = r.sign();
            let v = r.range(50.0, 130.0);
            bits.add(Bit {
                x: s * r.range(3.0, 8.0),
                y: -1.0,
                vx: s * v,
                vy: -r.range(140.0, 230.0),
                g: 1.0,
                drag: 0.5,
                bounce: 0.55,
                life: r.range(1.0, 1.45),
                a: r.range(0.0, TAU),
                va: s * r.range(8.0, 16.0),
                size: if r.chance(0.3) { 2.0 } else { 1.0 },
                points: (0..6).map(|_| r.range(0.7, 1.12)).collect(),
                ..Bit::default()
            });
        }
        for _ in 0..24 {
            let s = r.sign();
            let a = -PI / 2.0 + s * r.range(0.3, 1.2);
            let v = r.range(60.0, 170.0);
            spawn!(
                front,
                &mut r,
                Kind::Grit,
                s * r.range(2.0, 7.0),
                -1.0,
                a.cos() * v,
                a.sin() * v,
                r.range(0.6, 1.1),
                r.pick(&[st.light, st.body, st.lip]),
                if r.chance(0.3) { 2.0 } else { 1.0 },
                0.0,
                false
            );
        }
        Self {
            c,
            st,
            rng: r,
            back,
            front,
            bits,
            cracks,
            dust,
            lit,
            phase,
            t: 0.0,
        }
    }
    pub fn update(&mut self, dt: f64, g: &dyn Ground) {
        self.t += dt;
        for b in &mut self.bits.list {
            b.previous_vy = b.vy;
        }
        self.bits.update(dt, g);
        let r = &mut self.rng;
        for b in &mut self.bits.list {
            if b.previous_vy > 80.0 && b.vy <= 0.0 {
                spawn!(
                    self.front,
                    r,
                    Kind::Dust,
                    b.x,
                    -0.5,
                    r.sign() * r.range(6.0, 20.0),
                    -r.range(2.0, 10.0),
                    r.range(0.25, 0.45),
                    self.dust,
                    r.range(1.0, 1.8),
                    0.0,
                    false
                );
                b.previous_vy = 0.0;
            }
        }
        self.back.update(dt, r, g);
        self.front.update(dt, r, g);
    }
    fn rings(&self, p: &mut Painter<'_>, far: bool) {
        for (t0, dur, r1, w, k0, edge) in [
            (0.01, 0.45, 58.0, 4.5, 0.75, true),
            (0.12, 0.85, 42.0, 6.0, 0.8, false),
        ] {
            let g = (self.t - t0) / dur;
            if g <= 0.0 || g >= 1.0 {
                continue;
            }
            let r = 6.0 + r1 * out_cubic(g);
            dust_ring(
                p,
                r,
                w * (1.0 + g),
                k0 * (1.0 - g).powf(1.2) * (g / 0.06).min(1.0),
                self.dust,
                self.phase + t0 * 10.0,
                g * 0.8,
                far,
            );
            if edge && g < 0.75 {
                band(
                    p,
                    0.0,
                    0.5,
                    r + 2.0,
                    0.2,
                    (2.5 * (1.0 - g)).max(1.0),
                    1.1 - g / 0.75,
                    &[self.st.body, self.st.light, self.lit, WHITE],
                    far,
                );
            }
        }
    }
    pub fn back(&self, p: &mut Painter<'_>) {
        let k = 1.0 - smooth(1.6 - 0.5, 1.6 - 0.1, self.t);
        for x in -10..=9 {
            let q = (x as f64 + 0.5).abs() / 10.0;
            let dep = round(1.8 * (1.0 - q * q)) as i32;
            if q < 0.82 {
                for y in 0..dep {
                    p.dot(x as f64, y as f64, SOOT, k);
                }
                p.dot(x as f64, dep as f64, self.st.lip, k);
            } else {
                p.dot(
                    x as f64,
                    -1.0,
                    if q > 0.92 { self.st.light } else { self.st.lip },
                    k,
                );
            }
        }
        draw_cracks(p, &self.cracks, self.t, self.st.light, k);
        self.rings(p, true);
        self.back.draw(p, Layer::All);
    }
    pub fn draw(&self, p: &mut Painter<'_>) {
        self.rings(p, false);
        self.front.draw(p, Layer::All);
        for b in &self.bits.list {
            rock(p, b, self.st);
        }
        let t = self.t;
        if t < 0.035 {
            p.ellipse(
                0.0,
                0.0,
                15.0,
                2.5,
                |d| if d < 0.55 { WHITE } else { self.c[6] },
                1.0,
            );
        } else if t < 0.07 {
            for x in -22_i32..=22 {
                p.dot(
                    x as f64,
                    0.0,
                    if x.abs() < 10 { WHITE } else { self.c[5] },
                    1.0 - x.abs() as f64 / 24.0,
                );
            }
        }
        if t < 0.12 {
            p.glow_line(
                -20.0,
                0.0,
                20.0,
                0.0,
                8.0,
                self.c[6],
                0.45 * (1.0 - t / 0.12),
            );
        }
    }
}
