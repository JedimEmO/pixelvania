//! Rooms, the world, and movement through them.
//!
//! This crate has no Bevy dependency, so its tests run in milliseconds
//! and the offline tools can use it without starting the engine.

pub mod body;
pub mod combat;
pub mod controller;
pub mod doors;
pub mod progress;
pub mod projectile;
pub mod reach;
pub mod room;

pub use body::Body;
pub use controller::{Abilities, Controller, Controls, Events, MoveTuning, MovementLock, Rope};
pub use doors::{Door, Side, World, WorldRoom};
pub use progress::{Ability, Item, Progress};
pub use room::{EntityDef, ParseError, Room, TILE, Tile};
