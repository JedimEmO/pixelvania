//! The Sleeper: the gardens' boss, asleep in a bed of flowers in his bower.
//!
//! He is a guest of his own class, so he moves, is hit, staggers and falls as guests do,
//! and this module is his brain. He starts asleep, and stays asleep until a blow wakes him,
//! startled, into a thrash. Then he fights in three phases, each a phrase of moves:
//! light sleep (above 60% health), thrashes and lying down again; restless, which adds the vines;
//! and nightmare (under 25%), when he no longer sleeps at all.
//!
//! Each move has its own telegraph. The thrash starts with twitching, arms drawn in,
//! and flails out both ways at once. The vines start with his arms raised, trembling,
//! and the floor glowing; then they come up through the whole floor for longer than a jump lasts,
//! so the player hangs from a bloom until they sink. That is what he teaches.
//! Asleep he is open to blows, until enough of them wake him into a thrash.
//!
//! Beaten, he stays beaten: the progress record keeps him, and his bower is quiet after.

use anim::guest::{Class, SleeperMove};
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use world::Controls;
use world::combat::{AttackId, Phase as AttackPhase, Reaction};

use crate::boss::{Boss, BossDefeated};
use crate::combat::{Fighting, SimulateBodies, WeaponAsset, Weapons, combat_id};
use crate::enemies::Guest;
use crate::player::Player;
use crate::rooms::ActiveRoom;

pub struct SleeperPlugin;

impl Plugin for SleeperPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            tick.in_set(SimulateBodies).after(crate::player::PlayerTick),
        )
        .add_systems(Update, (spawn, draw_vines).chain());
    }
}

/// His kind, in the progress record and in `assets/story.ron`.
const KIND: &str = "sleeper";
/// How near the player must be for a thrash, in pixels; further, he sleepwalks up first.
const THRASH_REACH: f32 = 70.0;
/// Ticks he stands still after a phase begins.
const PHASE_PAUSE: u32 = 100;
/// Below this share of his health, a blow on him staggered finishes him.
const FINISH_AT: f32 = 0.12;
/// Blows that wake him from a sleep he lay down for.
const WAKE_HITS: u32 = 3;
/// The windup of the thrash he wakes into.
const STARTLE: u32 = 18;
/// The first sleep, which only a blow ends.
const FIRST_SLEEP: u32 = 1_000_000;

/// The fight's phases.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    LightSleep,
    Restless,
    Nightmare,
}

impl Phase {
    fn at(share: f32) -> Self {
        if share < 0.25 {
            Self::Nightmare
        } else if share <= 0.6 {
            Self::Restless
        } else {
            Self::LightSleep
        }
    }

    fn number(self) -> u8 {
        match self {
            Self::LightSleep => 1,
            Self::Restless => 2,
            Self::Nightmare => 3,
        }
    }

    /// The phrase of moves he repeats in this phase.
    fn phrase(self) -> &'static [SleeperMove] {
        use SleeperMove::*;
        match self {
            Self::LightSleep => &[Thrash, Thrash, Sleep],
            Self::Restless => &[Vines, Thrash, Thrash, Sleep],
            Self::Nightmare => &[Vines, Thrash, Vines, Thrash],
        }
    }

    /// Ticks to recover after a move: long at first, short in the nightmare.
    fn recovery(self, m: SleeperMove) -> u32 {
        match (self, m) {
            (_, SleeperMove::Sleep) => 50,
            (Self::LightSleep, _) => 70,
            (Self::Restless, _) => 56,
            (Self::Nightmare, _) => 34,
        }
    }

    /// Windups shorten as the dream gets worse, but never below a readable minimum.
    fn windup(self, ticks: u32) -> u32 {
        match self {
            Self::LightSleep => ticks,
            Self::Restless => (ticks * 9 / 10).max(40),
            Self::Nightmare => (ticks * 3 / 4).max(36),
        }
    }
}

fn attack_of(m: SleeperMove) -> AttackId {
    match m {
        SleeperMove::Thrash => AttackId::Combo1,
        SleeperMove::Vines => AttackId::Combo2,
        SleeperMove::Sleep => AttackId::Charged,
    }
}

/// Which of his moves an attack is, for drawing him.
pub fn move_of(attack: Option<AttackId>) -> Option<SleeperMove> {
    match attack? {
        AttackId::Combo1 => Some(SleeperMove::Thrash),
        AttackId::Combo2 => Some(SleeperMove::Vines),
        AttackId::Charged => Some(SleeperMove::Sleep),
        _ => None,
    }
}

/// His move's frame data in his weapon.
fn data_of(weapon: &mut world::combat::Weapon, m: SleeperMove) -> &mut world::combat::AttackData {
    match m {
        SleeperMove::Thrash => &mut weapon.combo[0],
        SleeperMove::Vines => &mut weapon.combo[1],
        SleeperMove::Sleep => &mut weapon.charged,
    }
}

/// The Sleeper's brain, on a guest of the Sleeper class.
#[derive(Component)]
pub struct Sleeper {
    phase: Phase,
    /// The move in the phrase he is on.
    beat: usize,
    /// Ticks he waits before choosing a move.
    pause: u32,
    /// He has lain down for his first sleep.
    lain: bool,
    /// He has been woken, so the fight is on.
    engaged: bool,
    announced: bool,
    /// Blows taken during this sleep.
    blows: u32,
    /// His health at the end of the last tick, to notice blows while he sleeps.
    last_health: f32,
}

/// The vines along the floor.
#[derive(Component)]
struct FloorVines;

/// The room build he was last spawned for.
#[derive(Default)]
struct SpawnedFor(Option<u64>);

fn spawn(
    mut commands: Commands,
    active: Res<ActiveRoom>,
    progress: Res<crate::progress::PlayerProgress>,
    weapons: Option<Res<Weapons>>,
    mut images: ResMut<Assets<Image>>,
    mut spawned: Local<SpawnedFor>,
) {
    let (Some(room), Some(weapons)) = (&active.room, weapons) else {
        return;
    };
    if spawned.0 == Some(active.build) {
        return;
    }
    spawned.0 = Some(active.build);
    if progress.0.beaten.contains(KIND) {
        return;
    }
    for def in room.entities_of("boss") {
        if def.props.get("kind").map(String::as_str) != Some(KIND) {
            continue;
        }
        let e = crate::enemies::spawn_guest(
            &mut commands,
            Class::Sleeper,
            def.pixel_pos(),
            &weapons,
            &mut images,
        );
        commands.entity(e).insert((
            Name::new("The Sleeper"),
            Sleeper {
                phase: Phase::LightSleep,
                beat: 0,
                pause: 0,
                lain: false,
                engaged: false,
                announced: false,
                blows: 0,
                last_health: 600.0,
            },
            Boss {
                name: "THE SLEEPER",
                kind: KIND,
                music: "sleeper",
                max_health: 600.0,
                phase: 1,
                finishable: false,
            },
        ));
        commands.entity(e).with_child((
            FloorVines,
            Sprite::from_color(Color::srgb(0.45, 1.0, 0.7), Vec2::new(8.0, 3.0)),
            Transform::from_xyz(0.0, 2.0, 0.2),
            Visibility::Hidden,
            RenderLayers::layer(crate::render::GLOW_LAYER),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn tick(
    active: Res<ActiveRoom>,
    tuning: Res<crate::tuning::Tuning>,
    weapons: Res<Assets<WeaponAsset>>,
    players: Query<(&Player, &Fighting), Without<Sleeper>>,
    mut sleepers: Query<
        (Entity, &mut Guest, &mut Fighting, &mut Sleeper, &mut Boss),
        Without<Player>,
    >,
    mut log: ResMut<crate::combat::CombatLog>,
    mut shake: ResMut<crate::camera::Shake>,
    mut fx: MessageWriter<crate::effects::SpawnFx>,
    mut defeated: MessageWriter<BossDefeated>,
    mut sfx: MessageWriter<crate::audio::PlaySfx>,
) {
    use crate::audio::{PlaySfx, Sfx};
    let Some(room) = &active.room else { return };
    let target = players
        .iter()
        .find(|(_, f)| f.combat.health > 0.0)
        .map(|(p, _)| {
            let b = &p.ctrl.body;
            Vec2::new(b.x as f32 + b.w as f32 / 2.0, (b.y + b.h) as f32)
        });
    for (entity, mut g, mut f, mut d, mut boss) in &mut sleepers {
        let Some(asset) = weapons.get(&f.weapon) else {
            continue;
        };
        g.prev = g.feet();
        let mut weapon = asset.0.clone();
        let feet = g.feet();
        let mut controls = Controls::default();

        // At first he lies asleep, for as long as nobody touches him.
        if !d.lain && f.combat.health > 0.0 {
            d.lain = true;
            let data = data_of(&mut weapon, SleeperMove::Sleep);
            data.windup = 1;
            data.active = FIRST_SLEEP;
            f.combat
                .attacker
                .start(&weapon, AttackId::Charged, g.ctrl.facing);
        }

        // Blows while he sleeps: the first ever wakes him; later, enough of them do.
        let sleeping = move_of(f.combat.attacker.attack()) == Some(SleeperMove::Sleep)
            && f.combat.attacker.phase() != AttackPhase::Recovery;
        if sleeping && f.combat.health < d.last_health - 0.01 && f.combat.health > 0.0 {
            d.blows += 1;
            if !d.engaged || d.blows >= WAKE_HITS {
                d.engaged = true;
                d.blows = 0;
                d.pause = 0;
                f.combat.attacker.interrupt();
                if let Some(p) = target {
                    g.ctrl.facing = if p.x < feet.x { -1 } else { 1 };
                }
                let data = data_of(&mut weapon, SleeperMove::Thrash);
                data.windup = STARTLE;
                data.recovery = d.phase.recovery(SleeperMove::Thrash);
                f.combat
                    .attacker
                    .start(&weapon, AttackId::Combo1, g.ctrl.facing);
                f.flash = 0.2;
                shake.kick(Vec2::Y, 3.0);
                sfx.write(PlaySfx {
                    pitch: 1.3,
                    ..PlaySfx::new(Sfx::GuestWindup)
                });
            }
        }

        if !f.combat.is_frozen() && f.combat.health > 0.0 && d.engaged {
            let phase = Phase::at(f.combat.health / boss.max_health);
            if phase != d.phase {
                d.phase = phase;
                boss.phase = phase.number();
                d.beat = 0;
                d.pause = PHASE_PAUSE;
                f.combat.stagger();
                f.flash = 0.22;
                sfx.write(PlaySfx {
                    pitch: 0.8,
                    ..PlaySfx::new(Sfx::DancerPhase)
                });
                shake.kick(Vec2::Y, 3.0);
            }
            d.pause = d.pause.saturating_sub(1);
            if d.pause == 0
                && !f.combat.helpless()
                && f.combat.attacker.attack().is_none()
                && let Some(p) = target
            {
                g.ctrl.facing = if p.x < feet.x { -1 } else { 1 };
                let m = d.phase.phrase()[d.beat % d.phase.phrase().len()];
                if m == SleeperMove::Thrash && (p.x - feet.x).abs() > THRASH_REACH {
                    controls.x = g.ctrl.facing as f32;
                } else {
                    let data = data_of(&mut weapon, m);
                    data.recovery = d.phase.recovery(m);
                    if m != SleeperMove::Sleep {
                        data.windup = d.phase.windup(data.windup);
                    }
                    f.combat
                        .attacker
                        .start(&weapon, attack_of(m), g.ctrl.facing);
                    d.beat += 1;
                    d.blows = 0;
                    sfx.write(match m {
                        SleeperMove::Thrash => PlaySfx {
                            pitch: 1.1,
                            ..PlaySfx::new(Sfx::GuestWindup)
                        },
                        SleeperMove::Vines => PlaySfx {
                            pitch: 0.5,
                            ..PlaySfx::new(Sfx::DancerPhase)
                        },
                        SleeperMove::Sleep => PlaySfx {
                            pitch: 0.6,
                            volume: 0.6,
                            ..PlaySfx::new(Sfx::LureRelease)
                        },
                    });
                }
            }
        }

        let tuning_now = world::MoveTuning {
            run_speed: 55.0,
            ..tuning.movement.clone()
        };
        let (_, events) = f.combat.tick_controller(
            combat_id(entity),
            &mut g.ctrl,
            room,
            &tuning_now,
            &weapon,
            controls,
        );
        log.0.extend(events);

        if !f.combat.is_frozen() && !f.combat.helpless() {
            let a = &f.combat.attacker;
            let (phase, tick) = (a.phase(), a.phase_tick());
            match (move_of(a.attack()), phase) {
                (Some(SleeperMove::Vines), AttackPhase::Active) if tick == 6 => {
                    // Up through the floor, all at once.
                    shake.kick(Vec2::Y, 5.0);
                    sfx.write(PlaySfx {
                        pitch: 0.5,
                        ..PlaySfx::new(Sfx::WallSplat)
                    });
                    for dx in [-300.0, -180.0, -60.0, 60.0, 180.0, 300.0] {
                        fx.write(crate::effects::SpawnFx::dust(
                            Vec2::new(feet.x + dx, -feet.y),
                            pixelfx::effects::DustVariant::Land,
                        ));
                    }
                }
                (Some(SleeperMove::Vines), AttackPhase::Windup) if tick % 30 == 29 => {
                    shake.kick(Vec2::Y, 0.8);
                }
                _ => {}
            }
        }
        crate::combat::fall_dead(&f, &mut g.ctrl, room, &tuning.movement);
        boss.finishable = f.combat.health < boss.max_health * FINISH_AT;
        d.last_health = f.combat.health;
        if f.combat.reaction() == Reaction::Dead && !d.announced {
            d.announced = true;
            defeated.write(BossDefeated {
                kind: KIND.into(),
                at: Vec2::new(feet.x, -feet.y),
            });
        }
    }
}

/// The vines along the floor: a faint glow while he calls them, then the whole floor.
fn draw_vines(
    sleepers: Query<&Fighting, With<Sleeper>>,
    mut vines: Query<(&ChildOf, &mut Visibility, &mut Sprite), With<FloorVines>>,
) {
    for (parent, mut visible, mut sprite) in &mut vines {
        let Ok(f) = sleepers.get(parent.parent()) else {
            continue;
        };
        let a = &f.combat.attacker;
        let calling = move_of(a.attack()) == Some(SleeperMove::Vines);
        let (phase, tick) = (a.phase(), a.phase_tick());
        let (on, height, alpha) = match phase {
            AttackPhase::Windup if calling => (true, 2.0, 0.2 + 0.2 * (tick as f32 * 0.25).sin()),
            AttackPhase::Active if calling => {
                let sink = (tick as f32 / 150.0).powi(3);
                (true, 10.0 * (1.0 - sink) + 2.0, 1.0 - 0.5 * sink)
            }
            _ => (false, 0.0, 0.0),
        };
        visible.set_if_neq(if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if on {
            sprite.custom_size = Some(Vec2::new(840.0, height));
            sprite.color = Color::srgba(0.45, 1.0, 0.7, alpha);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_phases_follow_his_health_and_the_nightmare_never_sleeps() {
        assert_eq!(Phase::at(1.0), Phase::LightSleep);
        assert_eq!(Phase::at(0.6), Phase::Restless);
        assert_eq!(Phase::at(0.24), Phase::Nightmare);
        assert!(!Phase::LightSleep.phrase().contains(&SleeperMove::Vines));
        assert!(Phase::Restless.phrase().contains(&SleeperMove::Vines));
        assert!(!Phase::Nightmare.phrase().contains(&SleeperMove::Sleep));
        for phase in [Phase::LightSleep, Phase::Restless, Phase::Nightmare] {
            assert!(phase.windup(66) >= 36, "every windup stays readable");
            for m in phase.phrase() {
                assert_eq!(move_of(Some(attack_of(*m))), Some(*m));
            }
        }
    }

    #[test]
    fn his_vines_outlast_a_jump() {
        let weapon = world::combat::Weapon::from_ron(include_str!(
            "../../../assets/combat/boss-sleeper.ron"
        ))
        .unwrap();
        let vines = weapon.attack(AttackId::Combo2);
        let window = vines.hitboxes[0].ticks.unwrap();
        // A jump lasts about two thirds of a second; the vines stay up longer.
        assert!(window.end - window.start > 120);
        assert!(weapon.attack(AttackId::Charged).hitboxes.is_empty());
    }
}
