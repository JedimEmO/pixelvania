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
    /// One more Cordial between rests.
    Goblet,
    /// The Host's favour, spent at a lounge on a level.
    Favour,
}

impl Upgrade {
    pub const ALL: [Upgrade; 3] = [Upgrade::Carnation, Upgrade::Goblet, Upgrade::Favour];

    pub fn name(self) -> &'static str {
        match self {
            Upgrade::Carnation => "carnation",
            Upgrade::Goblet => "goblet",
            Upgrade::Favour => "favour",
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
                Upgrade::ALL
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

/// A stat raised at a lounge with a Favour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stat {
    /// More health.
    Vitality,
    /// More poise, so blows stagger less.
    Composure,
    /// Harder blows: more damage and poise damage.
    Strength,
    /// Longer Highs and stronger Cordials.
    Appetite,
}

impl Stat {
    pub const ALL: [Stat; 4] = [
        Stat::Vitality,
        Stat::Composure,
        Stat::Strength,
        Stat::Appetite,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Stat::Vitality => "Vitality",
            Stat::Composure => "Composure",
            Stat::Strength => "Strength",
            Stat::Appetite => "Appetite",
        }
    }

    /// What a level of it does, for the lounge's menu.
    pub fn effect(self) -> &'static str {
        match self {
            Stat::Vitality => "+12 health",
            Stat::Composure => "+8 poise",
            Stat::Strength => "+6% to every blow",
            Stat::Appetite => "+8% to a High, +10% to a Cordial",
        }
    }
}

/// The levels bought in each stat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Stats {
    pub vitality: u32,
    pub composure: u32,
    pub strength: u32,
    pub appetite: u32,
}

impl Stats {
    pub fn get(&self, s: Stat) -> u32 {
        match s {
            Stat::Vitality => self.vitality,
            Stat::Composure => self.composure,
            Stat::Strength => self.strength,
            Stat::Appetite => self.appetite,
        }
    }

    fn get_mut(&mut self, s: Stat) -> &mut u32 {
        match s {
            Stat::Vitality => &mut self.vitality,
            Stat::Composure => &mut self.composure,
            Stat::Strength => &mut self.strength,
            Stat::Appetite => &mut self.appetite,
        }
    }

    /// Every level bought.
    pub fn level(&self) -> u32 {
        Stat::ALL.iter().map(|&s| self.get(s)).sum()
    }

    /// Multiplier on outgoing damage and poise damage.
    pub fn strength_scale(&self) -> f32 {
        1.0 + 0.06 * self.strength as f32
    }

    /// Multiplier on a High's length and on a heal.
    pub fn high_scale(&self) -> f32 {
        1.0 + 0.08 * self.appetite as f32
    }

    pub fn heal_scale(&self) -> f32 {
        1.0 + 0.1 * self.appetite as f32
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
    /// Favours found and not yet spent on a level.
    pub favours: u32,
    /// Floors broken by the Plunge, as `room:x,y` in tiles, so they stay broken.
    pub broken: BTreeSet<String>,
    /// Rooms the player has been in, for the map.
    pub visited: BTreeSet<String>,
    /// Levels bought with Favours.
    pub stats: Stats,
    /// Seconds played, not counting paused time.
    pub seconds: f64,
    /// Times the player blacked out.
    pub blackouts: u32,
    /// The slice's end has been seen, so it does not play again.
    pub ended: bool,
    /// Bosses beaten, by kind, so they stay beaten.
    pub beaten: BTreeSet<String>,
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

    /// Health with every Carnation found and every level of Vitality.
    pub fn max_health(&self) -> f32 {
        100.0 + 20.0 * self.carnations as f32 + 12.0 * self.stats.vitality as f32
    }

    /// Poise with every level of Composure.
    pub fn max_poise(&self) -> f32 {
        60.0 + 8.0 * self.stats.composure as f32
    }

    /// Spends a Favour on a level in a stat. False if there is no Favour to spend.
    pub fn level_up(&mut self, stat: Stat) -> bool {
        if self.favours == 0 {
            return false;
        }
        self.favours -= 1;
        *self.stats.get_mut(stat) += 1;
        true
    }

    /// How much Nectar can be carried.
    pub fn nectar_capacity(&self) -> f32 {
        100.0
    }

    /// Cordials carried between rests, with every Goblet found.
    pub fn cordials(&self) -> u32 {
        3 + self.goblets
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
            Item::Upgrade(Upgrade::Favour) => {
                self.favours += 1;
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
        assert_eq!(Item::parse("favour"), Some(Item::Upgrade(Upgrade::Favour)));
    }

    #[test]
    fn a_favour_buys_one_level() {
        let mut p = Progress::new_game();
        assert!(!p.level_up(Stat::Vitality));
        p.take("cellar", Item::Upgrade(Upgrade::Favour));
        assert_eq!(p.favours, 1);
        assert!(p.level_up(Stat::Vitality));
        assert_eq!((p.favours, p.stats.vitality), (0, 1));
        assert!(!p.level_up(Stat::Strength));
    }

    #[test]
    fn levels_raise_the_stats() {
        let mut p = Progress::new_game();
        p.favours = 2;
        assert!(p.level_up(Stat::Vitality));
        assert!(p.level_up(Stat::Strength));
        assert_eq!(p.stats.level(), 2);
        assert_eq!(p.max_health(), 112.0);
        assert!((p.stats.strength_scale() - 1.06).abs() < 1e-6);
    }

    #[test]
    fn progress_survives_a_save() {
        let mut p = Progress::slice_start();
        p.take("ballroom", Item::Weapon(Weapon::Candelabrum));
        let text = ron::to_string(&p).unwrap();
        assert_eq!(ron::from_str::<Progress>(&text).unwrap(), p);
    }
}
