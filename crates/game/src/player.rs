//! The player: the movement controller on the fixed tick, and how it looks and feels.
//!
//! The movement itself is [`world::Controller`]. This plugin feeds it each tick's input,
//! draws the body between ticks, and turns what happened into squash and stretch,
//! afterimages and rumble. Everything here is placeholder art until the rig arrives.

use std::time::Duration;

use bevy::camera::visibility::RenderLayers;
use bevy::input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest};
use bevy::prelude::*;
use bevy::sprite::Anchor;
use world::{Abilities, Controller, Events};

use pixelfx::EffectId;
use pixelfx::effects::DustVariant;

use crate::effects::SpawnFx;
use crate::input::{TickControls, TickInput, TickSet};
use crate::render::{GLOW_LAYER, Light2d};
use crate::rooms::ActiveRoom;
use crate::tuning::Tuning;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, tick.in_set(TickSet::Simulate))
            .add_systems(
                Update,
                (
                    reset,
                    spawn,
                    place,
                    react,
                    recover_shape,
                    leave_afterimages,
                    fade_afterimages,
                )
                    .chain(),
            );
    }
}

/// Width and height of the body, in pixels.
pub const SIZE: IVec2 = IVec2::new(12, 34);

const BODY_COLOR: Color = Color::srgb(0.55, 0.53, 0.60);
const FRONT_COLOR: Color = Color::srgb(0.80, 0.78, 0.84);

#[derive(Component)]
pub struct Player {
    pub ctrl: Controller,
    /// Exact feet position at the start of the last tick, for interpolation. Pixels, y down.
    prev: Vec2,
    /// What happened in the ticks since the last frame.
    events: Events,
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

/// The lighter strip on the side the player faces.
#[derive(Component)]
pub(crate) struct Front;

#[derive(Component)]
struct Afterimage {
    age: f32,
}

/// Spawns the player at the room's `spawn` entity once a room has loaded.
fn spawn(mut commands: Commands, active: Res<ActiveRoom>, existing: Query<(), With<Player>>) {
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
    };
    let feet = player.feet();
    let size = SIZE.as_vec2();
    commands.spawn((
        Name::new("player"),
        Player {
            prev: feet,
            ..player
        },
        Transform::from_xyz(feet.x, -feet.y, 1.0),
        Visibility::default(),
        children![
            (
                Shape { scale: Vec2::ONE },
                Sprite::from_color(BODY_COLOR, size),
                Anchor::BOTTOM_CENTER,
                Transform::default(),
                children![(
                    Front,
                    Sprite::from_color(FRONT_COLOR, Vec2::new(3.0, size.y - 8.0)),
                    Anchor::BOTTOM_LEFT,
                    Transform::from_xyz(size.x / 2.0 - 3.0, 4.0, 0.1),
                )],
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

fn reset(mut commands: Commands, input: Res<TickInput>, players: Query<Entity, With<Player>>) {
    if input.reset_pressed {
        for p in &players {
            commands.entity(p).despawn();
        }
    }
}

fn tick(
    time: Res<Time>,
    controls: Res<TickControls>,
    tuning: Res<Tuning>,
    active: Res<ActiveRoom>,
    mut players: Query<&mut Player>,
) {
    let Some(room) = &active.room else { return };
    let dt = time.delta_secs();
    for mut p in &mut players {
        p.prev = p.feet();
        let ev = p.ctrl.tick(room, &tuning.movement, controls.0, dt);
        p.events.merge(ev);
    }
}

/// Puts the player at the interpolated position, on whole pixels, and faces the body.
pub(crate) fn place(
    fixed: Res<Time<Fixed>>,
    mut players: Query<(&Player, &mut Transform)>,
    mut fronts: Query<&mut Transform, (With<Front>, Without<Player>)>,
) {
    let t = fixed.overstep_fraction();
    // There is one player; the front strip follows whichever way it faces.
    for (p, mut transform) in &mut players {
        let pos = p.render_feet(t).round();
        transform.translation.x = pos.x;
        transform.translation.y = pos.y;
        let half = SIZE.x as f32 / 2.0;
        for mut front in &mut fronts {
            front.translation.x = if p.ctrl.facing > 0 { half - 3.0 } else { -half };
        }
    }
}

/// Turns the ticks' events into squash, stretch, rumble and dust.
pub(crate) fn react(
    tuning: Res<Tuning>,
    mut players: Query<(&mut Player, &Children, &Transform)>,
    mut fx: MessageWriter<SpawnFx>,
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
    players: Query<(&Player, &Transform)>,
    mut since: Local<f32>,
) {
    *since += time.delta_secs();
    for (p, t) in &players {
        if p.ctrl.lunging <= 0.0 || *since < 1.0 / 60.0 {
            continue;
        }
        *since = 0.0;
        commands.spawn((
            Afterimage { age: 0.0 },
            Sprite::from_color(
                Color::LinearRgba(LinearRgba::new(0.9, 0.35, 0.55, 0.5)),
                SIZE.as_vec2(),
            ),
            Anchor::BOTTOM_CENTER,
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
