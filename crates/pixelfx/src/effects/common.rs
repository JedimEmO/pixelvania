//! Stone colors, floor cracks, dust rings and shaded rocks shared by the effects.
// Ported from pixelfx impacts/_lib.js and melee/_lib.js.
// The star and flying-arc recipes are adapted from Sprite Fusion's open-source impact
// (https://www.spritefusion.com/pixel-art-vfx/impact), and stone colors from its explosion
// (https://www.spritefusion.com/pixel-art-vfx/explosion), MIT License, Copyright (c) 2026 Hugo Duprez.
// The crescent and trace recipe is adapted from Sprite Fusion's open-source slash
// (https://www.spritefusion.com/pixel-art-vfx/slash), MIT License, Copyright (c) 2026 Hugo Duprez.
use crate::{
    color::{Color, WHITE, hex, mix, scale},
    math::{PI, Rng, TAU, clamp, out_cubic, rand, round},
    painter::{Ink, Painter},
    sim::Bit,
};
pub const SOOT: Color = hex(0x0d0a12);
const OUTLINE: Color = hex(0x1a1422);
#[derive(Clone, Copy)]
pub struct Stone {
    pub light: Color,
    pub body: Color,
    pub shade: Color,
    pub dust: Color,
    pub lip: Color,
    pub dark: Color,
}
pub fn stone(c: Color) -> Stone {
    Stone {
        light: mix(scale(c, 1.35), WHITE, 0.08),
        body: c,
        shade: scale(c, 0.64),
        dust: mix(scale(c, 1.2), hex(0x968c96), 0.3),
        lip: mix(scale(c, 1.55), WHITE, 0.12),
        dark: scale(c, 0.42),
    }
}
#[derive(Clone)]
pub struct Crack {
    pub paths: Vec<Vec<[f64; 3]>>,
    pub len: f64,
    pub t0: f64,
}
fn walk(
    r: &mut Rng,
    mut x: f64,
    mut y: f64,
    a: f64,
    l: f64,
    jit: f64,
    step: [f64; 2],
    d0: f64,
) -> Vec<[f64; 3]> {
    let mut pts = vec![[x, y, d0]];
    let (mut d, mut cur) = (0.0, a);
    while d < l - 0.01 {
        let s = (l - d).min(r.range(step[0], step[1]));
        cur += r.range(-jit, jit) + (a - cur) * 0.45;
        x += cur.cos() * s;
        y = (y + cur.sin() * s).clamp(0.5, 12.0);
        d += s;
        pts.push([x, y, d0 + d]);
    }
    pts
}
pub fn make_cracks(r: &mut Rng) -> Vec<Crack> {
    let mut out = vec![];
    for i in 0..6 {
        let side = if i % 2 != 0 { 1.0 } else { -1.0 };
        let dip = 0.06 + r.sample().powf(1.7) * 0.7;
        let l = r.range(8.0, 20.0) * (1.0 - (0.5 * dip) / (0.7 + 0.06));
        let a = if side > 0.0 { dip } else { PI - dip };
        let x = side * r.range(2.0, 6.0);
        let y = r.range(0.5, 1.5);
        let pts = walk(r, x, y, a, l, 0.55, [2.0, 4.0], 0.0);
        let t0 = r.range(0.0, 0.035);
        let mut kids = vec![];
        for j in 2..pts.len() - 1 {
            if r.chance(0.4 * 0.4) {
                let [x, y, d] = pts[j];
                let aa = a + r.sign() * r.range(0.45, 1.0);
                let ll = r.range(3.0, 9.0) * (1.0 - (0.5 * j as f64) / pts.len() as f64);
                kids.push(walk(r, x, y, aa, ll, 0.7, [1.5, 3.0], d));
            }
        }
        let mut paths = vec![pts];
        paths.extend(kids);
        out.push(Crack { paths, len: l, t0 });
    }
    out
}
pub fn draw_cracks(p: &mut Painter<'_>, cracks: &[Crack], t: f64, lip: Color, k: f64) {
    if k <= 0.0 {
        return;
    }
    for c in cracks {
        let g = c.len * out_cubic(clamp((t - c.t0) / 0.08));
        if g <= 0.0 {
            continue;
        }
        for path in &c.paths {
            for pair in path.windows(2) {
                let [x0, y0, d0] = pair[0];
                let [x1, y1, d1] = pair[1];
                if d0 >= g {
                    break;
                }
                let f = if d1 > g { (g - d0) / (d1 - d0) } else { 1.0 };
                let (ex, ey) = (x0 + (x1 - x0) * f, y0 + (y1 - y0) * f);
                p.line_fn(x0, y0 + 1.0, ex, ey + 1.0, |u| {
                    Ink::new(
                        lip,
                        if rand(&[
                            round(x0 + (ex - x0) * u) as i32,
                            round(y0 + (ey - y0) * u) as i32,
                            5,
                        ]) < 0.3
                        {
                            k * 0.8
                        } else {
                            0.0
                        },
                    )
                });
                p.line(x0, y0, ex, ey, SOOT, k);
            }
        }
    }
}
pub fn dust_ring(
    p: &mut Painter<'_>,
    r: f64,
    w: f64,
    k: f64,
    c: Color,
    ph: f64,
    brk: f64,
    far: bool,
) {
    if k <= 0.01 || r <= 0.0 {
        return;
    }
    let ww = w * 1.6;
    let lite = mix(c, WHITE, 0.35);
    for y in (-(r + ww) * 0.2).floor() as i32 - 1..=((r + ww) * 0.2).ceil() as i32 + 1 {
        for x in (-r - ww).floor() as i32 - 1..=(r + ww).ceil() as i32 + 1 {
            let dx = x as f64 + 0.5;
            let dy = (y as f64 + 0.5) / 0.2;
            let d = dx.hypot(dy);
            let g = dy.atan2(dx);
            if (dy < 0.0) != far {
                continue;
            }
            let lumpy = 0.6
                + 0.3 * (5.0 * g + ph).sin()
                + 0.18 * (11.0 * g + ph * 2.1).sin()
                + 0.12 * (17.0 * g + ph * 0.7).sin();
            let wt = w * lumpy;
            let off = (d - r - w * 0.25 * (7.0 * g + ph).sin()).abs();
            if off >= wt {
                continue;
            }
            let tear = if brk > 0.0 {
                clamp((0.55 + 0.45 * (6.0 * g + ph * 1.7).sin() - brk) / 0.25)
            } else {
                1.0
            };
            let a = k * (1.0 - (off / wt).powi(2)) * tear;
            if a > 0.01 {
                p.blend(
                    x as f64,
                    y as f64,
                    if dy < 0.0 && d > r - wt * 0.3 {
                        lite
                    } else {
                        c
                    },
                    a,
                );
            }
        }
    }
}
pub fn rock(p: &mut Painter<'_>, b: &Bit, st: Stone) {
    let r = 0.75 + b.size * 0.8;
    let n = b.points.len();
    let fade = (b.life / 0.5).min(1.0);
    let (ca, sa) = (b.a.cos(), b.a.sin());
    p.shade(
        [b.x - r - 2.0, b.y - r - 2.0, b.x + r + 2.0, b.y + r + 2.0],
        |x, y| {
            let (dx, dy) = (x - b.x, y - b.y);
            let (u, v) = (dx * ca + dy * sa, -dx * sa + dy * ca);
            let d = u.hypot(v);
            let mut ang = v.atan2(u);
            if ang < 0.0 {
                ang += TAU;
            }
            let f = ang / TAU * n as f64;
            let i = f.floor() as usize % n;
            let w = f - f.floor();
            let edge = r * (b.points[i] * (1.0 - w) + b.points[(i + 1) % n] * w);
            if d > edge + 0.85 {
                return None;
            }
            let lit = (((i as f64 + 0.5) / n as f64) * TAU + b.a - (-1.0_f64).atan2(-1.0)).cos();
            let c = if d > edge {
                OUTLINE
            } else if d < edge * 0.42 {
                if lit > 0.55 { st.light } else { st.body }
            } else if lit > 0.35 {
                st.light
            } else if lit < -0.4 {
                st.shade
            } else {
                st.body
            };
            Some(Ink::new(c, fade))
        },
    );
}
#[derive(Clone)]
pub struct FloorCrack {
    pub pts: Vec<[f64; 2]>,
    pub at: f64,
    pub fork: bool,
}
pub fn floor_cracks(r: &mut Rng, x0: f64) -> Vec<FloorCrack> {
    let mut out = vec![];
    for i in 0..5 {
        let side = if r.chance(0.5) { 1.0 } else { -1.0 };
        let l = 24.0 * r.range(0.45, 1.0) * if i < 2 { 1.0 } else { 0.7 };
        let mut x = x0 + side * r.range(0.0, 1.5);
        let mut y = 0.0;
        let mut pts = vec![[x, y]];
        while (x - x0).abs() < l && y < 6.0 {
            x += side * r.range(1.5, 3.5);
            y = (y + r.range(-0.6, 1.6) * if (x - x0).abs() > 3.0 { 1.0 } else { 0.3 })
                .clamp(0.0, 6.0);
            pts.push([round(x), round(y)]);
            if r.chance(0.18) && pts.len() > 3 && y < 4.0 {
                let mut f = vec![[round(x), round(y)]];
                let (mut fx, mut fy) = (x, y);
                for _ in 0..r.int(1, 3) {
                    fx += side * r.range(1.0, 3.0);
                    fy += r.range(0.8, 2.0);
                    f.push([round(fx), round(fy.min(7.0))]);
                }
                out.push(FloorCrack {
                    pts: f,
                    at: (x - x0).abs() / 24.0,
                    fork: true,
                });
            }
        }
        out.push(FloorCrack {
            pts,
            at: 0.0,
            fork: false,
        });
    }
    out
}
pub fn along(pts: &[[f64; 2]], len: f64, mut f: impl FnMut(f64, f64, f64, f64)) {
    let mut d = 0.0;
    for pair in pts.windows(2) {
        if d >= len {
            break;
        }
        let [x0, y0] = pair[0];
        let [x1, y1] = pair[1];
        let l = (x1 - x0).hypot(y1 - y0);
        let k = ((len - d) / l).min(1.0);
        f(x0, y0, x0 + (x1 - x0) * k, y0 + (y1 - y0) * k);
        d += l;
    }
}
pub fn melee_rock(p: &mut Painter<'_>, b: &Bit, st: Stone) {
    let r = 0.6 + b.size * 0.75;
    let fade = (b.life / 0.5).min(1.0);
    let lx = b.a.cos() * r * 0.4;
    let ly = b.a.sin() * r * 0.4;
    p.shade(
        [b.x - r - 1.0, b.y - r - 1.0, b.x + r + 1.0, b.y + r + 1.0],
        |x, y| {
            let (dx, dy) = (x - b.x, y - b.y);
            let d = (dx * 1.15).hypot(dy);
            if d > r + 0.9 {
                return None;
            }
            let lit = dx * lx + dy * ly;
            Some(Ink::new(
                if d > r {
                    OUTLINE
                } else if lit < -0.4 {
                    st.light
                } else if lit > 0.6 {
                    st.shade
                } else {
                    st.body
                },
                fade,
            ))
        },
    );
}
