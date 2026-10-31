//! Combat in the game: weapons as assets, contacts resolved each tick, and the impact of a hit.
//!
//! The rules live in `world::combat`, without the engine. This plugin loads the frame data,
//! resolves the contacts of every [`Fighting`] body once all of them have moved, and turns what
//! happened into what the player sees and feels: sparks and slash trails from pixelfx,
//! shake along the line of the hit, rumble by weight, and a white flash on the body that was hit.
//! The [`Mood`] scales every impact; the High makes hits heavier and the Comedown hollow.

use std::f32::consts::FRAC_PI_2;
use std::time::Duration;

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest};
use bevy::prelude::*;
use pixelfx::EffectId;
use world::Body;
use world::combat::{
    AttackId, CombatEvent, Combatant, Fighter, ImpactScale, Phase, Rect, Weapon, Weight, resolve,
    resolve_body_collisions,
};

use crate::camera::Shake;
use crate::effects::SpawnFx;
use crate::input::TickSet;
use crate::player::Player;
use crate::tuning::Tuning;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<WeaponAsset>()
            .register_asset_loader(WeaponLoader)
            .init_resource::<Mood>()
            .init_resource::<CombatLog>()
            .init_resource::<SlowMotion>()
            .add_systems(Startup, load_weapons)
            .add_systems(
                FixedUpdate,
                (resolve_contacts, count_reactions)
                    .chain()
                    .in_set(TickSet::Simulate)
                    .after(SimulateBodies),
            )
            .configure_sets(FixedUpdate, SimulateBodies.in_set(TickSet::Simulate))
            .add_systems(
                Update,
                (swing_effects, impact, fade_flash, slow_motion)
                    .chain()
                    .after(crate::player::place)
                    .before(crate::player::animate)
                    .before(crate::enemies::animate),
            );
    }
}

/// Fighters move in this set; contacts are resolved after it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimulateBodies;

/// The current weight of impacts: neutral, the High or the Comedown.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct Mood {
    pub name: &'static str,
    pub scale: ImpactScale,
}

impl Mood {
    pub const ALL: [Mood; 3] = [
        Mood {
            name: "neutral",
            scale: ImpactScale::NEUTRAL,
        },
        Mood {
            name: "the High",
            scale: ImpactScale::HIGH,
        },
        Mood {
            name: "the Comedown",
            scale: ImpactScale::COMEDOWN,
        },
    ];

    /// The next mood in the cycle neutral, High, Comedown.
    pub fn next(self) -> Self {
        let i = Self::ALL
            .iter()
            .position(|m| m.name == self.name)
            .unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }
}

impl Default for Mood {
    fn default() -> Self {
        Self::ALL[0]
    }
}

/// A weapon's frame data, loaded from `assets/combat/*.ron`.
#[derive(Asset, TypePath, Clone)]
pub struct WeaponAsset(pub Weapon);

/// The weapons by file name, loaded at startup.
#[derive(Resource)]
pub struct Weapons {
    pub sabre: Handle<WeaponAsset>,
    pub jeroboam: Handle<WeaponAsset>,
    pub candelabrum: Handle<WeaponAsset>,
    pub guest_light: Handle<WeaponAsset>,
    pub guest_medium: Handle<WeaponAsset>,
    pub guest_heavy: Handle<WeaponAsset>,
}

fn load_weapons(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(Weapons {
        sabre: assets.load("combat/sabre.ron"),
        jeroboam: assets.load("combat/jeroboam.ron"),
        candelabrum: assets.load("combat/candelabrum.ron"),
        guest_light: assets.load("combat/guest-light.ron"),
        guest_medium: assets.load("combat/guest-medium.ron"),
        guest_heavy: assets.load("combat/guest-heavy.ron"),
    });
}

/// A body that fights: its combat state, its side, and how it is shown being hit.
#[derive(Component)]
pub struct Fighting {
    pub combat: Combatant,
    /// Bodies on the same team cannot hit each other with weapons.
    pub team: u32,
    pub weapon: Handle<WeaponAsset>,
    /// Health when whole, for bars.
    pub max_health: f32,
    /// Seconds of white flash left after being hit.
    pub flash: f32,
    /// Seconds left flat against a wall after a wall splat.
    pub splat: f32,
    /// The attack phase last frame, to start a slash trail on the first active frame.
    last_phase: Phase,
    /// The reaction last tick, and seconds since it began, for reaction poses.
    /// Counted in gameplay ticks that are not frozen, so a pose holds through hit-stop.
    last_reaction: world::combat::Reaction,
    reaction_time: f32,
    /// The direction of the last blow taken, so the body can turn to face it.
    pub hit_from: Option<i32>,
    /// The x of the wall a splat happened against, in pixels; the splat pose ends once the body leaves it.
    splat_x: Option<i32>,
}

impl Fighting {
    /// Seconds since the current reaction began.
    pub fn reaction_time(&self) -> f32 {
        self.reaction_time
    }

    /// Ends the splat pose once the body has come away from the wall it hit.
    pub fn leave_wall(&mut self, body: &Body) {
        if let Some(x) = self.splat_x
            && (body.x - x).abs() > 4
        {
            self.splat = 0.0;
            self.splat_x = None;
        }
    }

    pub fn new(combat: Combatant, team: u32, weapon: Handle<WeaponAsset>) -> Self {
        Self {
            max_health: combat.health,
            combat,
            team,
            weapon,
            flash: 0.0,
            splat: 0.0,
            last_phase: Phase::Idle,
            last_reaction: world::combat::Reaction::Ready,
            reaction_time: 0.0,
            hit_from: None,
            splat_x: None,
        }
    }
}

pub const PLAYER_TEAM: u32 = 0;
pub const GUEST_TEAM: u32 = 1;

/// Something with a movement controller that can be hit: the player or a guest.
pub trait HasBody {
    fn body(&self) -> &Body;
}

impl HasBody for Player {
    fn body(&self) -> &Body {
        &self.ctrl.body
    }
}

impl HasBody for crate::enemies::Guest {
    fn body(&self) -> &Body {
        &self.ctrl.body
    }
}

/// What combat did on the ticks since the last frame, for the impact system to show.
#[derive(Resource, Default)]
pub struct CombatLog(pub Vec<CombatEvent>, pub Vec<Finisher>);

/// The player's blow landing on a staggered guest: a kill, framed by slow motion.
#[derive(Clone, Copy, Debug)]
pub struct Finisher {
    pub target: u64,
    pub direction: i32,
}

/// Real seconds of slow motion left, and how slow.
#[derive(Resource, Default)]
pub struct SlowMotion {
    pub left: f32,
    pub speed: f32,
}

/// How long a finisher's slow motion lasts in real seconds, and the game speed during it.
const FINISHER_SLOW: (f32, f32) = (0.3, 0.25);

/// An entity's id in combat: stable, unique, and the way back to the entity.
pub fn combat_id(e: Entity) -> u64 {
    e.to_bits()
}

fn resolve_contacts(
    mood: Res<Mood>,
    mut log: ResMut<CombatLog>,
    mut fighters: Query<(
        Entity,
        &mut Fighting,
        Option<&Player>,
        Option<&crate::enemies::Guest>,
    )>,
) {
    let mut rows: Vec<(u64, u32, Mut<Fighting>, Body, [Rect; 1])> = fighters
        .iter_mut()
        .filter_map(|(e, f, p, g)| {
            let body = *p.map(HasBody::body).or(g.map(HasBody::body))?;
            Some((combat_id(e), f.team, f, body, [Rect::from_body(&body)]))
        })
        .collect();
    let mut views: Vec<Fighter> = rows
        .iter_mut()
        .map(|(id, team, f, body, hurt)| Fighter {
            id: *id,
            team: *team,
            combat: &mut f.combat,
            body,
            hurtboxes: hurt,
        })
        .collect();
    // A finisher is the player's blow on a body that was already staggered before it landed.
    let staggered: Vec<u64> = views
        .iter()
        .filter(|v| v.combat.reaction() == world::combat::Reaction::Stagger)
        .map(|v| v.id)
        .collect();
    let events = resolve(&mut views, mood.scale);
    for e in &events {
        if let CombatEvent::Hit(h) = e
            && staggered.contains(&h.target)
            && views
                .iter()
                .any(|v| v.id == h.attacker && v.team == PLAYER_TEAM)
            && let Some(v) = views.iter_mut().find(|v| v.id == h.target)
        {
            v.combat.health = 0.0;
            log.1.push(Finisher {
                target: h.target,
                direction: h.direction,
            });
        }
    }
    let mut events = events;
    events.extend(resolve_body_collisions(&mut views, mood.scale));
    drop(views);
    // A fresh blow restarts the reaction pose, and turns the body to face where it came from.
    for e in &events {
        let (target, dir) = match e {
            CombatEvent::Hit(h) => (h.target, h.direction),
            CombatEvent::BodyHit {
                target, velocity, ..
            } => (*target, velocity.0.signum() as i32),
            _ => continue,
        };
        if let Some(row) = rows.iter_mut().find(|r| r.0 == target) {
            row.2.reaction_time = 0.0;
            row.2.hit_from = Some(dir);
        }
    }
    log.0.extend(events);
}

/// Advances each body's reaction clock by one tick, except while it is frozen in hit-stop.
fn count_reactions(mut fighters: Query<&mut Fighting>) {
    for mut f in &mut fighters {
        let r = f.combat.reaction();
        if r != f.last_reaction {
            f.last_reaction = r;
            f.reaction_time = 0.0;
        } else if !f.combat.is_frozen() {
            f.reaction_time += world::combat::DT;
        }
    }
}

/// Lets a dead body fall and slide to a stop: combat stops moving it, but it should land.
pub fn fall_dead(
    f: &Fighting,
    ctrl: &mut world::Controller,
    room: &world::Room,
    t: &world::MoveTuning,
) {
    if f.combat.reaction() != world::combat::Reaction::Dead {
        return;
    }
    let dt = world::combat::DT;
    ctrl.vy = (ctrl.vy + t.gravity * dt).min(t.max_fall);
    ctrl.vx -= ctrl.vx.signum() * ctrl.vx.abs().min(600.0 * dt);
    if ctrl.body.move_x(room, ctrl.vx * dt) {
        ctrl.vx = 0.0;
    }
    if ctrl.body.move_y(room, ctrl.vy * dt) {
        ctrl.vy = 0.0;
    }
    ctrl.grounded = ctrl.body.on_ground(room);
}

/// Eases the game into and out of a finisher's slow motion, in real time.
fn slow_motion(
    real: Res<Time<Real>>,
    mut slow: ResMut<SlowMotion>,
    mut time: ResMut<Time<Virtual>>,
) {
    if slow.left <= 0.0 {
        return;
    }
    slow.left -= real.delta_secs();
    let speed = if slow.left > 0.0 { slow.speed } else { 1.0 };
    time.set_relative_speed(speed);
}

/// How far the view jolts for a blow of each weight, in pixels, before the mood.
fn shake_pixels(w: Weight) -> f32 {
    match w {
        Weight::Light => 1.5,
        Weight::Heavy => 3.0,
        Weight::Charged => 5.0,
    }
}

/// Where a body's centre is in world space (y up).
fn centre(body: &Body) -> Vec2 {
    Vec2::new(
        body.x as f32 + body.w as f32 / 2.0,
        -(body.y as f32 + body.h as f32 / 2.0),
    )
}

/// Starts a slash trail on the first active frame of a swing.
fn swing_effects(
    mut commands: Commands,
    mut fighters: Query<(
        &mut Fighting,
        &Transform,
        Option<&Player>,
        Option<&crate::enemies::Guest>,
    )>,
    mut fx: MessageWriter<SpawnFx>,
    active: Res<crate::rooms::ActiveRoom>,
    mut images: ResMut<Assets<Image>>,
) {
    for (mut f, t, player, guest) in &mut fighters {
        let phase = f.combat.attacker.phase();
        let started = phase == Phase::Active && f.last_phase != Phase::Active;
        f.last_phase = phase;
        let (Some(id), Some(facing)) = (f.combat.attacker.attack(), f.combat.attacker.facing())
        else {
            continue;
        };
        if !started {
            continue;
        }
        let feet = t.translation.truncate();
        let mut req = SpawnFx::new(EffectId::SwordSlash, feet).facing(facing);
        let (radius, angle) = match id {
            AttackId::Combo3 | AttackId::Air2 => (38.0, 0.0),
            AttackId::Charged => (44.0, 0.0),
            AttackId::Rising => (34.0, -0.6),
            AttackId::Pogo => (30.0, FRAC_PI_2 as f64),
            _ => (32.0, 0.0),
        };
        req.options.radius = radius;
        req.options.angle = angle * facing as f64;
        if let Some(g) = guest {
            // Guests' blows trail a dull red, so they read as danger rather than as the player's.
            req.options.palette = [
                "3a1418", "6a2028", "9a3036", "c45a4a", "e08a6a", "f0c0a0", "fff0e0",
            ]
            .iter()
            .map(|h| pixelfx::color::hex(u32::from_str_radix(h, 16).unwrap_or(0)))
            .collect();
            req.options.radius *= g.class.scale();
        }
        // The candelabrum's grounded swings sweep fire onto the floor in front, and trail flame.
        if let Some(p) = player
            && p.carried == crate::player::Carried::Candelabrum
        {
            req.options.palette = [
                0x4a1712, 0xc9281f, 0xf2561d, 0xff8a2a, 0xffd36a, 0xfff3a6, 0xffffff,
            ]
            .iter()
            .map(|&c| pixelfx::color::hex(c))
            .collect();
            req.options.radius *= 1.3;
            let grounded = matches!(
                id,
                AttackId::Combo1 | AttackId::Combo2 | AttackId::Combo3 | AttackId::Charged
            );
            if grounded && let Some(room) = &active.room {
                let x = feet.x + facing as f32 * 44.0;
                crate::fire::ignite(&mut commands, &mut images, room, x, -feet.y);
            }
        }
        if player.is_some() || guest.is_some() {
            fx.write(req);
        }
    }
}

/// Turns combat events into sparks, shake, rumble and flashes.
fn impact(
    mut log: ResMut<CombatLog>,
    tuning: Res<Tuning>,
    mut fighters: Query<(
        Entity,
        &mut Fighting,
        Option<&Player>,
        Option<&crate::enemies::Guest>,
    )>,
    mut fx: MessageWriter<SpawnFx>,
    mut shake: ResMut<Shake>,
    gamepads: Query<Entity, With<Gamepad>>,
    mut rumble: MessageWriter<GamepadRumbleRequest>,
    mut slow: ResMut<SlowMotion>,
) {
    let mut pulse = |strong: f32, weak: f32, ms: u64| {
        let k = tuning.juice.rumble;
        if k <= 0.0 {
            return;
        }
        for gamepad in &gamepads {
            rumble.write(GamepadRumbleRequest::Add {
                gamepad,
                duration: Duration::from_millis(ms),
                intensity: GamepadRumbleIntensity {
                    strong_motor: (strong * k).min(1.0),
                    weak_motor: (weak * k).min(1.0),
                },
            });
        }
    };
    let body_of = |id: u64| -> Option<(Body, bool)> {
        let (_, _, p, g) = fighters.get(Entity::from_bits(id)).ok()?;
        let body = *p.map(HasBody::body).or(g.map(HasBody::body))?;
        Some((body, p.is_some()))
    };
    let mut flashes: Vec<(u64, f32)> = Vec::new();
    let mut splats: Vec<u64> = Vec::new();
    for event in log.0.drain(..) {
        match event {
            CombatEvent::Hit(h) => {
                let Some((target, target_is_player)) = body_of(h.target) else {
                    continue;
                };
                let at = centre(&target) + Vec2::new(-(h.direction as f32) * 3.0, -8.0);
                let mut spark = SpawnFx::new(EffectId::HitSpark, at).facing(h.direction);
                spark.options.light = if h.weight == Weight::Light { 0.8 } else { 1.3 };
                fx.write(spark);
                // The view jolts the way the blow went, and down with it for heavy blows.
                let down = if h.weight == Weight::Light { 0.0 } else { -0.5 };
                shake.kick(
                    Vec2::new(h.direction as f32, down),
                    shake_pixels(h.weight) * h.shake,
                );
                match (h.weight, target_is_player) {
                    (_, true) => pulse(0.7, 0.4, 160),
                    (Weight::Light, _) => pulse(0.0, 0.35, 60),
                    (Weight::Heavy, _) => pulse(0.55, 0.2, 110),
                    (Weight::Charged, _) => pulse(0.9, 0.3, 180),
                }
                flashes.push((h.target, 0.07));
            }
            CombatEvent::Parry {
                attacker, defender, ..
            } => {
                if let Some((body, _)) = body_of(defender) {
                    let mut spark = SpawnFx::new(EffectId::HitSpark, centre(&body) + Vec2::Y * 4.0);
                    spark.options.sparks = vec![
                        pixelfx::color::hex(0xffffff),
                        pixelfx::color::hex(0xe8f1ff),
                        pixelfx::color::hex(0xb8c9e8),
                        pixelfx::color::hex(0x62c8f4),
                    ];
                    spark.options.light = 1.6;
                    fx.write(spark);
                }
                shake.kick(Vec2::Y, 3.0);
                pulse(0.4, 0.6, 140);
                flashes.push((attacker, 0.1));
            }
            CombatEvent::WallSplat {
                target,
                incoming_speed,
                ..
            } => {
                if let Some((body, _)) = body_of(target) {
                    let side = incoming_speed.signum();
                    let wall = centre(&body) + Vec2::new(side * body.w as f32 / 2.0, 0.0);
                    fx.write(SpawnFx::new(EffectId::HitSpark, wall).facing(-side as i32));
                    fx.write(SpawnFx::new(EffectId::Poof, wall - Vec2::Y * 16.0));
                }
                shake.kick(Vec2::new(incoming_speed.signum(), 0.0), 4.0);
                pulse(0.8, 0.2, 150);
                splats.push(target);
                flashes.push((target, 0.07));
            }
            CombatEvent::PogoBounce { attacker, .. } => {
                if let Some((body, _)) = body_of(attacker) {
                    let feet = Vec2::new(
                        body.x as f32 + body.w as f32 / 2.0,
                        -(body.y + body.h) as f32,
                    );
                    fx.write(SpawnFx::dust(feet, pixelfx::effects::DustVariant::Jump));
                }
            }
            CombatEvent::BodyHit {
                target, velocity, ..
            } => {
                if let Some((body, _)) = body_of(target) {
                    fx.write(
                        SpawnFx::new(EffectId::HitSpark, centre(&body))
                            .facing(velocity.0.signum() as i32),
                    );
                }
                shake.kick(Vec2::new(velocity.0.signum(), 0.0), 2.0);
                flashes.push((target, 0.07));
            }
        }
    }
    for f in std::mem::take(&mut log.1) {
        if let Some((body, _)) = body_of(f.target) {
            let feet = Vec2::new(
                body.x as f32 + body.w as f32 / 2.0,
                -((body.y + body.h) as f32),
            );
            fx.write(SpawnFx::new(EffectId::HammerSmash, feet).facing(f.direction));
            // Nectar: the sweet, pink payout of a kill.
            let mut nectar = SpawnFx::new(EffectId::Poof, centre(&body));
            nectar.options.palette = [0x5a1838, 0x9a2a5a, 0xd04a86, 0xf27ab0, 0xffb8d8, 0xfff0f8]
                .iter()
                .map(|&c| pixelfx::color::hex(c))
                .collect();
            nectar.options.light = 1.6;
            fx.write(nectar);
        }
        shake.kick(Vec2::new(f.direction as f32, -0.6), 6.0);
        pulse(1.0, 0.6, 260);
        flashes.push((f.target, 0.15));
        slow.left = FINISHER_SLOW.0;
        slow.speed = FINISHER_SLOW.1;
    }
    for (id, secs) in flashes {
        if let Ok((_, mut f, _, _)) = fighters.get_mut(Entity::from_bits(id)) {
            f.flash = f.flash.max(secs);
        }
    }
    for id in splats {
        if let Ok((_, mut f, p, g)) = fighters.get_mut(Entity::from_bits(id)) {
            let x = p.map(|p| p.ctrl.body.x).or(g.map(|g| g.ctrl.body.x));
            f.splat = 0.3;
            f.splat_x = x;
        }
    }
}

fn fade_flash(time: Res<Time>, mut fighters: Query<&mut Fighting>) {
    let dt = time.delta_secs();
    for mut f in &mut fighters {
        if f.flash > 0.0 {
            f.flash = (f.flash - dt).max(0.0);
        }
        if f.splat > 0.0 {
            f.splat = (f.splat - dt).max(0.0);
        }
    }
}

/// The swing pose of an attack, `u` of the way through its current phase.
pub fn swing_of(id: AttackId) -> anim::swing::Swing {
    use anim::swing::Swing;
    match id {
        AttackId::Combo1 => Swing::Combo1,
        AttackId::Combo2 => Swing::Combo2,
        AttackId::Combo3 => Swing::Combo3,
        AttackId::Charged => Swing::Charged,
        AttackId::Rising => Swing::Rising,
        AttackId::Air1 => Swing::Air1,
        AttackId::Air2 => Swing::Air2,
        AttackId::Pogo => Swing::Pogo,
    }
}

/// The pose a fighter's combat state asks for, if any: a swing, a reaction or a charge.
/// `None` leaves the pose to movement.
/// `standing` is whether the body is on the ground and still: the charge only coils then.
pub fn combat_pose(f: &Fighting, charge_ticks: u32, standing: bool) -> Option<anim::Pose> {
    use anim::guest::{Reaction as R, reaction_pose};
    use anim::swing::{Phase as P, swing_pose};
    use world::combat::Reaction;
    let c = &f.combat;
    if f.splat > 0.0 {
        return Some(reaction_pose(R::Splat, 0.0));
    }
    let reaction = match c.reaction() {
        Reaction::Ready => None,
        Reaction::Flinch => Some(R::Flinch),
        Reaction::Stagger => Some(R::Stagger),
        Reaction::Knockdown => Some(R::Knockdown),
        Reaction::Launched => Some(R::Launched),
        Reaction::Dead => Some(R::Dead),
    };
    if let Some(r) = reaction {
        return Some(reaction_pose(r, f.reaction_time as f64));
    }
    let a = &c.attacker;
    if let (Some(id), Some(data)) = (a.attack(), a.data()) {
        let (phase, len) = match a.phase() {
            Phase::Windup => (P::Windup, data.windup),
            Phase::Active => (P::Active, data.active),
            _ => (P::Recovery, data.recovery),
        };
        let u = a.phase_tick() as f64 / len.max(1) as f64;
        return Some(swing_pose(swing_of(id), phase, u));
    }
    // Holding attack after a swing coils for the charged attack.
    if standing && a.held_ticks() > charge_ticks / 4 {
        let u = (a.held_ticks() as f64 / charge_ticks.max(1) as f64).min(1.0);
        return Some(swing_pose(anim::swing::Swing::Charged, P::Windup, u));
    }
    None
}

#[derive(TypePath, Default)]
struct WeaponLoader;

impl AssetLoader for WeaponLoader {
    type Asset = WeaponAsset;
    type Settings = ();
    type Error = WeaponError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _context: &mut LoadContext<'_>,
    ) -> Result<WeaponAsset, WeaponError> {
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .await
            .map_err(|e| WeaponError(e.to_string()))?;
        let text = String::from_utf8(bytes).map_err(|e| WeaponError(e.to_string()))?;
        Weapon::from_ron(&text)
            .map(WeaponAsset)
            .map_err(WeaponError)
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}

#[derive(Debug)]
struct WeaponError(String);

impl std::fmt::Display for WeaponError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "loading weapon: {}", self.0)
    }
}

impl std::error::Error for WeaponError {}
