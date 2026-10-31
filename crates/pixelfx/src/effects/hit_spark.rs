//! Directional cross flare, sparks and hot chips.
// The two-tone spark spray and light pop are adapted from Sprite Fusion's open-source impact
// (https://www.spritefusion.com/pixel-art-vfx/impact), MIT License, Copyright (c) 2026 Hugo Duprez.
use super::Options;
use crate::{
    color::{Color, WHITE},
    math::{PI, Rng},
    painter::Painter,
    sim::{Ground, Kind, Layer, Lights, Particles},
};
#[derive(Clone)]
pub(super) struct HitSpark {
    c: Vec<Color>,
    rng: Rng,
    y: f64,
    aim: f64,
    rays: Vec<(f64, f64, f64)>,
    ps: Particles,
    lights: Lights,
    t: f64,
}
impl HitSpark {
    pub fn new(seed: u32, o: &Options) -> Self {
        let c = o.palette.clone();
        let mut r = Rng::new(seed);
        let y = -o.height;
        let a = o.aim;
        let mut ps = Particles {
            sparks: vec![c[6], c[5], c[4], c[2]],
            ..Particles::default()
        };
        let mut lights = Lights::default();
        let tilt = r.range(-0.08, 0.08);
        let rays = vec![
            (a + tilt, r.range(33.0, 37.0), 2.2),
            (a + PI + tilt, r.range(8.0, 10.0), 2.2),
            (a - PI / 2.0 + tilt, r.range(15.0, 18.0), 2.6),
            (a + PI / 2.0 + tilt, r.range(10.0, 12.0), 2.3),
            (a - PI / 4.0 + tilt, r.range(5.0, 7.0), 1.0),
        ];
        lights.add(0.0, y, 38.0, c[4], 0.35, 0.18);
        lights.add(a.cos() * 10.0, y + a.sin() * 10.0, 22.0, c[5], 0.45, 0.1);
        lights.add(0.0, -2.0, 26.0, c[3], 0.22, 0.22);
        for i in 0..9 {
            let a = a + r.range(-0.6, 0.6) - 0.15;
            let v = r.range(170.0, 360.0);
            spawn!(
                ps,
                &mut r,
                Kind::Spark,
                1.0,
                y,
                a.cos() * v,
                a.sin() * v - 40.0,
                r.range(0.25, 0.45),
                WHITE,
                1.0,
                if i > 6 { r.range(0.01, 0.03) } else { 0.0 },
                false
            );
        }
        for _ in 0..6 {
            let a = a + r.range(-1.1, 0.5) - 0.3;
            let v = r.range(70.0, 170.0);
            spawn!(
                ps,
                &mut r,
                Kind::Grit,
                1.0,
                y,
                a.cos() * v,
                a.sin() * v - r.range(30.0, 80.0),
                r.range(0.35, 0.45),
                r.pick(&[c[5], c[4], c[4], c[3]]),
                1.0,
                0.0,
                false
            );
        }
        Self {
            c,
            rng: r,
            y,
            aim: o.aim,
            rays,
            ps,
            lights,
            t: 0.0,
        }
    }
    pub fn update(&mut self, dt: f64, g: &dyn Ground) {
        self.t += dt;
        self.ps.update(dt, &mut self.rng, g);
        self.lights.update(dt);
    }
    pub fn draw(&self, p: &mut Painter<'_>) {
        self.ps.draw(p, Layer::All);
        let t = self.t;
        let c = &self.c;
        if t < 0.025 {
            for &(a, l, w) in &self.rays {
                ray(p, self.y, a, l, w, c, 0.0);
            }
            diamond(p, self.y, 4, c);
        } else if t < 0.058 {
            for &(a, l, w) in &self.rays {
                ray(p, self.y, a, l * 1.2, w * 0.55, c, 0.12);
            }
            diamond(p, self.y, 3, c);
        } else if t < 0.092 {
            let (a, l, _) = self.rays[0];
            ray(
                p,
                self.y,
                a,
                l * 1.3,
                0.6,
                &[c[0], c[1], c[2], c[3], c[3], c[4]],
                0.4,
            );
        }
        let pop = (-t / 0.045).exp();
        if pop > 0.02 {
            p.glow(0.0, self.y, 22.0, c[4], 0.3 * pop);
            p.glow(0.0, self.y, 7.0, WHITE, 0.15 * pop);
            p.glow_line(
                0.0,
                self.y,
                self.aim.cos() * 26.0,
                self.y + self.aim.sin() * 26.0,
                4.0,
                c[5],
                0.25 * pop,
            );
        }
        self.lights.draw(p);
    }
}
fn ray(p: &mut Painter<'_>, y: f64, a: f64, l: f64, w: f64, c: &[Color], from: f64) {
    let (ca, sa) = (a.cos(), a.sin());
    let mut d = from * l;
    while d <= l {
        let u = d / l;
        let hw = w * (1.0 - u).powf(1.6);
        let col = if u < 0.3 {
            WHITE
        } else if u < 0.62 {
            c[5]
        } else if u < 0.88 {
            c[4]
        } else {
            c[3]
        };
        p.px(ca * d, y + sa * d, col);
        if hw > 0.45 {
            let mut o = 0.5;
            while o <= hw {
                for e in [-o, o] {
                    p.px(
                        ca * d - sa * e,
                        y + sa * d + ca * e,
                        if u < 0.3 && o < 1.0 { c[5] } else { c[4] },
                    );
                }
                o += 0.5;
            }
        }
        d += 0.5;
    }
}
fn diamond(p: &mut Painter<'_>, y: f64, r: i32, c: &[Color]) {
    for dy in -r - 1..=r + 1 {
        for dx in -r - 1..=r + 1 {
            let m = dx.abs() + dy.abs();
            if m <= r {
                p.px(dx as f64, y + dy as f64, WHITE);
            } else if m == r + 1 {
                p.px(dx as f64, y + dy as f64, c[3]);
            }
        }
    }
}
