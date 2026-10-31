//! The player: the movement controller on the fixed tick, and how it looks and feels.
//!
//! The movement itself is [`world::Controller`]. This plugin feeds it each tick's input,
//! draws the body between ticks as the pixelfx rig, and turns what happened into squash and stretch,
//! afterimages, dust and rumble.

use std::time::Duration;

use anim::{Animator, Motion, Rig};
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite::Anchor;
use world::combat::{Combatant, Reaction, WeightClass};
use world::{Abilities, Controller, Events};

use pixelfx::EffectId;
use pixelfx::effects::DustVariant;

use crate::combat::{Fighting, PLAYER_TEAM, WeaponAsset, Weapons, combat_id, combat_pose};
use crate::effects::SpawnFx;
use crate::input::{TickControls, TickInput};
use crate::render::{GLOW_LAYER, Light2d};
use crate::rooms::ActiveRoom;
use crate::tuning::Tuning;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            tick.in_set(crate::combat::SimulateBodies)
                .in_set(PlayerTick),
        )
        .add_systems(
            Update,
            (
                reset,
                spawn,
                switch_weapon,
                place,
                light_rigs,
                animate,
                react,
                recover_shape,
                leave_afterimages,
                fade_afterimages,
            )
                .chain(),
        );
    }
}

/// The player's three weapons, in the order North steps through them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Carried {
    Sabre,
    Jeroboam,
    Candelabrum,
}

impl Carried {
    fn next(self) -> Self {
        match self {
            Self::Sabre => Self::Jeroboam,
            Self::Jeroboam => Self::Candelabrum,
            Self::Candelabrum => Self::Sabre,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Sabre => "sabre",
            Self::Jeroboam => "Jeroboam",
            Self::Candelabrum => "candelabrum",
        }
    }

    /// The rig's drawing of the weapon, and whether it is held with both hands.
    fn rig_weapon(self) -> (pixelfx::rig::Weapon, bool) {
        match self {
            Self::Sabre => (pixelfx::rig::Weapon::Sword, false),
            Self::Jeroboam => (pixelfx::rig::Weapon::Bottle, true),
            Self::Candelabrum => (pixelfx::rig::Weapon::Candelabrum, true),
        }
    }
}

/// The player's tick, which guests run after so they react to where the player is now.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlayerTick;

/// Width and height of the body, in pixels.
pub const SIZE: IVec2 = IVec2::new(12, 34);

/// The size of the image a rig is drawn into, and where the feet are in it.
/// Large enough for the heaviest guest with its mallet raised.
pub(crate) const LOOK_SIZE: UVec2 = UVec2::new(128, 96);
pub(crate) const LOOK_FEET: [f64; 2] = [64.0, 78.0];

/// The player's health and poise.
const HEALTH: f32 = 100.0;
const POISE: f32 = 60.0;
/// Seconds a defeated player lies still before getting up at the spawn point.
const DEFEAT_TIME: f32 = 1.5;

#[derive(Component)]
pub struct Player {
    pub ctrl: Controller,
    /// Exact feet position at the start of the last tick, for interpolation. Pixels, y down.
    prev: Vec2,
    /// What happened in the ticks since the last frame.
    events: Events,
    /// In a Swoon, from the second jump until the body next stands or clings.
    swooning: bool,
    /// The weapon in hand.
    pub carried: Carried,
}

impl Player {
    /// Feet position, exact, in pixels with y down.
    fn feet(&self) -> Vec2 {
        let (x, y) = self.ctrl.body.exact_pos();
        Vec2::new(x + SIZE.x as f32 / 2.0, y + SIZE.y as f32)
    }

    /// Feet position in world space (y up), interpolated between ticks.
    pub fn render_feet(&self, overstep: f32) -> Vec2 {
        let p = self.prev.lerp(self.feet(), overstep);
        Vec2::new(p.x, -p.y)
    }

    /// Centre of the body in world space (y up), interpolated between ticks.
    pub fn render_centre(&self, overstep: f32) -> Vec2 {
        self.render_feet(overstep) + Vec2::new(0.0, SIZE.y as f32 / 2.0)
    }
}

/// The drawn body, a child of the player, scaled for squash and stretch about the feet.
#[derive(Component)]
pub(crate) struct Shape {
    scale: Vec2,
}

/// The rig, its animation, and the image it is drawn into each frame.
#[derive(Component)]
pub(crate) struct Look {
    pub rig: Rig,
    pub animator: Animator,
    pub frame: pixelfx::Frame,
    pub image: Handle<Image>,
}

impl Look {
    pub fn new(rig: Rig, images: &mut Assets<Image>) -> Self {
        Self {
            rig,
            animator: Animator::default(),
            frame: pixelfx::Frame::new(LOOK_SIZE.x, LOOK_SIZE.y),
            image: images.add(look_image()),
        }
    }

    /// Draws a pose and uploads it. A flashing body is drawn as a white silhouette;
    /// a shivering one is nudged a pixel either way on alternate frames.
    pub fn draw(
        &mut self,
        pose: &anim::Pose,
        flash: bool,
        shiver: bool,
        images: &mut Assets<Image>,
        frame_no: u32,
    ) {
        self.frame.clear();
        let nudge = if shiver {
            if frame_no.is_multiple_of(2) {
                1.0
            } else {
                -1.0
            }
        } else {
            0.0
        };
        anim::draw(
            &self.rig,
            pose,
            &mut self.frame,
            [LOOK_FEET[0] + nudge, LOOK_FEET[1]],
        );
        if let Some(mut image) = images.get_mut(&self.image) {
            image.data = Some(if flash {
                self.frame
                    .color
                    .iter()
                    .flat_map(|c| {
                        if c[3] > 0 {
                            [255, 255, 255, 255]
                        } else {
                            [0; 4]
                        }
                    })
                    .collect()
            } else {
                self.frame.color.as_flattened().to_vec()
            });
        }
    }
}

pub(crate) fn look_image() -> Image {
    Image::new_fill(
        Extent3d {
            width: LOOK_SIZE.x,
            height: LOOK_SIZE.y,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

/// Puts the image's feet point on the entity's position.
pub(crate) fn look_anchor() -> Anchor {
    Anchor(Vec2::new(
        LOOK_FEET[0] as f32 / LOOK_SIZE.x as f32 - 0.5,
        0.5 - LOOK_FEET[1] as f32 / LOOK_SIZE.y as f32,
    ))
}

#[derive(Component)]
struct Afterimage {
    age: f32,
}

/// Spawns the player at the room's `spawn` entity once a room has loaded.
fn spawn(
    mut commands: Commands,
    active: Res<ActiveRoom>,
    existing: Query<(), With<Player>>,
    mut images: ResMut<Assets<Image>>,
    weapons: Res<Weapons>,
) {
    let Some(room) = &active.room else { return };
    if !existing.is_empty() {
        return;
    }
    let (x, y) = room
        .entities_of("spawn")
        .next()
        .map(|s| s.pixel_pos())
        .unwrap_or((32.0, 32.0));
    let ctrl = Controller::standing_at(x, y, SIZE.x, SIZE.y, Abilities::ALL);
    let player = Player {
        ctrl,
        prev: Vec2::ZERO,
        events: Events::default(),
        swooning: false,
        carried: Carried::Sabre,
    };
    let feet = player.feet();
    let size = SIZE.as_vec2();
    let look = Look::new(anim::player_rig(), &mut images);
    let image = look.image.clone();
    commands.spawn((
        Name::new("player"),
        Player {
            prev: feet,
            ..player
        },
        look,
        Fighting::new(
            Combatant::new(HEALTH, POISE, WeightClass::Medium),
            PLAYER_TEAM,
            weapons.sabre.clone(),
        ),
        Transform::from_xyz(feet.x, -feet.y, 1.0),
        Visibility::default(),
        children![
            (
                Shape { scale: Vec2::ONE },
                Sprite::from_image(image),
                look_anchor(),
                Transform::default(),
            ),
            (
                // A faint light of the player's own, so they never vanish in the dark.
                Light2d {
                    color: LinearRgba::rgb(1.0, 0.85, 0.75),
                    radius: 48.0,
                    strength: 0.35,
                    glow: 0.0,
                },
                Transform::from_xyz(0.0, size.y * 0.6, 0.0),
            ),
        ],
    ));
}

/// Sends the player back to the spawn point on Reset, or after lying defeated for a while.
fn reset(
    mut commands: Commands,
    input: Res<TickInput>,
    players: Query<(Entity, &Fighting), With<Player>>,
) {
    for (p, f) in &players {
        let defeated = f.combat.reaction() == Reaction::Dead && f.reaction_time() > DEFEAT_TIME;
        if input.reset_pressed || defeated {
            commands.entity(p).despawn();
        }
    }
}

fn tick(
    fixed: Res<Time<Fixed>>,
    controls: Res<TickControls>,
    tuning: Res<Tuning>,
    active: Res<ActiveRoom>,
    weapons: Res<Assets<WeaponAsset>>,
    mut log: ResMut<crate::combat::CombatLog>,
    mut players: Query<(Entity, &mut Player, &mut Fighting)>,
) {
    let Some(room) = &active.room else { return };
    // The tick's own length, so a tick stepped by hand while paused is a whole tick.
    let dt = fixed.timestep().as_secs_f32();
    for (e, mut p, mut f) in &mut players {
        let p = &mut *p;
        p.prev = p.feet();
        // Combat drives movement once the weapon has loaded: swings lock it, hits take it over.
        let ev = match weapons.get(&f.weapon) {
            Some(w) => {
                let (ev, combat_events) = f.combat.tick_controller(
                    combat_id(e),
                    &mut p.ctrl,
                    room,
                    &tuning.movement,
                    &w.0,
                    controls.0,
                );
                log.0.extend(combat_events);
                ev
            }
            None => p.ctrl.tick(room, &tuning.movement, controls.0, dt),
        };
        crate::combat::fall_dead(&f, &mut p.ctrl, room, &tuning.movement);
        // A body taking a blow turns to face where it came from.
        if let Some(d) = f.hit_from.take()
            && f.combat.helpless()
        {
            p.ctrl.facing = -d;
        }
        if ev.swooned {
            p.swooning = true;
        } else if p.ctrl.grounded || p.ctrl.wall != 0 {
            p.swooning = false;
        }
        p.events.merge(ev);
    }
}

/// Puts the player at the interpolated position, on whole pixels.
pub(crate) fn place(fixed: Res<Time<Fixed>>, mut players: Query<(&Player, &mut Transform)>) {
    let t = fixed.overstep_fraction();
    for (p, mut transform) in &mut players {
        let pos = p.render_feet(t).round();
        transform.translation.x = pos.x;
        transform.translation.y = pos.y;
    }
}

/// Poses the rig from how the player is moving and draws it.
pub(crate) fn animate(
    time: Res<Time>,
    tuning: Res<Tuning>,
    weapons: Res<Assets<WeaponAsset>>,
    mut players: Query<(&Player, &mut Look, &mut Fighting)>,
    mut images: ResMut<Assets<Image>>,
    mut frame_no: Local<u32>,
) {
    *frame_no = frame_no.wrapping_add(1);
    for (p, mut look, mut f) in &mut players {
        let look = &mut *look;
        let c = &p.ctrl;
        f.leave_wall(&c.body);
        // Hit-stop freezes the whole body, legs included.
        let dt = if f.combat.is_frozen() {
            0.0
        } else {
            time.delta_secs_f64()
        };
        look.animator.update(
            Motion {
                grounded: c.grounded,
                vx: c.vx,
                vy: c.vy,
                facing: c.facing,
                wall: c.wall,
                lunging: c.lunging > 0.0,
                swooning: p.swooning,
                run_speed: tuning.movement.run_speed,
            },
            dt,
        );
        let charge = weapons.get(&f.weapon).map_or(48, |w| w.0.charge_ticks);
        let standing = c.grounded && c.vx.abs() < 10.0;
        let pose = combat_pose(&f, charge, standing).unwrap_or_else(|| carry(look.animator.pose()));
        // Reeling bodies face the way the controller says, which a blow has turned.
        let face = if f.combat.helpless() {
            c.facing
        } else {
            f.combat
                .attacker
                .facing()
                .unwrap_or(look.animator.pose().face)
        };
        let pose = anim::Pose { face, ..pose };
        // Swings are posed for the sabre; the heavier weapons take both hands.
        let (weapon, two) = p.carried.rig_weapon();
        let pose = if pose.weapon.is_some() {
            anim::Pose {
                weapon: Some(weapon),
                two: pose.two || two,
                ..pose
            }
        } else {
            pose
        };
        let flash = f.flash > 0.0;
        look.draw(&pose, flash, f.combat.shivering(), &mut images, *frame_no);
    }
}

/// How quickly a rig's shading turns towards a new light, per second.
const LIGHT_TURN: f32 = 6.0;

/// Shades every rig from the light that reaches it most strongly, effects' lights included,
/// so a guest beside a candle is lit from the candle's side. With no light in reach,
/// the shading drifts back to the default, from the upper left.
pub(crate) fn light_rigs(
    time: Res<Time>,
    lights: Query<(&GlobalTransform, &Light2d)>,
    mut looks: Query<(&Transform, &mut Look)>,
) {
    let fallback = Vec3::new(-0.55, 0.72, 0.5);
    let k = 1.0 - (-LIGHT_TURN * time.delta_secs()).exp();
    for (t, mut look) in &mut looks {
        let chest = t.translation.truncate() + Vec2::Y * 22.0;
        let mut best: Option<(f32, Vec2)> = None;
        for (lt, l) in &lights {
            let to = lt.translation().truncate() - chest;
            let d = to.length();
            // A body's own light sits on it and says nothing about direction.
            if d < 8.0 || d >= l.radius {
                continue;
            }
            let reach = l.strength * (1.0 - d / l.radius);
            if best.is_none_or(|(b, _)| reach > b) {
                best = Some((reach, to / d));
            }
        }
        let want = match best {
            Some((_, dir)) => Vec3::new(dir.x, dir.y, 0.6).normalize(),
            None => fallback,
        };
        let now = Vec3::new(
            look.rig.light[0] as f32,
            look.rig.light[1] as f32,
            look.rig.light[2] as f32,
        );
        let next = now.lerp(want, k).normalize_or(fallback);
        look.rig.light = [next.x as f64, next.y as f64, next.z as f64];
    }
}

/// Steps to the next weapon, between swings.
fn switch_weapon(
    input: Res<TickInput>,
    weapons: Res<Weapons>,
    mut players: Query<(&mut Player, &mut Fighting)>,
) {
    if !input.switch_weapon_pressed {
        return;
    }
    for (mut p, mut f) in &mut players {
        if f.combat.attacker.attack().is_some() || f.combat.helpless() {
            continue;
        }
        p.carried = p.carried.next();
        f.weapon = match p.carried {
            Carried::Sabre => weapons.sabre.clone(),
            Carried::Jeroboam => weapons.jeroboam.clone(),
            Carried::Candelabrum => weapons.candelabrum.clone(),
        };
        info!("now carrying the {}", p.carried.name());
    }
}

/// Puts the weapon in the hand of a movement pose, trailing down and behind.
fn carry(pose: anim::Pose) -> anim::Pose {
    anim::Pose {
        weapon: Some(pixelfx::rig::Weapon::Sword),
        sa: 2.75,
        ..pose
    }
}

/// Turns the ticks' events into squash, stretch, rumble and dust.
pub(crate) fn react(
    tuning: Res<Tuning>,
    mut players: Query<(&mut Player, &Children, &Transform)>,
    mut fx: MessageWriter<SpawnFx>,
    mut shake: ResMut<crate::camera::Shake>,
    mut shapes: Query<&mut Shape>,
    gamepads: Query<Entity, With<Gamepad>>,
    mut rumble: MessageWriter<GamepadRumbleRequest>,
) {
    let j = &tuning.juice;
    let m = &tuning.movement;
    for (mut p, children, transform) in &mut players {
        let ev = std::mem::take(&mut p.events);
        let feet = transform.translation.truncate();
        let facing = p.ctrl.facing;
        let mut set_shape = |s: (f32, f32)| {
            for &c in children {
                if let Ok(mut shape) = shapes.get_mut(c) {
                    shape.scale = Vec2::new(s.0, s.1);
                }
            }
        };
        let mut pulse = |strong: f32, weak: f32, ms: u64| {
            if j.rumble <= 0.0 {
                return;
            }
            for gamepad in &gamepads {
                rumble.write(GamepadRumbleRequest::Add {
                    gamepad,
                    duration: Duration::from_millis(ms),
                    intensity: GamepadRumbleIntensity {
                        strong_motor: (strong * j.rumble).min(1.0),
                        weak_motor: (weak * j.rumble).min(1.0),
                    },
                });
            }
        };
        if let Some(speed) = ev.landed {
            // Scale the squash by how hard the landing was.
            let k = ((speed - 80.0) / (m.hard_landing_speed - 80.0)).clamp(0.0, 1.0);
            if k > 0.0 {
                set_shape((
                    1.0 + (j.land_squash.0 - 1.0) * k,
                    1.0 + (j.land_squash.1 - 1.0) * k,
                ));
            }
            if speed >= m.hard_landing_speed {
                pulse(0.45, 0.2, 110);
            }
            // A slam from a fast fall cracks the floor; an ordinary landing puffs dust.
            if speed > m.max_fall + 20.0 {
                fx.write(SpawnFx::new(EffectId::HeavyLanding, feet));
                shake.kick(Vec2::NEG_Y, 3.0);
            } else if speed >= 150.0 {
                fx.write(SpawnFx::dust(feet, DustVariant::Land));
            }
        }
        if ev.jumped {
            fx.write(SpawnFx::dust(feet, DustVariant::Jump).facing(facing));
        }
        if let Some(away) = ev.wall_jumped {
            // Kicked off the wall behind the player, at mid height.
            let wall = feet + Vec2::new(-away as f32 * SIZE.x as f32 / 2.0, 10.0);
            fx.write(SpawnFx::dust(wall, DustVariant::Jump).facing(away));
        }
        if ev.lunged && p.ctrl.grounded {
            fx.write(SpawnFx::dust(feet, DustVariant::Run).facing(facing));
        }
        if ev.jumped || ev.wall_jumped.is_some() {
            set_shape(j.jump_stretch);
        }
        if ev.swooned {
            // The Swoon floats: a softer stretch.
            set_shape((0.85, 1.15));
        }
        if ev.wall_jumped.is_some() {
            pulse(0.0, 0.25, 50);
        }
        if ev.lunged {
            set_shape(j.lunge_stretch);
            pulse(0.15, 0.35, 70);
        }
    }
}

fn recover_shape(
    time: Res<Time>,
    tuning: Res<Tuning>,
    mut shapes: Query<(&mut Shape, &mut Transform)>,
) {
    let k = (-tuning.juice.squash_recovery * time.delta_secs()).exp();
    for (mut shape, mut t) in &mut shapes {
        shape.scale = Vec2::ONE + (shape.scale - Vec2::ONE) * k;
        t.scale = shape.scale.extend(1.0);
    }
}

/// While lunging, leaves fading copies of the body in the glow buffer.
fn leave_afterimages(
    mut commands: Commands,
    time: Res<Time>,
    players: Query<(&Player, &Transform, &Look)>,
    mut images: ResMut<Assets<Image>>,
    mut since: Local<f32>,
) {
    *since += time.delta_secs();
    for (p, t, look) in &players {
        if p.ctrl.lunging <= 0.0 || *since < 1.0 / 60.0 {
            continue;
        }
        *since = 0.0;
        // A copy of this frame's body, as a silhouette the sprite colour tints.
        let mut copy = look_image();
        copy.data = Some(
            look.frame
                .color
                .iter()
                .flat_map(|c| {
                    if c[3] > 0 {
                        [255, 255, 255, 255]
                    } else {
                        [0; 4]
                    }
                })
                .collect(),
        );
        commands.spawn((
            Afterimage { age: 0.0 },
            Sprite {
                image: images.add(copy),
                color: Color::LinearRgba(LinearRgba::new(0.9, 0.35, 0.55, 0.5)),
                ..default()
            },
            look_anchor(),
            Transform::from_translation(t.translation.with_z(0.5)),
            RenderLayers::layer(GLOW_LAYER),
        ));
    }
}

fn fade_afterimages(
    mut commands: Commands,
    time: Res<Time>,
    tuning: Res<Tuning>,
    mut images: Query<(Entity, &mut Afterimage, &mut Sprite)>,
) {
    let life = tuning.juice.afterimage_life.max(0.01);
    for (e, mut a, mut sprite) in &mut images {
        a.age += time.delta_secs();
        if a.age >= life {
            commands.entity(e).despawn();
        } else {
            sprite.color.set_alpha(0.5 * (1.0 - a.age / life));
        }
    }
}
