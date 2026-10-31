//! Pixel effects and a posed character rig, drawn on the CPU without an engine dependency.
//!
//! ```
//! use pixelfx::{Effect, EffectId, Options, Frame};
//! let mut effect = Effect::new(EffectId::HitSpark, 4, Options::default());
//! let mut frame = Frame::new(136, 80);
//! effect.update(1.0 / 30.0, &|_| 0.0);
//! effect.draw(&mut frame, [44.0, 64.0]);
//! ```
//! The painter writes separate color and additive light buffers. Simulation coordinates are
//! art pixels, with positive y down; ground callbacks return a floor height in local coordinates.
pub mod act;
pub mod actor;
pub mod color;
pub mod effects;
pub mod math;
pub mod painter;
pub mod rig;
pub mod sim;
pub use effects::{Effect, EffectId, Options};
/// Separate RGBA buffers. Empty pixels are transparent black in both buffers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub color: Vec<[u8; 4]>,
    pub light: Vec<[u8; 4]>,
}
impl Frame {
    pub fn new(width: u32, height: u32) -> Self {
        let n = width as usize * height as usize;
        Self {
            width,
            height,
            color: vec![[0; 4]; n],
            light: vec![[0; 4]; n],
        }
    }
    pub fn clear(&mut self) {
        self.color.fill([0; 4]);
        self.light.fill([0; 4]);
    }
    /// Composites color and additive light over an opaque background for tool previews.
    pub fn composite(&self, bg: [u8; 3]) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.color.len() * 4);
        for (c, l) in self.color.iter().zip(&self.light) {
            let a = c[3] as f64 / 255.0;
            for i in 0..3 {
                out.push(
                    (c[i] as f64 * a + bg[i] as f64 * (1.0 - a) + l[i] as f64)
                        .round_ties_even()
                        .clamp(0.0, 255.0) as u8,
                );
            }
            out.push(255);
        }
        out
    }
}
