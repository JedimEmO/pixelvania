//! Nectar, the High and the Comedown: the loop of feeling the game is built on. And Cordials, which heal.
//!
//! Nectar is golden and abstract. Guests drop it when they fall, a finisher shakes more loose,
//! and leftovers of the party lie about the rooms (`nectar x y` in a room file).
//! It drifts to the player when they come near. It is spent on nothing but Highs.
//!
//! A tap of the left trigger drinks a Cordial: three between rests, and one more for each Goblet.
//! Drinking takes a moment and slows the player, and a blow in that moment spills it.
//! Resting at a lounge, or waking at one, fills them again.
//! Holding the trigger spends Nectar on a High:
//! the world saturates, the music opens up, the player moves faster and hits harder.
//! When the High ends the Comedown follows, drained, quiet and slower, for a while.
//! Tolerance builds with every High, so the next is shorter;
//! resting at a lounge lowers it again, but never to nothing once it has started.
//! A blackout's hangover is a Comedown that lasts until the player rests or indulges.

use std::collections::HashSet;

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use pixelfx::effects::EffectId;

use crate::combat::{Fighting, GUEST_TEAM, Mood};
use crate::effects::SpawnFx;
use crate::enemies::Guest;
use crate::input::TickInput;
use crate::player::Player;
use crate::progress::PlayerProgress;
use crate::render::GLOW_LAYER;
use crate::rooms::{ActiveRoom, RoomPart};

pub struct IndulgePlugin;

impl Plugin for IndulgePlugin {
    fn build(&self, app: &mut App) {
        let saved = crate::lounge::startup_save();
        app.insert_resource(Indulgence {
            nectar: saved.as_ref().map_or(30.0, |s| s.nectar),
            tolerance: saved.as_ref().map_or(0.0, |s| s.tolerance),
            phase: Phase::Sober,
            held: 0.0,
            started: false,
            cordials: FULL,
            drinking: None,
            empty: 0.0,
        })
        .add_systems(
            Update,
            (indulge, drop_from_the_fallen, spawn_leftovers, drift),
        );
    }
}

/// What a Cordial heals, before Appetite, and how long it takes to drink.
const HEAL: f32 = 40.0;
const DRINK_TIME: f32 = 0.4;
/// Nectar to start a High.
pub const HIGH_COST: f32 = 50.0;
/// Cordials as many as can be carried; clamped to the progress's count each frame.
const FULL: u32 = u32::MAX;
/// Seconds the trigger must be held to start a High.
const HOLD: f32 = 0.5;
/// A High's length before tolerance, and how much each High adds to tolerance.
const HIGH_TIME: f32 = 14.0;
const TOLERANCE_STEP: f32 = 0.4;
/// How long the Comedown after a High lasts.
const COMEDOWN_TIME: f32 = 9.0;
/// The least tolerance a lounge leaves once a High has been taken.
const TOLERANCE_FLOOR: f32 = 0.2;

/// Where the player is in the loop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    Sober,
    High {
        left: f32,
    },
    /// `None` for a hangover, which lasts until the player rests or indulges.
    Comedown {
        left: Option<f32>,
    },
}

/// The player's Nectar and how used to it they are.
#[derive(Resource, Debug)]
pub struct Indulgence {
    pub nectar: f32,
    pub tolerance: f32,
    pub phase: Phase,
    /// Seconds the trigger has been held.
    pub held: f32,
    /// This hold has already started a High.
    started: bool,
    /// Cordials left until the next rest.
    pub cordials: u32,
    /// Seconds left of the Cordial being drunk, and the health it started at.
    pub drinking: Option<(f32, f32)>,
    /// Seconds left of the HUD's flash after a tap with no Cordial left.
    pub empty: f32,
}

impl Indulgence {
    /// How fast the player moves in this phase, against the tuning.
    pub fn pace(&self) -> f32 {
        let mood = match self.phase {
            Phase::High { .. } => 1.1,
            Phase::Comedown { .. } => 0.9,
            Phase::Sober => 1.0,
        };
        if self.drinking.is_some() {
            mood * 0.5
        } else {
            mood
        }
    }

    /// Blacks out: a hangover until the player rests or indulges. They wake at a lounge,
    /// so the Cordials are full again.
    pub fn hangover(&mut self) {
        self.phase = Phase::Comedown { left: None };
        self.cordials = FULL;
        self.drinking = None;
    }

    /// Rests at a lounge: the Cordials fill, the hangover lifts and tolerance falls.
    pub fn rest(&mut self) {
        self.cordials = FULL;
        if matches!(self.phase, Phase::Comedown { .. }) {
            self.phase = Phase::Sober;
        }
        let floor = if self.tolerance > 0.0 {
            TOLERANCE_FLOOR
        } else {
            0.0
        };
        self.tolerance = (self.tolerance - 2.0 * TOLERANCE_STEP).max(floor);
    }

    /// Adds Nectar up to what can be carried.
    pub fn gain(&mut self, amount: f32, capacity: f32) {
        self.nectar = (self.nectar + amount).min(capacity);
    }

    fn mood(&self) -> Mood {
        match self.phase {
            Phase::Sober => Mood::ALL[0],
            Phase::High { .. } => Mood::ALL[1],
            Phase::Comedown { .. } => Mood::ALL[2],
        }
    }
}

/// Reads the left trigger: a tap drinks a Cordial, a hold starts a High; and runs the High and the Comedown.
#[allow(clippy::too_many_arguments)]
fn indulge(
    time: Res<Time>,
    input: Res<TickInput>,
    progress: Res<PlayerProgress>,
    mut ind: ResMut<Indulgence>,
    mut mood: ResMut<Mood>,
    mut players: Query<(&mut Fighting, &Transform), With<Player>>,
    mut fx: MessageWriter<SpawnFx>,
    mut sfx: MessageWriter<crate::audio::PlaySfx>,
    mut shake: ResMut<crate::camera::Shake>,
) {
    let dt = time.delta_secs();
    let Ok((mut f, at)) = players.single_mut() else {
        return;
    };
    let at = at.translation.truncate() + Vec2::Y * 17.0;
    ind.nectar = ind.nectar.min(progress.0.nectar_capacity());
    ind.cordials = ind.cordials.min(progress.0.cordials());
    ind.empty = (ind.empty - dt).max(0.0);
    let alive = f.combat.health > 0.0;

    // A Cordial being drunk: a blow spills it, otherwise it heals once it is down.
    if let Some((left, before)) = ind.drinking {
        if !alive || f.combat.health < before {
            ind.drinking = None;
            sfx.write(crate::audio::PlaySfx {
                pitch: 0.7,
                volume: 0.6,
                ..crate::audio::PlaySfx::new(crate::audio::Sfx::LureRelease)
            });
        } else if left - dt > 0.0 {
            ind.drinking = Some((left - dt, before));
        } else {
            ind.drinking = None;
            let heal = HEAL * progress.0.stats.heal_scale();
            f.combat.health = (f.combat.health + heal).min(f.max_health);
            let mut glow = SpawnFx::new(EffectId::Poof, at);
            glow.options.palette = CORDIAL.iter().map(|&c| pixelfx::color::hex(c)).collect();
            glow.options.light = 1.4;
            fx.write(glow);
            sfx.write(crate::audio::PlaySfx {
                pitch: 1.6,
                volume: 0.8,
                ..crate::audio::PlaySfx::new(crate::audio::Sfx::LureCatch)
            });
            // Drinking lifts a hangover.
            if ind.phase == (Phase::Comedown { left: None }) {
                ind.phase = Phase::Sober;
            }
        }
    }

    if input.indulge_held && alive {
        ind.held += dt;
        let high = matches!(ind.phase, Phase::High { .. });
        if ind.held >= HOLD && !ind.started && !high && ind.nectar >= HIGH_COST {
            ind.started = true;
            ind.nectar -= HIGH_COST;
            let left = HIGH_TIME * progress.0.stats.high_scale() / (1.0 + ind.tolerance);
            ind.tolerance += TOLERANCE_STEP;
            ind.phase = Phase::High { left };
            let mut burst = SpawnFx::new(EffectId::Poof, at);
            burst.options.palette = GOLD.iter().map(|&c| pixelfx::color::hex(c)).collect();
            burst.options.light = 2.0;
            fx.write(burst);
            shake.kick(Vec2::Y, 3.0);
            sfx.write(crate::audio::PlaySfx {
                pitch: 1.25,
                ..crate::audio::PlaySfx::new(crate::audio::Sfx::Finisher)
            });
            info!(
                "a High, for {left:.1} s; tolerance now {:.1}",
                ind.tolerance
            );
        }
    } else {
        // Let go before the hold became a High: a tap, which drinks a Cordial.
        let tapped = ind.held > 0.0 && ind.held < HOLD;
        let hurt = f.combat.health < f.max_health;
        if tapped && alive && hurt && ind.drinking.is_none() {
            if ind.cordials > 0 {
                ind.cordials -= 1;
                ind.drinking = Some((DRINK_TIME, f.combat.health));
                sfx.write(crate::audio::PlaySfx {
                    pitch: 2.2,
                    volume: 0.5,
                    ..crate::audio::PlaySfx::new(crate::audio::Sfx::Footstep)
                });
            } else {
                ind.empty = 0.5;
                sfx.write(crate::audio::PlaySfx {
                    pitch: 0.5,
                    volume: 0.5,
                    ..crate::audio::PlaySfx::new(crate::audio::Sfx::HitLight)
                });
            }
        }
        ind.held = 0.0;
        ind.started = false;
    }

    ind.phase = match ind.phase {
        Phase::High { left } if left - dt <= 0.0 => {
            sfx.write(crate::audio::PlaySfx {
                pitch: 0.5,
                volume: 0.8,
                ..crate::audio::PlaySfx::new(crate::audio::Sfx::LureRelease)
            });
            Phase::Comedown {
                left: Some(COMEDOWN_TIME),
            }
        }
        Phase::High { left } => Phase::High { left: left - dt },
        Phase::Comedown { left: Some(left) } if left - dt <= 0.0 => Phase::Sober,
        Phase::Comedown { left: Some(left) } => Phase::Comedown {
            left: Some(left - dt),
        },
        other => other,
    };
    let want = ind.mood();
    if mood.name != want.name && !crate::devtools::mood_set_by_hand() {
        *mood = want;
    }
}

/// A Cordial's rose-red ramp, dark to light.
const CORDIAL: [u32; 6] = [0x3a0c1c, 0x7a1830, 0xb83048, 0xe05a6a, 0xff9aa0, 0xffe0e0];

/// Nectar's golden ramp, dark to light.
const GOLD: [u32; 6] = [0x4a3010, 0x8a5a18, 0xc88a28, 0xf0b840, 0xffd878, 0xfff4d0];

/// A mote of Nectar: it waits, then drifts to the player and is drunk.
#[derive(Component)]
struct Mote {
    /// Velocity in pixels per second, world y up.
    v: Vec2,
    /// Seconds before it starts to drift to the player.
    rest: f32,
    /// A leftover in a room waits until the player comes near.
    waiting: bool,
    amount: f32,
}

fn mote(at: Vec2, v: Vec2, rest: f32, waiting: bool, amount: f32) -> impl Bundle {
    (
        RoomPart,
        Mote {
            v,
            rest,
            waiting,
            amount,
        },
        Sprite::from_color(Color::srgb(1.0, 0.85, 0.4), Vec2::splat(2.0)),
        RenderLayers::layer(GLOW_LAYER),
        Transform::from_translation(at.round().extend(3.0)),
    )
}

/// Guests drop Nectar as they fall: more from heavier ones.
fn drop_from_the_fallen(
    mut commands: Commands,
    guests: Query<(Entity, &Guest, &Fighting, &Transform)>,
    mut fallen: Local<HashSet<Entity>>,
    mut seed: Local<u32>,
) {
    for (e, g, f, t) in &guests {
        if f.team != GUEST_TEAM || f.combat.health > 0.0 || !fallen.insert(e) {
            continue;
        }
        let count = match g.class {
            anim::guest::Class::Light => 3,
            anim::guest::Class::Heavy => 7,
            _ => 5,
        };
        let at = t.translation.truncate() + Vec2::Y * 16.0;
        for i in 0..count {
            *seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            let r = (*seed >> 16) as f32 / 65_536.0;
            let a = std::f32::consts::PI * (0.15 + 0.7 * ((i as f32 + r) / count as f32));
            let v = Vec2::new(a.cos(), a.sin()) * (90.0 + 60.0 * r);
            commands.spawn(mote(at, v, 0.5 + 0.2 * r, false, 4.0));
        }
    }
    fallen.retain(|e| guests.contains(*e));
}

/// The room's leftovers: clusters of Nectar waiting where the party left them.
fn spawn_leftovers(
    mut commands: Commands,
    active: Res<ActiveRoom>,
    mut spawned: Local<Option<u64>>,
) {
    let Some(room) = &active.room else { return };
    if *spawned == Some(active.build) {
        return;
    }
    *spawned = Some(active.build);
    for def in room.entities_of("nectar") {
        let (x, y) = def.pixel_pos();
        for i in 0..6 {
            let a = i as f32 * 1.047;
            let at = Vec2::new(x, -y) + Vec2::new(a.cos() * 5.0, a.sin() * 3.0 + 4.0);
            commands.spawn(mote(at, Vec2::ZERO, 0.0, true, 5.0));
        }
    }
}

/// Moves motes: a short fall after a drop, a hover while waiting, then a pull to the player.
fn drift(
    mut commands: Commands,
    time: Res<Time>,
    progress: Res<PlayerProgress>,
    mut ind: ResMut<Indulgence>,
    players: Query<&Transform, (With<Player>, Without<Mote>)>,
    mut motes: Query<(Entity, &mut Mote, &mut Transform)>,
    mut sfx: MessageWriter<crate::audio::PlaySfx>,
) {
    let dt = time.delta_secs();
    let Ok(player) = players.single() else { return };
    let target = player.translation.truncate() + Vec2::Y * 17.0;
    let mut drunk = 0;
    for (e, mut m, mut t) in &mut motes {
        let at = t.translation.truncate();
        let to = target - at;
        if m.waiting {
            if to.length() < 64.0 {
                m.waiting = false;
            }
            let bob = (time.elapsed_secs() * 3.0 + at.x * 0.1).sin() * 6.0;
            m.v = Vec2::new(0.0, bob);
        } else if m.rest > 0.0 {
            m.rest -= dt;
            m.v.y -= 400.0 * dt;
            m.v *= 1.0 - 2.0 * dt;
        } else {
            // Pulled ever harder, so a mote never orbits the player.
            let pull = to.normalize_or_zero() * 900.0;
            m.v = (m.v + pull * dt) * (1.0 - 3.0 * dt);
            m.v += to * 4.0 * dt;
            if to.length() < 8.0 {
                ind.gain(m.amount, progress.0.nectar_capacity());
                commands.entity(e).despawn();
                drunk += 1;
                continue;
            }
        }
        t.translation += (m.v * dt).extend(0.0);
    }
    if drunk > 0 {
        sfx.write(crate::audio::PlaySfx {
            pitch: 2.0,
            volume: 0.25,
            ..crate::audio::PlaySfx::new(crate::audio::Sfx::Footstep)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> Indulgence {
        Indulgence {
            nectar: 0.0,
            tolerance: 0.0,
            phase: Phase::Sober,
            held: 0.0,
            started: false,
            cordials: 0,
            drinking: None,
            empty: 0.0,
        }
    }

    #[test]
    fn resting_and_waking_fill_the_cordials() {
        let mut i = fresh();
        i.rest();
        assert_eq!(i.cordials, FULL);
        i.cordials = 0;
        i.drinking = Some((0.2, 50.0));
        i.hangover();
        assert_eq!(i.cordials, FULL);
        assert_eq!(i.drinking, None);
    }

    #[test]
    fn drinking_slows_the_player() {
        let mut i = fresh();
        i.drinking = Some((0.2, 50.0));
        assert!(i.pace() < 1.0);
    }

    #[test]
    fn resting_lifts_a_hangover_and_never_clears_tolerance_once_it_has_started() {
        let mut i = fresh();
        i.hangover();
        i.rest();
        assert_eq!(i.phase, Phase::Sober);
        assert_eq!(i.tolerance, 0.0);
        i.tolerance = 1.2;
        i.rest();
        i.rest();
        assert!((i.tolerance - TOLERANCE_FLOOR).abs() < 1e-6);
    }

    #[test]
    fn nectar_stops_at_what_can_be_carried() {
        let mut i = fresh();
        i.gain(80.0, 100.0);
        i.gain(80.0, 100.0);
        assert_eq!(i.nectar, 100.0);
    }
}
