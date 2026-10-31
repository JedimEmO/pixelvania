use serde::{Deserialize, Serialize};

use crate::Body;

/// An axis-aligned rectangle. Touching edges do not overlap.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn from_body(body: &Body) -> Self {
        Self {
            x: body.x as f32,
            y: body.y as f32,
            w: body.w as f32,
            h: body.h as f32,
        }
    }

    pub fn overlaps(self, other: Self) -> bool {
        self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }

    /// Places a right-facing local rectangle at the feet, mirroring it when facing left.
    pub fn at_feet(self, feet: (f32, f32), facing: i32) -> Self {
        Self {
            x: feet.0 + if facing < 0 { -self.x - self.w } else { self.x },
            y: feet.1 + self.y,
            ..self
        }
    }

    fn valid(self) -> bool {
        [self.x, self.y, self.w, self.h]
            .iter()
            .all(|n| n.is_finite())
            && self.w > 0.0
            && self.h > 0.0
    }
}

/// A zero-based, half-open range: `start` is included, `end` is excluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TickRange {
    pub start: u32,
    pub end: u32,
}

impl TickRange {
    pub fn contains(self, tick: u32) -> bool {
        (self.start..self.end).contains(&tick)
    }
    fn valid(self, duration: u32) -> bool {
        self.start < self.end && self.end <= duration
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Weight {
    Light,
    Heavy,
    Charged,
}

/// A slot in a weapon. A swing keeps a copy of its data until it ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttackId {
    Combo1,
    Combo2,
    Combo3,
    Charged,
    Rising,
    Air1,
    Air2,
    Pogo,
}

impl AttackId {
    pub const ALL: [Self; 8] = [
        Self::Combo1,
        Self::Combo2,
        Self::Combo3,
        Self::Charged,
        Self::Rising,
        Self::Air1,
        Self::Air2,
        Self::Pogo,
    ];
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hitbox {
    /// Relative to the feet-centre, authored facing right.
    pub rect: Rect,
    /// Active-phase ticks. `None` means the whole active phase.
    pub ticks: Option<TickRange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chain {
    pub next: AttackId,
    /// Recovery ticks in which a press selects `next`.
    pub window: TickRange,
}

/// Frame data for one move. Durations are ticks; hit-stop is seconds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttackData {
    pub windup: u32,
    pub active: u32,
    pub recovery: u32,
    /// First recovery tick that permits a Lunge after connecting.
    pub cancel_window: u32,
    pub hitboxes: Vec<Hitbox>,
    pub damage: f32,
    pub poise_damage: f32,
    pub hit_stop: f32,
    /// Right-facing velocity, pixels per second. Negative y launches upwards.
    pub knockback: (f32, f32),
    pub launches: bool,
    pub weight: Weight,
    pub armour: bool,
    /// Forward velocity throughout windup and active; recovery has no step.
    pub step: Option<f32>,
    pub chain: Option<Chain>,
    /// Ticks held at zero vertical speed, counted from the start of the move.
    pub air_hang: u32,
}

impl AttackData {
    pub fn total_ticks(&self) -> u32 {
        self.windup + self.active + self.recovery
    }

    fn validate(&self) -> Result<(), String> {
        if self.windup == 0
            || self.active == 0
            || self.recovery == 0
            || self
                .windup
                .checked_add(self.active)
                .and_then(|n| n.checked_add(self.recovery))
                .is_none()
        {
            return Err("phase durations must be positive and their sum must fit u32".into());
        }
        if self.cancel_window >= self.recovery {
            return Err("cancel window is outside recovery".into());
        }
        if self.air_hang > self.total_ticks() {
            return Err("air hang exceeds the move".into());
        }
        if self.hitboxes.is_empty()
            || self
                .hitboxes
                .iter()
                .any(|h| !h.rect.valid() || h.ticks.is_some_and(|r| !r.valid(self.active)))
        {
            return Err("hitboxes need finite positive rectangles and valid active ranges".into());
        }
        if self
            .chain
            .as_ref()
            .is_some_and(|c| !c.window.valid(self.recovery))
        {
            return Err("chain window is outside recovery".into());
        }
        if [
            self.damage,
            self.poise_damage,
            self.hit_stop,
            self.knockback.0,
        ]
        .iter()
        .any(|n| !n.is_finite() || *n < 0.0)
            || !self.knockback.1.is_finite()
            || self.step.is_some_and(|n| !n.is_finite() || n < 0.0)
        {
            return Err("impact values must be finite, with non-negative magnitudes".into());
        }
        Ok(())
    }
}

/// The eight moves of a weapon. Parse and validate before making it available to fighters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Weapon {
    pub name: String,
    pub charge_ticks: u32,
    pub combo: [AttackData; 3],
    pub charged: AttackData,
    pub rising: AttackData,
    pub air: [AttackData; 2],
    pub pogo: AttackData,
}

impl Weapon {
    pub fn from_ron(source: &str) -> Result<Self, String> {
        let weapon: Self = ron::from_str(source).map_err(|e| e.to_string())?;
        weapon.validate()?;
        Ok(weapon)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() || self.charge_ticks == 0 {
            return Err("a weapon needs a name and a positive charge threshold".into());
        }
        for id in AttackId::ALL {
            self.attack(id)
                .validate()
                .map_err(|e| format!("{id:?}: {e}"))?;
        }
        Ok(())
    }

    pub fn attack(&self, id: AttackId) -> &AttackData {
        match id {
            AttackId::Combo1 => &self.combo[0],
            AttackId::Combo2 => &self.combo[1],
            AttackId::Combo3 => &self.combo[2],
            AttackId::Charged => &self.charged,
            AttackId::Rising => &self.rising,
            AttackId::Air1 => &self.air[0],
            AttackId::Air2 => &self.air[1],
            AttackId::Pogo => &self.pogo,
        }
    }
}
