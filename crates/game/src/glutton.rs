//! The Glutton: the feast's boss, at the end of the long table.
//!
//! He is a guest of his own class, so he moves, is hit, staggers and falls as guests do,
//! and this module is his brain. He fights in three phases, each a phrase of moves:
//! appetite (above 60% health), swipes of a magnum and hops; seconds, which adds the gorge;
//! and too much (under 25%), when he cannot eat any more and hops back to back.
//!
//! Each move has its own telegraph. The swipe draws the bottle back over his shoulder,
//! the hop sinks him into a wobbling crouch, and the gorge puts a hand to his mouth.
//! The hop is a real jump at the player; when he lands, the landing starts,
//! and the floor shakes out both ways under anyone not in the air.
//! While he gorges he heals, until three blows make him choke, which staggers him.
//!
//! Beaten, he stays beaten: the progress record keeps him, and his hall is quiet after.

use anim::guest::{Class, GluttonMove};
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use world::Controls;
use world::combat::{AttackId, Phase as AttackPhase, Reaction};

use crate::boss::{Boss, BossDefeated};
use crate::combat::{Fighting, SimulateBodies, WeaponAsset, Weapons, combat_id};
use crate::enemies::Guest;
use crate::player::Player;
use crate::rooms::ActiveRoom;

pub struct GluttonPlugin;

impl Plugin for GluttonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            tick.in_set(SimulateBodies).after(crate::player::PlayerTick),
        )
        .add_systems(Update, (spawn, draw_wave).chain());
    }
}

/// His kind, in the progress record and in `assets/story.ron`.
const KIND: &str = "glutton";
/// How near the player must be for him to start, in pixels.
const NOTICE: f32 = 400.0;
/// How near the player must be for a swipe, in pixels; further, he waddles up first.
const SWIPE_REACH: f32 = 76.0;
/// The hop's speed upward, and across towards the player, in pixels per second.
const HOP_UP: f32 = 440.0;
const HOP_ACROSS: f32 = 130.0;
/// Ticks he stands still after a phase begins, and at the start.
const PHASE_PAUSE: u32 = 110;
/// Below this share of his health, a blow on him staggered finishes him.
const FINISH_AT: f32 = 0.12;
/// Health he gains each tick he gorges.
const GORGE_HEAL: f32 = 0.5;
/// Blows that make him choke on what he is eating.
const CHOKE_HITS: u32 = 3;

/// The fight's phases.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Appetite,
    Seconds,
    TooMuch,
}

impl Phase {
    fn at(share: f32) -> Self {
        if share < 0.25 {
            Self::TooMuch
        } else if share <= 0.6 {
            Self::Seconds
        } else {
            Self::Appetite
        }
    }

    fn number(self) -> u8 {
        match self {
            Self::Appetite => 1,
            Self::Seconds => 2,
            Self::TooMuch => 3,
        }
    }

    /// The phrase of moves he repeats in this phase.
    fn phrase(self) -> &'static [GluttonMove] {
        use GluttonMove::*;
        match self {
            Self::Appetite => &[Swipe, Swipe, Hop],
            Self::Seconds => &[Swipe, Hop, Gorge, Swipe, Hop],
            Self::TooMuch => &[Hop, Swipe, Hop, Hop],
        }
    }

    /// Ticks to recover after a move: long at first, short once it is too much.
    fn recovery(self, m: GluttonMove) -> u32 {
        match (self, m) {
            (_, GluttonMove::Hop) => 1,
            (_, GluttonMove::Gorge) => 50,
            (Self::Appetite, GluttonMove::Land) => 80,
            (Self::Appetite, _) => 80,
            (Self::Seconds, GluttonMove::Land) => 70,
            (Self::Seconds, _) => 60,
            (Self::TooMuch, GluttonMove::Land) => 40,
            (Self::TooMuch, _) => 34,
        }
    }

    /// Windups shorten as the fight goes on, but never below a readable minimum.
    fn windup(self, ticks: u32) -> u32 {
        match self {
            Self::Appetite => ticks,
            Self::Seconds => (ticks * 9 / 10).max(40),
            Self::TooMuch => (ticks * 3 / 4).max(36),
        }
    }
}

fn attack_of(m: GluttonMove) -> AttackId {
    match m {
        GluttonMove::Swipe => AttackId::Combo1,
        GluttonMove::Hop => AttackId::Combo2,
        GluttonMove::Land => AttackId::Combo3,
        GluttonMove::Gorge => AttackId::Charged,
    }
}

/// Which of his moves an attack is, for drawing him.
pub fn move_of(attack: Option<AttackId>) -> Option<GluttonMove> {
    match attack? {
        AttackId::Combo1 => Some(GluttonMove::Swipe),
        AttackId::Combo2 => Some(GluttonMove::Hop),
        AttackId::Combo3 => Some(GluttonMove::Land),
        AttackId::Charged => Some(GluttonMove::Gorge),
        _ => None,
    }
}

/// His move's frame data in his weapon.
fn data_of(weapon: &mut world::combat::Weapon, m: GluttonMove) -> &mut world::combat::AttackData {
    match m {
        GluttonMove::Swipe => &mut weapon.combo[0],
        GluttonMove::Hop => &mut weapon.combo[1],
        GluttonMove::Land => &mut weapon.combo[2],
        GluttonMove::Gorge => &mut weapon.charged,
    }
}

/// The Glutton's brain, on a guest of the Glutton class.
#[derive(Component)]
pub struct Glutton {
    phase: Phase,
    /// The move in the phrase he is on.
    beat: usize,
    /// Ticks he waits before choosing a move.
    pause: u32,
    /// The player has come near, so the fight is on.
    engaged: bool,
    announced: bool,
    /// In the air on a hop: the way across, and ticks since he left the floor.
    hop: Option<(i32, u32)>,
    /// Blows taken during this gorge.
    choke: u32,
    /// His health at the end of the last tick, to notice blows during a gorge.
    last_health: f32,
}

/// The landing's shake along the floor.
#[derive(Component)]
struct FloorWave;

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
            Class::Glutton,
            def.pixel_pos(),
            &weapons,
            &mut images,
        );
        commands.entity(e).insert((
            Name::new("The Glutton"),
            Glutton {
                phase: Phase::Appetite,
                beat: 0,
                pause: PHASE_PAUSE,
                engaged: false,
                announced: false,
                hop: None,
                choke: 0,
                last_health: 640.0,
            },
            Boss {
                name: "THE GLUTTON",
                kind: KIND,
                music: "glutton",
                max_health: 640.0,
                phase: 1,
                finishable: false,
            },
        ));
        commands.entity(e).with_child((
            FloorWave,
            Sprite::from_color(Color::srgb(1.0, 0.55, 0.4), Vec2::new(8.0, 3.0)),
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
    players: Query<(&Player, &Fighting), Without<Glutton>>,
    mut gluttons: Query<
        (Entity, &mut Guest, &mut Fighting, &mut Glutton, &mut Boss),
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
    for (entity, mut g, mut f, mut d, mut boss) in &mut gluttons {
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

        // A blow while he gorges: three, and he chokes.
        let gorging = move_of(f.combat.attacker.attack()) == Some(GluttonMove::Gorge);
        if gorging && f.combat.health < d.last_health - 0.01 {
            d.choke += 1;
            if d.choke >= CHOKE_HITS {
                d.choke = 0;
                f.combat.stagger();
                f.flash = 0.22;
                shake.kick(Vec2::Y, 3.0);
                sfx.write(PlaySfx {
                    pitch: 0.6,
                    ..PlaySfx::new(Sfx::PoiseBreak)
                });
            }
        }

        if !f.combat.is_frozen() && f.combat.health > 0.0 && d.engaged {
            let phase = Phase::at(f.combat.health / boss.max_health);
            if phase != d.phase && d.hop.is_none() {
                d.phase = phase;
                boss.phase = phase.number();
                d.beat = 0;
                d.pause = PHASE_PAUSE;
                f.combat.stagger();
                f.flash = 0.22;
                sfx.write(PlaySfx {
                    pitch: 0.6,
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
                if m == GluttonMove::Swipe && (p.x - feet.x).abs() > SWIPE_REACH {
                    controls.x = g.ctrl.facing as f32;
                } else {
                    let data = data_of(&mut weapon, m);
                    data.recovery = d.phase.recovery(m);
                    data.windup = d.phase.windup(data.windup);
                    f.combat
                        .attacker
                        .start(&weapon, attack_of(m), g.ctrl.facing);
                    d.beat += 1;
                    d.choke = 0;
                    sfx.write(match m {
                        GluttonMove::Swipe => PlaySfx {
                            pitch: 0.7,
                            ..PlaySfx::new(Sfx::BottleSwing)
                        },
                        GluttonMove::Hop | GluttonMove::Land => PlaySfx {
                            pitch: 0.55,
                            ..PlaySfx::new(Sfx::GuestWindup)
                        },
                        GluttonMove::Gorge => PlaySfx {
                            pitch: 0.5,
                            ..PlaySfx::new(Sfx::GuestWindup)
                        },
                    });
                }
            }
        }

        // In the air on a hop: across towards the player, until he lands.
        if let Some((across, ticks)) = d.hop {
            if move_of(f.combat.attacker.attack()) != Some(GluttonMove::Hop) {
                // Staggered or beaten in the air: the hop is over.
                d.hop = None;
            } else {
                g.ctrl.body.move_x(room, across as f32 * HOP_ACROSS * dt);
                d.hop = Some((across, ticks + 1));
                if ticks > 4 && g.ctrl.body.on_ground(room) {
                    d.hop = None;
                    f.combat.attacker.interrupt();
                    let data = data_of(&mut weapon, GluttonMove::Land);
                    data.recovery = d.phase.recovery(GluttonMove::Land);
                    f.combat.attacker.start(&weapon, AttackId::Combo3, across);
                    fx.write(crate::effects::SpawnFx::new(
                        pixelfx::EffectId::HeavyLanding,
                        Vec2::new(feet.x, -feet.y),
                    ));
                    for side in [-1.0, 1.0] {
                        fx.write(crate::effects::SpawnFx::dust(
                            Vec2::new(feet.x + side * 24.0, -feet.y),
                            pixelfx::effects::DustVariant::Land,
                        ));
                    }
                    shake.kick(Vec2::Y, 7.0);
                    sfx.write(PlaySfx {
                        pitch: 0.5,
                        ..PlaySfx::new(Sfx::WallSplat)
                    });
                    sfx.write(PlaySfx::new(Sfx::LandHeavy));
                }
            }
        }

        let tuning_now = world::MoveTuning {
            run_speed: 45.0,
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
                (Some(GluttonMove::Hop), AttackPhase::Active) if d.hop.is_none() && tick == 0 => {
                    // Up, and at where the player stands now.
                    let across = target.map_or(g.ctrl.facing, |p| {
                        if (p.x - feet.x).abs() < 8.0 {
                            0
                        } else if p.x < feet.x {
                            -1
                        } else {
                            1
                        }
                    });
                    g.ctrl.vy = -HOP_UP;
                    d.hop = Some((across, 0));
                    fx.write(crate::effects::SpawnFx::dust(
                        Vec2::new(feet.x, -feet.y),
                        pixelfx::effects::DustVariant::Land,
                    ));
                    sfx.write(PlaySfx {
                        pitch: 0.6,
                        ..PlaySfx::new(Sfx::Jump)
                    });
                }
                (Some(GluttonMove::Hop), AttackPhase::Windup) if tick % 40 == 39 => {
                    // The floor creaks under him as he sinks.
                    shake.kick(Vec2::Y, 0.6);
                }
                (Some(GluttonMove::Gorge), AttackPhase::Active) => {
                    f.combat.health = (f.combat.health + GORGE_HEAL).min(boss.max_health);
                    if tick % 24 == 0 {
                        sfx.write(PlaySfx {
                            pitch: 0.45 + 0.1 * ((tick / 24) % 3) as f32,
                            volume: 0.6,
                            ..PlaySfx::new(Sfx::Footstep)
                        });
                    }
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

/// The landing's shake out along the floor both ways, as wide as its hitboxes.
fn draw_wave(
    gluttons: Query<&Fighting, With<Glutton>>,
    mut waves: Query<(&ChildOf, &mut Visibility, &mut Sprite), With<FloorWave>>,
) {
    for (parent, mut visible, mut sprite) in &mut waves {
        let Ok(f) = gluttons.get(parent.parent()) else {
            continue;
        };
        let a = &f.combat.attacker;
        let landing = move_of(a.attack()) == Some(GluttonMove::Land);
        let on = landing && a.phase() == AttackPhase::Active;
        visible.set_if_neq(if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if on {
            let k = (a.phase_tick() as f32 / 26.0).min(1.0);
            sprite.custom_size = Some(Vec2::new(70.0 + 510.0 * k, 4.0));
            sprite.color = Color::srgba(1.0, 0.55, 0.4, 1.0 - 0.6 * k);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_phases_follow_his_health_and_he_stops_eating_when_it_is_too_much() {
        assert_eq!(Phase::at(1.0), Phase::Appetite);
        assert_eq!(Phase::at(0.6), Phase::Seconds);
        assert_eq!(Phase::at(0.24), Phase::TooMuch);
        assert!(!Phase::Appetite.phrase().contains(&GluttonMove::Gorge));
        assert!(Phase::Seconds.phrase().contains(&GluttonMove::Gorge));
        assert!(!Phase::TooMuch.phrase().contains(&GluttonMove::Gorge));
        for phase in [Phase::Appetite, Phase::Seconds, Phase::TooMuch] {
            assert!(phase.windup(66) >= 36, "every windup stays readable");
            for m in phase.phrase() {
                assert!(move_of(Some(attack_of(*m))) == Some(*m));
            }
        }
    }

    #[test]
    fn his_frame_data_parses_and_only_the_landing_hits_from_the_hop() {
        let weapon = world::combat::Weapon::from_ron(include_str!(
            "../../../assets/combat/boss-glutton.ron"
        ))
        .unwrap();
        assert!(weapon.attack(AttackId::Combo2).hitboxes.is_empty());
        assert!(weapon.attack(AttackId::Charged).hitboxes.is_empty());
        assert!(!weapon.attack(AttackId::Combo3).hitboxes.is_empty());
        // The landing's waves reach across his whole hall.
        let widest = weapon
            .attack(AttackId::Combo3)
            .hitboxes
            .iter()
            .map(|h| h.rect.w)
            .fold(0.0, f32::max);
        assert!(widest >= 48.0 * 16.0 / 2.0);
    }

    /// He gets off the floor and comes back down within the hop's active time.
    #[test]
    fn the_hop_lands_before_it_runs_out() {
        let src = "[room]\nname = t\n[legend]\n# stone\n[tiles]\n\
                   ..........\n..........\n..........\n..........\n..........\n\
                   ..........\n..........\n..........\n..........\n##########\n";
        let room = world::Room::parse(src).unwrap();
        let mut c = world::Controller::standing_at(80.0, 144.0, 32, 50, Default::default());
        let t = world::MoveTuning::default();
        c.vy = -HOP_UP;
        let mut ticks = 0;
        loop {
            c.tick(&room, &t, Controls::default(), world::combat::DT);
            ticks += 1;
            if ticks > 4 && c.body.on_ground(&room) {
                break;
            }
            assert!(ticks < 240, "the hop never lands");
        }
        assert!(ticks > 30, "the hop is a real jump: {ticks} ticks");
    }
}
