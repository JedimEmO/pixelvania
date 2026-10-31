//! Five stock pixelfx effects and dust puffs, with caller-owned placement and ground queries.
use crate::{
    Frame,
    color::{Color, fit, hex},
    math::clamp,
    painter::Painter,
    sim::{Ground, Layer},
};
macro_rules! spawn {
    ($ps:expr,$rng:expr,$kind:expr,$x:expr,$y:expr,$vx:expr,$vy:expr,$life:expr,$col:expr,$size:expr,$delay:expr,$big:expr) => {{
        let (x, y, vx, vy, life, col, size, delay, big) =
            ($x, $y, $vx, $vy, $life, $col, $size, $delay, $big);
        $ps.spawn($rng, $kind, x, y, vx, vy, life, col, size, delay, big);
    }};
}
mod common;
mod dust;
mod hammer_smash;
mod heavy_landing;
mod hit_spark;
mod poof;
mod sword_slash;
pub use dust::DustVariant;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectId {
    HitSpark,
    HeavyLanding,
    SwordSlash,
    HammerSmash,
    Poof,
    Dust,
}
impl EffectId {
    pub const ALL: [Self; 6] = [
        Self::HitSpark,
        Self::HeavyLanding,
        Self::SwordSlash,
        Self::HammerSmash,
        Self::Poof,
        Self::Dust,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::HitSpark => "impacts/hit-spark",
            Self::HeavyLanding => "impacts/heavy-landing",
            Self::SwordSlash => "melee/sword-slash",
            Self::HammerSmash => "melee/hammer-smash",
            Self::Poof => "spawn/poof",
            Self::Dust => "dust",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|id| id.name() == s || id.name().rsplit('/').next() == Some(s))
    }
    pub fn metadata(self) -> Metadata {
        match self {
            Self::HitSpark => Metadata {
                size: [136, 80],
                origin: [44.0, 64.0],
                duration: 0.5,
                seed: 4,
            },
            Self::HeavyLanding => Metadata {
                size: [176, 80],
                origin: [88.0, 60.0],
                duration: 1.6,
                seed: 14,
            },
            Self::SwordSlash => Metadata {
                size: [136, 104],
                origin: [44.0, 92.0],
                duration: 0.75,
                seed: 2,
            },
            Self::HammerSmash => Metadata {
                size: [208, 112],
                origin: [80.0, 92.0],
                duration: 2.0,
                seed: 9,
            },
            Self::Poof => Metadata {
                size: [112, 92],
                origin: [56.0, 80.0],
                duration: 1.6,
                seed: 21,
            },
            Self::Dust => Metadata {
                size: [96, 56],
                origin: [48.0, 40.0],
                duration: 0.8,
                seed: 1,
            },
        }
    }
    fn palette(self) -> Vec<Color> {
        let nums: &[u32] = match self {
            Self::HitSpark => &[
                0x4a1712, 0xc9281f, 0xf2561d, 0xff8a2a, 0xffe26b, 0xfff3a6, 0xffffff,
            ],
            Self::HeavyLanding => &[
                0x1c1a24, 0x2f2a3a, 0x4a4258, 0x6b6280, 0x9088a4, 0xbdb6cc, 0xe6e1ee,
            ],
            Self::SwordSlash => &[
                0x212f6b, 0x2c58aa, 0x3a8bd8, 0x62c8f4, 0xb8c9e8, 0xe8f1ff, 0xffffff,
            ],
            Self::HammerSmash => &[
                0x141834, 0x242c5c, 0x38479a, 0x5a72d0, 0x8fa8f0, 0xc4d4ff, 0xeef3ff, 0xffffff,
            ],
            Self::Poof => &[0x2a2440, 0x5e5878, 0x9a94b4, 0xcdc8e0, 0xf2f0fa, 0xffffff],
            Self::Dust => return crate::color::DUST.to_vec(),
        };
        nums.iter().map(|&n| hex(n)).collect()
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Metadata {
    pub size: [u32; 2],
    pub origin: [f64; 2],
    pub duration: f64,
    pub seed: u32,
}
/// Effect-specific values plus painter placement controls. Empty palettes use stock colors.
#[derive(Clone, Debug)]
pub struct Options {
    pub palette: Vec<Color>,
    pub sparks: Vec<Color>,
    pub stars: Vec<Color>,
    pub stone: Color,
    pub height: f64,
    pub aim: f64,
    pub radius: f64,
    pub flip: bool,
    pub angle: f64,
    pub light: f64,
    pub dust: DustVariant,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            palette: vec![],
            sparks: vec![hex(0xfff3a6), hex(0xffe26b), hex(0xff8a2a), hex(0xf2561d)],
            stars: vec![hex(0xffd84a), hex(0xfff3a6), hex(0xffffff)],
            stone: hex(0x4a3f69),
            height: 20.0,
            aim: 0.1,
            radius: 42.0,
            flip: false,
            angle: 0.0,
            light: 1.0,
            dust: DustVariant::Run,
        }
    }
}
#[derive(Clone)]
enum State {
    Hit(hit_spark::HitSpark),
    Landing(heavy_landing::HeavyLanding),
    Sword(sword_slash::SwordSlash),
    Hammer(hammer_smash::HammerSmash),
    Poof(poof::Poof),
    Dust(dust::Dust),
}
/// A seeded simulation. Draw repeatedly without advancing it; update in seconds.
#[derive(Clone)]
pub struct Effect {
    pub id: EffectId,
    pub options: Options,
    seed: u32,
    time: f64,
    stop_at: f64,
    state: State,
}
impl Effect {
    pub fn new(id: EffectId, seed: u32, mut options: Options) -> Self {
        let defaults = id.palette();
        options.palette = if options.palette.is_empty() {
            defaults
        } else {
            fit(&options.palette, defaults.len())
        };
        if options.sparks.is_empty() {
            options.sparks = Options::default().sparks;
        } else {
            options.sparks = fit(&options.sparks, 4);
        }
        if options.stars.is_empty() {
            options.stars = Options::default().stars;
        } else {
            options.stars = fit(&options.stars, 3);
        }
        let state = match id {
            EffectId::HitSpark => State::Hit(hit_spark::HitSpark::new(seed, &options)),
            EffectId::HeavyLanding => {
                State::Landing(heavy_landing::HeavyLanding::new(seed, &options))
            }
            EffectId::SwordSlash => State::Sword(sword_slash::SwordSlash::new(seed, &options)),
            EffectId::HammerSmash => State::Hammer(hammer_smash::HammerSmash::new(seed, &options)),
            EffectId::Poof => State::Poof(poof::Poof::new(seed, &options)),
            EffectId::Dust => State::Dust(dust::Dust::new(seed, &options)),
        };
        Self {
            id,
            options,
            seed,
            time: 0.0,
            stop_at: f64::INFINITY,
            state,
        }
    }
    pub fn time(&self) -> f64 {
        self.time
    }
    /// Subdivides each update to 120 Hz or finer, matching `createFx` in JavaScript.
    pub fn update(&mut self, dt: f64, ground: &dyn Ground) {
        assert!(
            dt.is_finite() && dt >= 0.0,
            "dt must be finite and nonnegative"
        );
        self.time += dt;
        let n = (dt * 120.0 - 1e-9).ceil().max(1.0) as usize;
        for _ in 0..n {
            let dt = dt / n as f64;
            match &mut self.state {
                State::Hit(s) => s.update(dt, ground),
                State::Landing(s) => s.update(dt, ground),
                State::Sword(s) => s.update(dt, ground),
                State::Hammer(s) => s.update(dt, ground),
                State::Poof(s) => s.update(dt, ground),
                State::Dust(s) => s.update(dt, ground),
            }
        }
    }
    /// Replays from the seed at 30 fps to an absolute time. This also supports backwards scrubbing.
    /// Use `update` for live effects; seek assumes the ground query is stable during replay.
    pub fn seek(&mut self, t: f64, ground: &dyn Ground) {
        assert!(t.is_finite() && t >= 0.0);
        *self = Self::new(self.id, self.seed, self.options.clone());
        let frames = (t * 30.0 + 1e-9).floor() as usize;
        for _ in 0..frames {
            self.update(1.0 / 30.0, ground);
        }
        let remainder = t - frames as f64 / 30.0;
        if remainder > 1e-9 {
            self.update(remainder, ground);
        }
    }
    pub fn draw(&self, frame: &mut Frame, origin: [f64; 2]) {
        self.draw_layer(frame, origin, Layer::All);
    }
    /// Draws behind or in front of a character without clearing the destination buffers.
    pub fn draw_layer(&self, frame: &mut Frame, origin: [f64; 2], layer: Layer) {
        let mut p = Painter::new(frame);
        p.place(origin[0], origin[1], self.options.flip, self.options.angle);
        p.fade = clamp(1.0 - (self.time - self.stop_at) / 0.3);
        p.light_strength = self.options.light;
        if layer != Layer::Front {
            match &self.state {
                State::Landing(s) => s.back(&mut p),
                State::Hammer(s) => s.back(&mut p),
                State::Poof(s) => s.back(&mut p),
                _ => (),
            }
        }
        if layer != Layer::Back {
            match &self.state {
                State::Hit(s) => s.draw(&mut p),
                State::Landing(s) => s.draw(&mut p),
                State::Sword(s) => s.draw(&mut p),
                State::Hammer(s) => s.draw(&mut p),
                State::Poof(s) => s.draw(&mut p),
                State::Dust(s) => s.draw(&mut p),
            }
        }
    }
    pub fn stop(&mut self) {
        if self.stop_at.is_infinite() {
            self.stop_at = self.time;
        }
    }
    pub fn finished(&self) -> bool {
        self.time >= self.id.metadata().duration || self.time - self.stop_at >= 0.3
    }
    pub fn timing(&self) -> Option<crate::act::ActTiming> {
        crate::act::timing(self.id)
    }
}
