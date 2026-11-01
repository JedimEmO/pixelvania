//! Guests with shared combat reactions and distinct movement loops.

use anim::guest::{Class, arm, rig, scale};
use bevy::prelude::*;
use pixelfx::EffectId;
use world::combat::{Combatant, Reaction, WeightClass};
use world::{Abilities, Controller, Controls, MoveTuning, Room};

use crate::combat::{
    Fighting, GUEST_TEAM, SimulateBodies, WeaponAsset, Weapons, combat_id, combat_pose,
};
use crate::effects::SpawnFx;
use crate::input::TickInput;
use crate::player::{Look, Player, look_anchor};
use crate::rooms::{ActiveRoom, RoomPart};

pub struct EnemiesPlugin;

impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpawnedFor>()
            .add_systems(
                FixedUpdate,
                (tick, separate)
                    .chain()
                    .in_set(SimulateBodies)
                    .after(crate::player::PlayerTick),
            )
            .add_systems(
                Update,
                (respawn_on_reset, spawn, place, animate, clear_dead).chain(),
            );
    }
}

/// A guest: its class, its movement, and what it is about to do.
#[derive(Component)]
pub struct Guest {
    pub class: Class,
    pub ctrl: Controller,
    tuning: MoveTuning,
    /// Seconds until it may swing again.
    rest: f32,
    /// Seconds left backing away after a swing.
    retreat: f32,
    /// The way it wants to face, kept while it steps backwards.
    face: i32,
    was_swinging: bool,
    /// Varies each guest's timing, so a group does not swing in unison.
    rng: u32,
    /// Feet at the start of the last tick, pixels with y down, for drawing between ticks.
    pub(crate) prev: Vec2,
}

impl Guest {
    /// Feet position in pixels with y down.
    pub(crate) fn feet(&self) -> Vec2 {
        let b = &self.ctrl.body;
        let (x, y) = b.exact_pos();
        Vec2::new(x + b.w as f32 / 2.0, y + b.h as f32)
    }
}

/// How a class fights: reach in pixels, rest between swings in seconds, walking speed,
/// how far it notices the player from, and its health and poise.
struct Temper {
    reach: f32,
    rest: f32,
    speed: f32,
    notice: f32,
    health: f32,
    poise: f32,
}

fn temper(class: Class) -> Temper {
    match class {
        Class::Spinner => Temper {
            reach: 220.0,
            rest: 0.25,
            speed: 190.0,
            notice: 280.0,
            health: 35.0,
            poise: 14.0,
        },
        Class::Toaster => Temper {
            reach: 210.0,
            rest: 1.7,
            speed: 65.0,
            notice: 300.0,
            health: 60.0,
            poise: 30.0,
        },
        Class::Light => Temper {
            reach: 26.0,
            rest: 0.9,
            speed: 100.0,
            notice: 200.0,
            health: 40.0,
            poise: 16.0,
        },
        Class::Medium => Temper {
            reach: 50.0,
            rest: 1.4,
            speed: 70.0,
            notice: 220.0,
            health: 70.0,
            poise: 32.0,
        },
        // The Doorman's own brain decides his moves; these are his walk and his body.
        Class::Doorman => Temper {
            reach: 64.0,
            rest: 1.0,
            speed: 60.0,
            notice: 320.0,
            health: 560.0,
            poise: 140.0,
        },
        Class::Heavy => Temper {
            reach: 44.0,
            rest: 2.2,
            speed: 45.0,
            notice: 240.0,
            health: 150.0,
            poise: 85.0,
        },
    }
}

fn weight(class: Class) -> WeightClass {
    match class {
        Class::Light | Class::Spinner => WeightClass::Light,
        Class::Medium | Class::Toaster => WeightClass::Medium,
        Class::Heavy | Class::Doorman => WeightClass::Heavy,
    }
}

/// The room build the guests were last spawned for, so they are spawned once per build.
#[derive(Resource, Default)]
struct SpawnedFor(Option<u64>);

fn spawn(
    mut commands: Commands,
    active: Res<ActiveRoom>,
    mut spawned: ResMut<SpawnedFor>,
    weapons: Option<Res<Weapons>>,
    mut images: ResMut<Assets<Image>>,
) {
    let (Some(room), Some(weapons)) = (&active.room, weapons) else {
        return;
    };
    if spawned.0 == Some(active.build) {
        return;
    }
    spawned.0 = Some(active.build);
    for def in room.entities_of("guest") {
        let class = match def.props.get("class").map(String::as_str) {
            Some("light") => Class::Light,
            Some("heavy") => Class::Heavy,
            Some("spinner") => Class::Spinner,
            Some("toaster") => Class::Toaster,
            _ => Class::Medium,
        };
        let (x, y) = def.pixel_pos();
        spawn_guest(&mut commands, class, (x, y), &weapons, &mut images);
    }
}

/// Spawns a guest of a class with its feet at `feet`, in pixels with y down.
pub fn spawn_guest(
    commands: &mut Commands,
    class: Class,
    feet: (f32, f32),
    weapons: &Weapons,
    images: &mut Assets<Image>,
) -> Entity {
    let (x, y) = feet;
    let t = temper(class);
    let (w, h) = class.size();
    let mut ctrl = Controller::standing_at(
        x,
        y,
        w,
        h,
        Abilities {
            lunge: false,
            swoon: false,
            cling: false,
            lure: false,
            plunge: false,
        },
    );
    ctrl.facing = -1;
    let tuning = MoveTuning {
        run_speed: t.speed,
        ..MoveTuning::default()
    };
    let weapon = match class {
        Class::Light => weapons.guest_light.clone(),
        Class::Medium => weapons.guest_medium.clone(),
        Class::Heavy => weapons.guest_heavy.clone(),
        Class::Spinner => weapons.guest_spinner.clone(),
        Class::Toaster => weapons.guest_toaster.clone(),
        Class::Doorman => weapons.boss_doorman.clone(),
    };
    let look = Look::new(rig(class), images);
    let image = look.image.clone();
    commands
        .spawn((
            Name::new(format!("{} guest", class.name())),
            RoomPart,
            Guest {
                class,
                ctrl,
                tuning,
                rest: 1.0,
                retreat: 0.0,
                face: -1,
                was_swinging: false,
                rng: (x as u32).wrapping_mul(2_654_435_761) ^ (y as u32).wrapping_mul(40_503),
                prev: Vec2::new(x, y),
            },
            Fighting::new(
                Combatant::new(t.health, t.poise, weight(class)),
                GUEST_TEAM,
                weapon,
            ),
            look,
            Transform::from_xyz(x, -y, 0.9),
            Visibility::default(),
            children![
                (
                    Sprite::from_image(image.clone()),
                    look_anchor(),
                    Transform::default()
                ),
                crate::player::glow_mask(image),
            ],
        ))
        .id()
}

/// Reset brings every guest back, as well as the player.
fn respawn_on_reset(
    mut commands: Commands,
    input: Res<TickInput>,
    guests: Query<Entity, With<Guest>>,
    mut spawned: ResMut<SpawnedFor>,
) {
    if input.reset_pressed {
        for g in &guests {
            commands.entity(g).despawn();
        }
        spawned.0 = None;
    }
}

/// Chooses approach, retreat and the next attack.
fn think(g: &mut Guest, f: &Fighting, player: Option<(f32, f32)>, dt: f32) -> Controls {
    if f.combat.is_frozen() {
        return Controls::default();
    }
    g.rest = (g.rest - dt).max(0.0);
    let swinging = f.combat.attacker.attack().is_some();
    if g.was_swinging && !swinging && g.class == Class::Light {
        g.retreat = 0.35;
    }
    g.was_swinging = swinging;
    let mut c = Controls::default();
    let Some((px, py)) = player else { return c };
    if f.combat.helpless() || swinging {
        return c;
    }
    let t = temper(g.class);
    let (w, h) = g.class.size();
    let x = g.ctrl.body.x as f32 + w as f32 / 2.0;
    let y = (g.ctrl.body.y + h) as f32;
    let dx = px - x;
    if dx.abs() > t.notice || (py - y).abs() > 64.0 {
        return c;
    }
    let dir = dx.signum();
    // Face the player before deciding anything, even when standing still or stepping back.
    g.face = dir as i32;
    g.ctrl.facing = g.face;
    let gap = dx.abs() - 6.0;
    if g.retreat > 0.0 {
        g.retreat -= dt;
        c.x = -dir;
        return c;
    }
    let crowded = matches!(g.class, Class::Medium | Class::Toaster) && gap < t.reach * 0.55;
    if crowded && !(g.class == Class::Toaster && g.rest <= 0.0) {
        c.x = -dir;
    } else if gap <= t.reach && g.rest <= 0.0 && (py - y).abs() < 24.0 {
        c.attack_pressed = true;
        // The next rest is the class's own, give or take a fifth.
        g.rng = g.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let jitter = (g.rng >> 8) as f32 / (1u32 << 24) as f32;
        g.rest = t.rest * (0.8 + 0.4 * jitter);
    } else if gap > t.reach * 0.7 {
        c.x = dir;
    }
    c
}

fn tick(
    mut commands: Commands,
    active: Res<ActiveRoom>,
    tuning: Res<crate::tuning::Tuning>,
    weapons: Res<Assets<WeaponAsset>>,
    mut log: ResMut<crate::combat::CombatLog>,
    players: Query<(&Player, &Fighting), Without<Guest>>,
    mut guests: Query<
        (Entity, &mut Guest, &mut Fighting),
        (Without<Player>, Without<crate::doorman::Doorman>),
    >,
) {
    let Some(room) = &active.room else { return };
    let player = players
        .iter()
        .find(|(_, f)| f.combat.reaction() != Reaction::Dead)
        .map(|(p, _)| {
            let b = &p.ctrl.body;
            (b.x as f32 + b.w as f32 / 2.0, (b.y + b.h) as f32)
        });
    for (e, mut g, mut f) in &mut guests {
        let Some(w) = weapons.get(&f.weapon) else {
            continue;
        };
        let g = &mut *g;
        g.prev = g.feet();
        // Guests share the live movement tuning, gravity included, at their own walking speed.
        g.tuning = MoveTuning {
            run_speed: temper(g.class).speed,
            ..tuning.movement.clone()
        };
        let mut input = think(g, &f, player, world::combat::DT);
        if input.x != 0.0 && matches!(g.class, Class::Spinner | Class::Toaster) {
            let mut ahead = g.ctrl.body;
            ahead.x += input.x.signum() as i32 * 6;
            if g.ctrl.grounded && !ahead.on_ground(room) {
                input.x = 0.0;
            }
        }
        let before = f.combat.attacker.phase();
        let (_, events) =
            f.combat
                .tick_controller(combat_id(e), &mut g.ctrl, room, &g.tuning, &w.0, input);
        log.0.extend(events);
        let phase = f.combat.attacker.phase();
        if !f.combat.is_frozen() && !f.combat.helpless() {
            if g.class == Class::Spinner && phase == world::combat::Phase::Active {
                let mut ahead = g.ctrl.body;
                ahead.x += g.face * 5;
                if ahead.collides_at(room, 0, 0) || !ahead.on_ground(room) {
                    g.face = -g.face;
                }
                g.ctrl
                    .body
                    .move_x(room, g.face as f32 * g.tuning.run_speed * world::combat::DT);
                g.ctrl.facing = g.face;
                f.combat.attacker.redirect(g.face);
            }
            if g.class == Class::Toaster
                && phase == world::combat::Phase::Active
                && before == world::combat::Phase::Windup
            {
                crate::projectiles::throw(&mut commands, g.feet(), player, g.ctrl.facing, &w.0);
            }
        }
        crate::combat::fall_dead(&f, &mut g.ctrl, room, &g.tuning);
        // A body taking a blow turns to face where it came from.
        if let Some(d) = f.hit_from.take()
            && f.combat.helpless()
        {
            g.ctrl.facing = -d;
        }
        // Walking backwards turns the controller round; a guest backing off still faces the player.
        if input.x != 0.0 && f.combat.attacker.attack().is_none() && !f.combat.helpless() {
            g.ctrl.facing = g.face;
        }
    }
}

/// Pushes overlapping bodies apart sideways, so nobody walks through anybody.
/// A guest gives way; if a wall stops it, the player is pushed back instead.
/// The fallen and the thrown pass through, so a launched body can fly past.
fn separate(
    active: Res<ActiveRoom>,
    mut players: Query<(&mut Player, &Fighting), Without<Guest>>,
    mut guests: Query<(&mut Guest, &Fighting), Without<Player>>,
) {
    let Some(room) = &active.room else { return };
    let solid = |f: &Fighting| {
        !matches!(
            f.combat.reaction(),
            Reaction::Dead | Reaction::Launched | Reaction::Knockdown
        )
    };
    let push = |a: &mut world::Body, b: &mut world::Body, room: &Room| {
        let overlap_y = a.y < b.y + b.h && b.y < a.y + a.h;
        let overlap_x = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
        if !overlap_y || overlap_x <= 0 {
            return;
        }
        // Push b away from a's centre, the shorter way.
        let dir = if (b.x * 2 + b.w) >= (a.x * 2 + a.w) {
            1.0
        } else {
            -1.0
        };
        if b.move_x(room, dir * overlap_x as f32) {
            let left = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
            if left > 0 {
                a.move_x(room, -dir * left as f32);
            }
        }
    };
    for (mut p, pf) in &mut players {
        // The Lunge passes through bodies. Once it ends, the push resumes from the far side,
        // which nudges the guest out behind the player rather than back in front.
        if !solid(pf) || p.ctrl.lunging > 0.0 {
            continue;
        }
        for (mut g, gf) in &mut guests {
            if solid(gf)
                && !(g.class == Class::Spinner
                    && gf.combat.attacker.phase() == world::combat::Phase::Active)
            {
                push(&mut p.ctrl.body, &mut g.ctrl.body, room);
            }
        }
    }
    // Guests keep apart from each other too, so they do not stack into one body.
    let mut list: Vec<_> = guests.iter_mut().filter(|(_, f)| solid(f)).collect();
    for i in 0..list.len() {
        for j in i + 1..list.len() {
            let (a, b) = list.split_at_mut(j);
            push(&mut a[i].0.ctrl.body, &mut b[0].0.ctrl.body, room);
        }
    }
}

/// Draws each guest between its last two ticks, on whole pixels, as the player is drawn.
fn place(fixed: Res<Time<Fixed>>, mut guests: Query<(&Guest, &mut Transform)>) {
    let k = fixed.overstep_fraction();
    for (g, mut t) in &mut guests {
        let feet = g.prev.lerp(g.feet(), k).round();
        t.translation.x = feet.x;
        t.translation.y = -feet.y;
    }
}

pub(crate) fn animate(
    time: Res<Time>,
    mut guests: Query<(&Guest, &mut Look, &mut Fighting)>,
    mut images: ResMut<Assets<Image>>,
    mut frame_no: Local<u32>,
) {
    *frame_no = frame_no.wrapping_add(1);
    for (g, mut look, mut f) in &mut guests {
        let look = &mut *look;
        let c = &g.ctrl;
        f.leave_wall(&c.body);
        // Hit-stop freezes the whole body, legs included.
        let dt = if f.combat.is_frozen() {
            0.0
        } else {
            time.delta_secs_f64()
        };
        look.animator.update(
            anim::Motion {
                grounded: c.grounded,
                vx: c.vx,
                vy: c.vy,
                facing: c.facing,
                wall: 0,
                lunging: false,
                swooning: false,
                run_speed: g.tuning.run_speed,
                rope: None,
                plunging: false,
            },
            dt,
        );
        let special = if !f.combat.helpless() {
            let phase = match f.combat.attacker.phase() {
                world::combat::Phase::Windup => anim::swing::Phase::Windup,
                world::combat::Phase::Active => anim::swing::Phase::Active,
                _ => anim::swing::Phase::Recovery,
            };
            let t = f.combat.attacker.phase_tick() as f64 / 120.0;
            match (g.class, crate::doorman::move_of(f.combat.attacker.attack())) {
                (Class::Doorman, Some(m)) => Some(anim::guest::doorman_pose(m, phase, t)),
                _ => anim::guest::loop_pose(g.class, phase, t),
            }
        } else {
            None
        };
        let special = if f.combat.reaction() == Reaction::Dead && !c.grounded {
            Some(anim::guest::reaction_pose(
                anim::guest::Reaction::Launched,
                f.reaction_time() as f64,
            ))
        } else {
            special
        };
        let pose = special
            .or_else(|| combat_pose(&f, u32::MAX, false))
            .unwrap_or_else(|| look.animator.pose());
        let face = if f.combat.helpless() {
            c.facing
        } else {
            f.combat.attacker.facing().unwrap_or(c.facing)
        };
        let pose = anim::Pose {
            face,
            ..arm(scale(&pose, g.class.scale()), g.class)
        };
        look.draw(
            &pose,
            f.flash > 0.0,
            f.combat.shivering(),
            &mut images,
            *frame_no,
        );
    }
}

/// Seconds a defeated guest lies still before it vanishes in a puff.
const DEAD_TIME: f32 = 1.2;

fn clear_dead(
    mut commands: Commands,
    guests: Query<(Entity, &Guest, &Fighting, &Transform)>,
    mut fx: MessageWriter<SpawnFx>,
) {
    for (e, g, f, t) in &guests {
        if f.combat.reaction() == Reaction::Dead && g.ctrl.grounded && f.reaction_time() > DEAD_TIME
        {
            let mut poof = SpawnFx::new(EffectId::Poof, t.translation.truncate());
            poof.options.radius *= g.class.scale();
            fx.write(poof);
            commands.entity(e).despawn();
        }
    }
}
