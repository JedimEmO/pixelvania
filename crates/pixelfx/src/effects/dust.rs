//! Short dust puffs for running, jumping and landing, using the shared dust simulation.
use super::Options;
use crate::{
    color::{Color, WHITE, mix},
    math::Rng,
    painter::Painter,
    sim::{Ground, Kind, Layer, Particles},
};
/// Directional run puff, narrow jump puff or wider landing puff.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DustVariant {
    Run,
    Jump,
    Land,
}
#[derive(Clone)]
pub(super) struct Dust {
    rng: Rng,
    ps: Particles,
}
impl Dust {
    pub fn new(seed: u32, o: &Options) -> Self {
        let mut r = Rng::new(seed);
        let mut ps = Particles::default();
        let c: Color = o.palette[4];
        let lit = mix(c, WHITE, 0.3);
        let n = match o.dust {
            DustVariant::Run => 7,
            DustVariant::Jump => 10,
            DustVariant::Land => 16,
        };
        for i in 0..n {
            let side = if o.dust == DustVariant::Run {
                -1.0
            } else if i % 2 == 0 {
                1.0
            } else {
                -1.0
            };
            let speed = match o.dust {
                DustVariant::Run => [12.0, 38.0],
                DustVariant::Jump => [8.0, 24.0],
                DustVariant::Land => [30.0, 85.0],
            };
            spawn!(
                ps,
                &mut r,
                Kind::Dust,
                side * r.range(1.0, 4.0),
                -r.range(0.5, 2.0),
                side * r.range(speed[0], speed[1]),
                -r.range(
                    3.0,
                    if o.dust == DustVariant::Jump {
                        30.0
                    } else {
                        14.0
                    }
                ),
                r.range(0.22, 0.55),
                if r.chance(0.4) { lit } else { c },
                r.range(1.4, 3.2),
                r.range(0.0, 0.025),
                false
            );
        }
        Self { rng: r, ps }
    }
    pub fn update(&mut self, dt: f64, g: &dyn Ground) {
        self.ps.update(dt, &mut self.rng, g);
    }
    pub fn draw(&self, p: &mut Painter<'_>) {
        self.ps.draw(p, Layer::All);
    }
}
