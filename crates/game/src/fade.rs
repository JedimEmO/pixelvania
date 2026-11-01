//! The progression fade: the world loses its colour as the player gains abilities.
//!
//! Each major ability takes a little colour out of the world's surfaces, and light never fades,
//! so the late game is grey stone under vivid lamps. The High brings most of the colour back
//! for as long as it lasts, which makes it more tempting just as it grows less effective.
//! The fade also thins the music's default mix.
//!
//! Until the game has abilities to collect, the level is set by hand:
//! F6 steps it through none, a third, two thirds and all, and `PIXELVANIA_FADE=0.5` starts there.
//! The ballroom decides whether the idea stays (see `docs/plan.md`).

use bevy::prelude::*;

use crate::combat::Mood;
use crate::render::Lighting;

pub struct FadePlugin;

impl Plugin for FadePlugin {
    fn build(&self, app: &mut App) {
        let level = std::env::var("PIXELVANIA_FADE")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        app.insert_resource(ProgressionFade { level })
            .add_systems(Update, (step_by_hand, grade).chain());
    }
}

/// How far the world has faded, from 0 (all its colour) to 1 (none in its surfaces).
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct ProgressionFade {
    pub level: f32,
}

impl ProgressionFade {
    /// The steps F6 cycles through.
    pub const STEPS: [f32; 4] = [0.0, 0.34, 0.67, 1.0];

    /// The fade that shows in this mood: the High gives back most of the colour.
    pub fn shown(&self, mood: &Mood) -> f32 {
        if mood.name == "the High" {
            self.level * HIGH_KEEPS
        } else {
            self.level
        }
    }
}

/// The share of the fade that still shows during a High.
const HIGH_KEEPS: f32 = 0.2;
/// How fast the picture eases to a new fade, per second. Slower than the mood's grade,
/// so colour returning at a High's onset reads as a bloom.
const RATE: f32 = 1.2;

fn step_by_hand(keys: Res<ButtonInput<KeyCode>>, mut fade: ResMut<ProgressionFade>) {
    if keys.just_pressed(KeyCode::F6) {
        let steps = ProgressionFade::STEPS;
        let next = steps
            .iter()
            .position(|s| *s > fade.level + 1e-3)
            .map_or(steps[0], |i| steps[i]);
        fade.level = next;
        info!("progression fade: {next:.2}");
    }
}

/// Eases the composite's fade towards what the level and the mood ask for.
fn grade(
    time: Res<Time<Real>>,
    fade: Res<ProgressionFade>,
    mood: Res<Mood>,
    mut lighting: ResMut<Lighting>,
) {
    let want = fade.shown(&mood);
    let k = 1.0 - (-RATE * time.delta_secs()).exp();
    let next = lighting.fade + (want - lighting.fade) * k;
    // Only write when it moves, so the composite is not re-uploaded every frame.
    if (next - lighting.fade).abs() > 1e-4 {
        lighting.fade = next;
    } else if lighting.fade != want {
        lighting.fade = want;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_high_gives_back_most_of_the_colour() {
        let fade = ProgressionFade { level: 1.0 };
        let [neutral, high, comedown] = Mood::ALL;
        assert_eq!(fade.shown(&neutral), 1.0);
        assert_eq!(fade.shown(&comedown), 1.0);
        assert!(fade.shown(&high) < 0.25);
    }
}
