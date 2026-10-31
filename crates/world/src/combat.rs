//! Deterministic combat at 120 ticks per second. Positions are pixels, y down.
//!
//! Call [`Combatant::tick_controller`] for every fighter, then [`resolve`], then
//! [`resolve_body_collisions`]. Build [`Fighter`] views after movement with world-space
//! hurtboxes. Stable, unique IDs make resolution independent of slice order.
//! Events carry the impact data the renderer needs; this crate does not draw or play sound.

mod attacker;
mod combatant;
mod data;
mod resolve;

pub use attacker::{AttackMotion, Attacker, Phase};
pub use combatant::{Combatant, Reaction, WeightClass};
pub use data::{AttackData, AttackId, Chain, Hitbox, Rect, TickRange, Weapon, Weight};
pub use resolve::{CombatEvent, Fighter, Hit, ImpactScale, resolve, resolve_body_collisions};

pub const TICK_RATE: u32 = 120;
pub const DT: f32 = 1.0 / TICK_RATE as f32;
pub const BUFFER_TICKS: u32 = 18;
pub const PARRY_SECONDS: f32 = 0.12;
pub const WALL_SPLAT_SPEED: f32 = 180.0;
pub const WALL_SPLAT_DAMAGE: f32 = 12.0;
pub const POGO_SPEED: f32 = 340.0;

/// Rounds durations up to whole ticks, so a freeze never ends early.
pub fn seconds_to_ticks(seconds: f32) -> u32 {
    (seconds * TICK_RATE as f32 - 0.00001).ceil().max(0.0) as u32
}
