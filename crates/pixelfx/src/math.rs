//! Deterministic randomness, interpolation and easing, ported from pixelfx core/math.js.

pub use std::f64::consts::{PI, TAU};
pub fn clamp(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}
/// Rounds ties toward positive infinity, as JavaScript does.
pub fn round(x: f64) -> f64 {
    (x + 0.5).floor()
}
pub fn smooth(a: f64, b: f64, t: f64) -> f64 {
    let u = clamp((t - a) / (b - a));
    u * u * (3.0 - 2.0 * u)
}
pub fn out_cubic(t: f64) -> f64 {
    1.0 - (1.0 - t).powi(3)
}
pub fn out_expo(t: f64) -> f64 {
    if t >= 1.0 {
        1.0
    } else {
        1.0 - 2.0_f64.powf(-10.0 * t)
    }
}
pub fn out_back(t: f64, s: f64) -> f64 {
    if t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else {
        1.0 + (s + 1.0) * (t - 1.0).powi(3) + s * (t - 1.0).powi(2)
    }
}
pub fn in_back(t: f64, s: f64) -> f64 {
    if t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else {
        (s + 1.0) * t * t * t - s * t * t
    }
}
pub fn hash(ns: &[i32]) -> u32 {
    let mut h = 0x9e3779b9u32;
    for &n in ns {
        h = (h ^ n as u32).wrapping_mul(0x85ebca6b);
        h ^= h >> 13;
        h = h.wrapping_mul(0xc2b2ae35);
        h ^= h >> 16;
    }
    h
}
pub fn rand(ns: &[i32]) -> f64 {
    hash(ns) as f64 / 4294967296.0
}
/// Mulberry32 with the original zero-seed fallback and wrapping integer arithmetic.
#[derive(Clone, Debug)]
pub struct Rng {
    state: u32,
}
impl Rng {
    pub fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }
    pub fn sample(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x6d2b79f5);
        let mut t = self.state;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        (t ^ (t >> 14)) as f64 / 4294967296.0
    }
    pub fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.sample()
    }
    pub fn int(&mut self, a: i32, b: i32) -> i32 {
        self.range(a as f64, (b + 1) as f64).floor() as i32
    }
    pub fn chance(&mut self, p: f64) -> bool {
        self.sample() < p
    }
    pub fn sign(&mut self) -> f64 {
        if self.sample() < 0.5 { -1.0 } else { 1.0 }
    }
    pub fn pick<T: Copy>(&mut self, list: &[T]) -> T {
        list[(self.sample() * list.len() as f64).floor() as usize]
    }
}
