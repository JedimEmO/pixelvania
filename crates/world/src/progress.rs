//! What the player has found: abilities, weapons and upgrades, and which pickups are gone.
//!
//! Progress is one record, so a save is this record written out, the reachability checker
//! can walk the world with any set of abilities, and the fade can be read off it.
//! `docs/design/progression.md` sets the order abilities are found in and what each costs.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::controller::Abilities;

/// A movement ability, in the order the game gives them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Ability {
    Lunge,
    Cling,
    Swoon,
    Lure,
    Plunge,
    Submerge,
    Drift,
    Numb,
}

impl Ability {
    pub const ALL: [Ability; 8] = [
        Ability::Lunge,
        Ability::Cling,
        Ability::Swoon,
        Ability::Lure,
        Ability::Plunge,
        Ability::Submerge,
        Ability::Drift,
        Ability::Numb,
    ];

    /// How much colour finding this ability takes from the world. The steps add up to 1.
    pub fn fade_step(self) -> f32 {
        match self {
            Ability::Lunge => 0.02,
            Ability::Cling => 0.04,
            Ability::Swoon => 0.08,
            Ability::Lure => 0.12,
            Ability::Plunge => 0.14,
            Ability::Submerge => 0.16,
            Ability::Drift => 0.18,
            Ability::Numb => 0.26,
        }
    }

    /// The name used in room files and on screen.
    pub fn name(self) -> &'static str {
        match self {
            Ability::Lunge => "lunge",
            Ability::Cling => "cling",
            Ability::Swoon => "swoon",
            Ability::Lure => "lure",
            Ability::Plunge => "plunge",
            Ability::Submerge => "submerge",
            Ability::Drift => "drift",
            Ability::Numb => "numb",
        }
    }
}

/// A weapon the player can carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Weapon {
    Sabre,
    Jeroboam,
    Candelabrum,
}

impl Weapon {
    pub const ALL: [Weapon; 3] = [Weapon::Sabre, Weapon::Jeroboam, Weapon::Candelabrum];

    pub fn name(self) -> &'static str {
        match self {
            Weapon::Sabre => "sabre",
            Weapon::Jeroboam => "jeroboam",
            Weapon::Candelabrum => "candelabrum",
        }
    }
}

/// An upgrade hidden in the rooms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Upgrade {
    /// Raises health.
    Carnation,
    /// Raises how much Nectar can be carried.
    Goblet,
}

impl Upgrade {
    pub fn name(self) -> &'static str {
        match self {
            Upgrade::Carnation => "carnation",
            Upgrade::Goblet => "goblet",
        }
    }
}

/// Something a pickup gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Ability(Ability),
    Weapon(Weapon),
    Upgrade(Upgrade),
}

impl Item {
    /// Reads the `item=` of a pickup in a room file.
    pub fn parse(name: &str) -> Option<Item> {
        Ability::ALL
            .into_iter()
            .find(|a| a.name() == name)
            .map(Item::Ability)
            .or_else(|| {
                Weapon::ALL
                    .into_iter()
                    .find(|w| w.name() == name)
                    .map(Item::Weapon)
            })
            .or_else(|| {
                [Upgrade::Carnation, Upgrade::Goblet]
                    .into_iter()
                    .find(|u| u.name() == name)
                    .map(Item::Upgrade)
            })
    }

    pub fn name(self) -> &'static str {
        match self {
            Item::Ability(a) => a.name(),
            Item::Weapon(w) => w.name(),
            Item::Upgrade(u) => u.name(),
        }
    }
}

/// Everything the player has found.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Progress {
    pub abilities: BTreeSet<Ability>,
    pub weapons: BTreeSet<Weapon>,
    /// Pickups taken, as `room:item`, so they do not come back.
    pub taken: BTreeSet<String>,
    pub carnations: u32,
    pub goblets: u32,
    /// Floors broken by the Plunge, as `room:x,y` in tiles, so they stay broken.
    pub broken: BTreeSet<String>,
    /// Rooms the player has been in, for the map.
    pub visited: BTreeSet<String>,
}

impl Progress {
    /// A new game: the sabre and nothing else.
    pub fn new_game() -> Progress {
        Progress {
            weapons: [Weapon::Sabre].into(),
            ..Progress::default()
        }
    }

    /// The vertical slice's start: what a player has on reaching the ballroom.
    pub fn slice_start() -> Progress {
        Progress {
            abilities: [
                Ability::Lunge,
                Ability::Cling,
                Ability::Swoon,
                Ability::Lure,
            ]
            .into(),
            weapons: [Weapon::Sabre, Weapon::Jeroboam].into(),
            ..Progress::default()
        }
    }

    /// Everything, for testing.
    pub fn everything() -> Progress {
        Progress {
            abilities: Ability::ALL.into(),
            weapons: Weapon::ALL.into(),
            ..Progress::default()
        }
    }

    pub fn has(&self, a: Ability) -> bool {
        self.abilities.contains(&a)
    }

    /// The movement abilities the controller knows about.
    pub fn movement(&self) -> Abilities {
        Abilities {
            lunge: self.has(Ability::Lunge),
            swoon: self.has(Ability::Swoon),
            cling: self.has(Ability::Cling),
            lure: self.has(Ability::Lure),
            plunge: self.has(Ability::Plunge),
        }
    }

    /// Health with every Carnation found.
    pub fn max_health(&self) -> f32 {
        100.0 + 20.0 * self.carnations as f32
    }

    /// How much Nectar can be carried, with every Goblet found.
    pub fn nectar_capacity(&self) -> f32 {
        100.0 + 25.0 * self.goblets as f32
    }

    /// How much colour the abilities found have taken from the world, 0 to 1.
    pub fn fade(&self) -> f32 {
        self.abilities
            .iter()
            .map(|a| a.fade_step())
            .sum::<f32>()
            .min(1.0)
    }

    /// Takes a pickup's item. Returns false if this pickup was already taken.
    pub fn take(&mut self, room: &str, item: Item) -> bool {
        if !self.taken.insert(format!("{room}:{}", item.name())) {
            return false;
        }
        match item {
            Item::Ability(a) => self.abilities.insert(a),
            Item::Weapon(w) => self.weapons.insert(w),
            Item::Upgrade(Upgrade::Carnation) => {
                self.carnations += 1;
                true
            }
            Item::Upgrade(Upgrade::Goblet) => {
                self.goblets += 1;
                true
            }
        };
        true
    }

    /// Records a broken floor tile.
    pub fn broke(&mut self, room: &str, x: i32, y: i32) {
        self.broken.insert(format!("{room}:{x},{y}"));
    }

    /// The tiles broken in a room.
    pub fn broken_in<'a>(&'a self, room: &'a str) -> impl Iterator<Item = (i32, i32)> + 'a {
        self.broken.iter().filter_map(move |k| {
            let (r, xy) = k.split_once(':')?;
            let (x, y) = xy.split_once(',')?;
            (r == room).then_some((x.parse().ok()?, y.parse().ok()?))
        })
    }

    pub fn taken(&self, room: &str, item: Item) -> bool {
        self.taken.contains(&format!("{room}:{}", item.name()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fade_steps_add_up_to_all_the_colour() {
        let total: f32 = Ability::ALL.iter().map(|a| a.fade_step()).sum();
        assert!((total - 1.0).abs() < 1e-5, "{total}");
        assert!((Progress::slice_start().fade() - 0.26).abs() < 1e-5);
        assert_eq!(Progress::new_game().fade(), 0.0);
    }

    #[test]
    fn a_pickup_is_taken_once() {
        let mut p = Progress::new_game();
        let lure = Item::parse("lure").unwrap();
        assert!(p.take("gardens", lure));
        assert!(!p.take("gardens", lure));
        assert!(p.movement().lure && p.taken("gardens", lure));
        assert_eq!(
            Item::parse("candelabrum"),
            Some(Item::Weapon(Weapon::Candelabrum))
        );
        assert_eq!(Item::parse("cake"), None);
    }

    #[test]
    fn progress_survives_a_save() {
        let mut p = Progress::slice_start();
        p.take("ballroom", Item::Weapon(Weapon::Candelabrum));
        let text = ron::to_string(&p).unwrap();
        assert_eq!(ron::from_str::<Progress>(&text).unwrap(), p);
    }
}
