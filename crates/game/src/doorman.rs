//! The Doorman: the Arrival's boss, who keeps the inner doors.
//!
//! He is a guest of his own class, so he moves, is hit, staggers and falls as guests do,
//! and this module is his brain. He fights in three phases, each a phrase of moves:
//! the list (above 60% health), shoves and knocks; impatience, which adds the rush;
//! and closing time (under 25%), where the moves come back to back.
//!
//! Each move has its own telegraph. The shove draws the staff back level, the knock raises it high
//! and sends a line along the floor to jump, and the rush starts with two stamps.
//! The rush is his body, too tall to jump, and the Lunge passes through it; that is what he teaches.
//! A rush that ends in a wall staggers him, which is the opening it leaves.
//!
//! Beaten, he stays beaten: the progress record keeps him, and his hall is quiet after.

use anim::guest::{Class, DoormanMove};
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use world::Controls;
use world::combat::{AttackId, Phase as AttackPhase, Reaction};

use crate::boss::{Boss, BossDefeated};
use crate::combat::{Fighting, SimulateBodies, WeaponAsset, Weapons, combat_id};
use crate::enemies::Guest;
use crate::player::Player;
use crate::rooms::ActiveRoom;

pub struct DoormanPlugin;

impl Plugin for DoormanPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            tick.in_set(SimulateBodies).after(crate::player::PlayerTick),
        )
        .add_systems(Update, (spawn, draw_line).chain());
    }
}

/// His kind, in the progress record and in `assets/story.ron`.
const KIND: &str = "doorman";
/// How near the player must be for him to start, in pixels.
const NOTICE: f32 = 320.0;
/// How near the player must be for a shove, in pixels; further, he walks up first.
const SHOVE_REACH: f32 = 66.0;
/// The rush's speed, in pixels per second.
const RUSH_SPEED: f32 = 330.0;
/// Ticks he stands still after a phase begins, and at the start.
const PHASE_PAUSE: u32 = 110;
/// Below this share of his health, a blow on him staggered finishes him.
const FINISH_AT: f32 = 0.12;

/// The fight's phases.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    List,
    Impatience,
    ClosingTime,
}

impl Phase {
    fn at(share: f32) -> Self {
        if share < 0.25 {
            Self::ClosingTime
        } else if share <= 0.6 {
            Self::Impatience
        } else {
            Self::List
        }
    }

    fn number(self) -> u8 {
        match self {
            Self::List => 1,
            Self::Impatience => 2,
            Self::ClosingTime => 3,
        }
    }

    /// The phrase of moves he repeats in this phase.
    fn phrase(self) -> &'static [DoormanMove] {
        use DoormanMove::*;
        match self {
            Self::List => &[Shove, Shove, Knock],
            Self::Impatience => &[Shove, Rush, Knock, Rush],
            Self::ClosingTime => &[Rush, Knock, Shove, Rush, Knock],
        }
    }

    /// Ticks to recover after a move: long at first, short at closing time.
    fn recovery(self, m: DoormanMove) -> u32 {
        match (self, m) {
            (Self::List, _) => 90,
            (Self::Impatience, DoormanMove::Rush) => 80,
            (Self::Impatience, _) => 60,
            (Self::ClosingTime, DoormanMove::Rush) => 50,
            (Self::ClosingTime, _) => 30,
        }
    }

    /// Windups shorten as he loses patience, but never below a readable minimum.
    fn windup(self, ticks: u32) -> u32 {
        match self {
            Self::List => ticks,
            Self::Impatience => (ticks * 9 / 10).max(40),
            Self::ClosingTime => (ticks * 3 / 4).max(36),
        }
    }
}

fn attack_of(m: DoormanMove) -> AttackId {
    match m {
        DoormanMove::Shove => AttackId::Combo1,
        DoormanMove::Knock => AttackId::Combo2,
        DoormanMove::Rush => AttackId::Combo3,
    }
}

/// Which of his moves an attack is, for drawing him.
pub fn move_of(attack: Option<AttackId>) -> Option<DoormanMove> {
    match attack? {
        AttackId::Combo1 => Some(DoormanMove::Shove),
        AttackId::Combo2 => Some(DoormanMove::Knock),
        AttackId::Combo3 => Some(DoormanMove::Rush),
        _ => None,
    }
}

/// The Doorman's brain, on a guest of the Doorman class.
#[derive(Component)]
pub struct Doorman {
    phase: Phase,
    /// The move in the phrase he is on.
    beat: usize,
    /// Ticks he waits before choosing a move.
    pause: u32,
    /// The player has come near, so the fight is on.
    engaged: bool,
    announced: bool,
}

/// The line the knock sends along the floor.
#[derive(Component)]
struct FloorLine;

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
            Class::Doorman,
            def.pixel_pos(),
            &weapons,
            &mut images,
        );
        commands.entity(e).insert((
            Name::new("The Doorman"),
            Doorman {
                phase: Phase::List,
                beat: 0,
                pause: PHASE_PAUSE,
                engaged: false,
                announced: false,
            },
            Boss {
                name: "THE DOORMAN",
                kind: KIND,
                music: "doorman",
                max_health: 560.0,
                phase: 1,
                finishable: false,
            },
        ));
        commands.entity(e).with_child((
            FloorLine,
            Sprite::from_color(Color::srgb(1.0, 0.8, 0.5), Vec2::new(8.0, 2.0)),
            Transform::from_xyz(0.0, 1.0, 0.2),
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
    players: Query<(&Player, &Fighting), Without<Doorman>>,
    mut doormen: Query<
        (Entity, &mut Guest, &mut Fighting, &mut Doorman, &mut Boss),
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
    let dt = world::combat::DT;
    for (entity, mut g, mut f, mut d, mut boss) in &mut doormen {
        let Some(asset) = weapons.get(&f.weapon) else {
            continue;
        };
        g.prev = g.feet();
        let mut weapon = asset.0.clone();
        let feet = g.feet();
        let mut controls = Controls::default();
        if let Some(p) = target
            && (p - feet).length() < NOTICE
        {
            d.engaged = true;
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
                    pitch: 0.7,
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
                if m == DoormanMove::Shove && (p.x - feet.x).abs() > SHOVE_REACH {
                    controls.x = g.ctrl.facing as f32;
                } else {
                    let id = attack_of(m);
                    let index = match m {
                        DoormanMove::Shove => 0,
                        DoormanMove::Knock => 1,
                        DoormanMove::Rush => 2,
                    };
                    let data = &mut weapon.combo[index];
                    data.recovery = d.phase.recovery(m);
                    data.windup = d.phase.windup(data.windup);
                    f.combat.attacker.start(&weapon, id, g.ctrl.facing);
                    d.beat += 1;
                    sfx.write(match m {
                        DoormanMove::Shove => PlaySfx {
                            pitch: 0.8,
                            ..PlaySfx::new(Sfx::SwingHeavy)
                        },
                        DoormanMove::Knock => PlaySfx {
                            pitch: 0.6,
                            ..PlaySfx::new(Sfx::DancerLeap)
                        },
                        DoormanMove::Rush => PlaySfx {
                            pitch: 0.7,
                            ..PlaySfx::new(Sfx::DancerDash)
                        },
                    });
                }
            }
        }
        let tuning_now = world::MoveTuning {
            run_speed: 60.0,
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
                (Some(DoormanMove::Rush), AttackPhase::Windup) if tick % 32 == 31 => {
                    // A stamp.
                    fx.write(crate::effects::SpawnFx::dust(
                        Vec2::new(feet.x, -feet.y),
                        pixelfx::effects::DustVariant::Land,
                    ));
                    shake.kick(Vec2::Y, 1.0);
                }
                (Some(DoormanMove::Rush), AttackPhase::Active) => {
                    let facing = a.facing().unwrap_or(g.ctrl.facing);
                    if g.ctrl.body.move_x(room, facing as f32 * RUSH_SPEED * dt) {
                        // Into the wall: he reels, and the rush is over.
                        f.combat.stagger();
                        shake.kick(Vec2::new(facing as f32, 0.0), 6.0);
                        sfx.write(PlaySfx::new(Sfx::WallSplat));
                        fx.write(crate::effects::SpawnFx::new(
                            pixelfx::EffectId::HeavyLanding,
                            Vec2::new(feet.x + facing as f32 * 12.0, -feet.y + 30.0),
                        ));
                    }
                }
                (Some(DoormanMove::Knock), AttackPhase::Active) if tick == 0 => {
                    let facing = a.facing().unwrap_or(g.ctrl.facing) as f32;
                    fx.write(crate::effects::SpawnFx::new(
                        pixelfx::EffectId::HeavyLanding,
                        Vec2::new(feet.x + facing * 22.0, -feet.y),
                    ));
                    shake.kick(Vec2::Y, 4.0);
                    sfx.write(PlaySfx {
                        pitch: 0.55,
                        ..PlaySfx::new(Sfx::WallSplat)
                    });
                }
                _ => {}
            }
        }
        crate::combat::fall_dead(&f, &mut g.ctrl, room, &tuning.movement);
        boss.finishable = f.combat.health < boss.max_health * FINISH_AT;
        if f.combat.reaction() == Reaction::Dead && !d.announced {
            d.announced = true;
            defeated.write(BossDefeated {
                kind: KIND.into(),
                at: Vec2::new(feet.x, -feet.y),
            });
        }
    }
}

/// The knock's line along the floor, out from the staff both ways, as wide as its hitbox.
fn draw_line(
    doormen: Query<&Fighting, With<Doorman>>,
    mut lines: Query<(&ChildOf, &mut Visibility, &mut Sprite), With<FloorLine>>,
) {
    for (parent, mut visible, mut sprite) in &mut lines {
        let Ok(f) = doormen.get(parent.parent()) else {
            continue;
        };
        let a = &f.combat.attacker;
        let knocking = move_of(a.attack()) == Some(DoormanMove::Knock);
        let (phase, tick) = (a.phase(), a.phase_tick());
        let (on, width, alpha) = match phase {
            // A faint warning on the floor while the staff is up.
            AttackPhase::Windup if knocking => (true, 40.0, 0.25 + 0.2 * (tick as f32 * 0.3).sin()),
            AttackPhase::Active if knocking => {
                let k = (tick as f32 / 30.0).min(1.0);
                (true, 60.0 + 500.0 * k, 1.0 - 0.6 * k)
            }
            _ => (false, 0.0, 0.0),
        };
        visible.set_if_neq(if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if on {
            sprite.custom_size = Some(Vec2::new(
                width,
                if phase == AttackPhase::Active {
                    4.0
                } else {
                    2.0
                },
            ));
            sprite.color = Color::srgba(1.0, 0.8, 0.5, alpha);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_phases_follow_his_health_and_the_rush_comes_with_impatience() {
        assert_eq!(Phase::at(1.0), Phase::List);
        assert_eq!(Phase::at(0.6), Phase::Impatience);
        assert_eq!(Phase::at(0.24), Phase::ClosingTime);
        assert!(!Phase::List.phrase().contains(&DoormanMove::Rush));
        assert!(Phase::Impatience.phrase().contains(&DoormanMove::Rush));
        for phase in [Phase::List, Phase::Impatience, Phase::ClosingTime] {
            for m in phase.phrase() {
                assert!(phase.windup(66) >= 36, "every windup stays readable");
                assert!(phase.recovery(*m) >= 30);
            }
        }
    }

    #[test]
    fn his_frame_data_parses_and_the_rush_is_his_body() {
        let weapon = world::combat::Weapon::from_ron(include_str!(
            "../../../assets/combat/boss-doorman.ron"
        ))
        .unwrap();
        assert!(weapon.attack(AttackId::Combo3).body);
        assert!(!weapon.attack(AttackId::Combo1).body);
        assert!(weapon.attack(AttackId::Combo3).armour);
    }
}
