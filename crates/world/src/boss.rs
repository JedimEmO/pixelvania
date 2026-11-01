//! The Dancer's phases and repeating phrases.

use crate::combat::AttackId;

pub const HEALTH: f32 = 450.0;
pub const POISE: f32 = 150.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Waltz,
    Frenzy,
    Burnout,
}

impl Phase {
    pub fn at(health: f32) -> Self {
        if health < HEALTH * 0.25 {
            Self::Burnout
        } else if health <= HEALTH * 0.6 {
            Self::Frenzy
        } else {
            Self::Waltz
        }
    }
    pub fn phrase(self) -> &'static [AttackId] {
        use AttackId::*;
        match self {
            Self::Waltz => &[Combo1, Combo2],
            Self::Frenzy => &[Combo1, Combo3, Combo2, Combo3],
            Self::Burnout => &[Combo3, Combo1, Combo1, Combo2, Combo1, Combo1],
        }
    }
    pub fn next(self, beat: usize) -> AttackId {
        self.phrase()[beat % self.phrase().len()]
    }
    pub fn recovery(self, beat: usize) -> u32 {
        match self {
            Self::Waltz => 132,
            Self::Frenzy if beat % 2 == 1 => 144,
            Self::Burnout if beat % 3 == 2 => 216,
            _ => 18,
        }
    }
    pub fn regen(self) -> f32 {
        if self == Self::Burnout { 8.0 } else { 24.0 }
    }
}

pub fn finisher(health: f32) -> bool {
    health > 0.0 && health < HEALTH * 0.15
}

/// Vertical travel in one tick of the 48-pixel leap.
pub fn leap_step(tick: u32) -> f32 {
    let height = |t: f32| -4.0 * 48.0 * (t / 72.0) * (1.0 - t / 72.0);
    if tick >= 72 {
        0.0
    } else {
        height((tick + 1) as f32) - height(tick as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn leap_returns_to_floor_before_the_shockwave() {
        let rise: f32 = (0..36).map(leap_step).sum();
        let fall: f32 = (36..72).map(leap_step).sum();
        assert!((rise + 48.0).abs() < 0.001);
        assert!((rise + fall).abs() < 0.001);
        assert_eq!(leap_step(72), 0.0);
    }
    #[test]
    fn health_thresholds() {
        assert_eq!(Phase::at(450.0), Phase::Waltz);
        assert_eq!(Phase::at(270.0), Phase::Frenzy);
        assert_eq!(Phase::at(112.5), Phase::Frenzy);
        assert_eq!(Phase::at(112.0), Phase::Burnout);
        assert!(!finisher(67.5));
        assert!(finisher(67.0));
        assert!(!finisher(0.0));
    }
    #[test]
    fn phrases_have_openings_after_their_chains() {
        for phase in [Phase::Waltz, Phase::Frenzy, Phase::Burnout] {
            let n = phase.phrase().len();
            assert_eq!(phase.next(n), phase.next(0));
            assert!(phase.recovery(n - 1) >= 132);
        }
        assert!(!Phase::Waltz.phrase().contains(&AttackId::Combo3));
        assert_eq!(Phase::Frenzy.recovery(0), 18);
        assert_eq!(Phase::Burnout.recovery(1), 18);
        assert!(Phase::Burnout.recovery(2) > Phase::Frenzy.recovery(1));
        assert!(Phase::Burnout.regen() < Phase::Waltz.regen());
    }
    #[test]
    fn frame_data_is_valid_and_dash_is_jumpable() {
        let w =
            crate::combat::Weapon::from_ron(include_str!("../../../assets/combat/boss-dancer.ron"))
                .unwrap();
        assert!(w.combo[2].hitboxes.iter().all(|h| h.rect.y >= -18.0));
        assert!(
            w.combo[1]
                .hitboxes
                .iter()
                .all(|h| h.ticks.as_ref().unwrap().start >= 72)
        );
    }
}
