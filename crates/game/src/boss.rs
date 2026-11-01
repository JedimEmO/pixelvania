//! The ballroom's Dancer and its visible health and poise.
use crate::combat::{Fighting, GUEST_TEAM, SimulateBodies, WeaponAsset, Weapons, combat_id};
use crate::player::{Look, Player, look_anchor};
use crate::rooms::{ActiveRoom, RoomPart};
use bevy::{camera::visibility::RenderLayers, prelude::*, sprite::Anchor};
use world::boss::{HEALTH, POISE, Phase};
use world::combat::{AttackId, Combatant, Phase as AttackPhase, Reaction, WeightClass};
use world::{Abilities, Controller, Controls};

pub struct BossPlugin;
impl Plugin for BossPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<BossDefeated>()
            .init_resource::<SpawnedFor>()
            .add_systems(Startup, spawn_hud)
            .add_systems(
                FixedUpdate,
                (tick, separate)
                    .chain()
                    .in_set(SimulateBodies)
                    .after(crate::player::PlayerTick),
            )
            .add_systems(
                Update,
                (spawn_room, draw, draw_wave)
                    .chain()
                    .after(crate::combat::impact),
            )
            .add_systems(PostUpdate, hud.before(TransformSystems::Propagate));
    }
}

#[derive(Message)]
#[allow(dead_code)] // Consumed by the stage reward integration.
pub struct BossDefeated {
    pub kind: String,
    /// Feet in world coordinates, with y up.
    pub at: Vec2,
}

#[derive(Component)]
pub struct Dancer {
    pub ctrl: Controller,
    phase: Phase,
    beat: usize,
    pause: u32,
    age: f64,
    prev: Vec2,
    announced: bool,
}
impl Dancer {
    fn feet(&self) -> Vec2 {
        let (x, y) = self.ctrl.body.exact_pos();
        Vec2::new(
            x + self.ctrl.body.w as f32 / 2.0,
            y + self.ctrl.body.h as f32,
        )
    }
    /// Which phase of the fight it is in.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn finishable(&self, health: f32) -> bool {
        self.phase == Phase::Burnout && self.pause == 0 && world::boss::finisher(health)
    }
}
#[derive(Resource, Default)]
struct SpawnedFor(Option<u64>);

fn spawn_room(
    mut commands: Commands,
    active: Res<ActiveRoom>,
    progress: Res<crate::progress::PlayerProgress>,
    weapons: Option<Res<Weapons>>,
    mut images: ResMut<Assets<Image>>,
    mut spawned: ResMut<SpawnedFor>,
) {
    let (Some(room), Some(weapons)) = (&active.room, weapons) else {
        return;
    };
    if spawned.0 == Some(active.build) {
        return;
    }
    spawned.0 = Some(active.build);
    if progress.0.has(world::Ability::Plunge) {
        return;
    }
    for def in room.entities_of("boss") {
        if def.props.get("kind").is_some_and(|k| k == "dancer") {
            spawn_dancer(&mut commands, def.pixel_pos(), &weapons, &mut images);
        }
    }
}

pub fn spawn_dancer(
    commands: &mut Commands,
    feet: (f32, f32),
    weapons: &Weapons,
    images: &mut Assets<Image>,
) {
    let ctrl = Controller::standing_at(
        feet.0,
        feet.1,
        18,
        51,
        Abilities {
            lunge: false,
            swoon: false,
            cling: false,
            lure: false,
            plunge: false,
        },
    );
    let look = Look::new(anim::boss::rig(), images);
    let image = look.image.clone();
    commands.spawn((
        Name::new("The Dancer"),
        RoomPart,
        Dancer {
            ctrl,
            phase: Phase::Waltz,
            beat: 0,
            pause: 120,
            age: 0.0,
            prev: Vec2::new(feet.0, feet.1),
            announced: false,
        },
        Fighting::new(
            Combatant::new(HEALTH, POISE * 0.75, WeightClass::Heavy),
            GUEST_TEAM,
            weapons.boss_dancer.clone(),
        ),
        look,
        Transform::from_xyz(feet.0, -feet.1, 0.9),
        Visibility::default(),
        children![
            (
                Sprite::from_image(image.clone()),
                look_anchor(),
                Transform::default()
            ),
            crate::player::glow_mask(image),
            (
                Shockwave,
                Sprite::from_color(Color::srgb(1.0, 0.73, 0.25), Vec2::new(300.0, 8.0)),
                Transform::from_xyz(0.0, 4.0, 0.2),
                Visibility::Hidden,
                RenderLayers::layer(crate::render::GLOW_LAYER)
            )
        ],
    ));
}

fn tick(
    mut commands: Commands,
    active: Res<ActiveRoom>,
    tuning: Res<crate::tuning::Tuning>,
    weapons: Res<Assets<WeaponAsset>>,
    players: Query<(&Player, &Fighting), Without<Dancer>>,
    mut dancers: Query<(Entity, &mut Dancer, &mut Fighting), Without<Player>>,
    mut log: ResMut<crate::combat::CombatLog>,
    mut shake: ResMut<crate::camera::Shake>,
    mut fx: MessageWriter<crate::effects::SpawnFx>,
    mut defeated: MessageWriter<BossDefeated>,
    mut sfx: MessageWriter<crate::audio::PlaySfx>,
) {
    let Some(room) = &active.room else { return };
    let target = players
        .iter()
        .find(|(_, f)| f.combat.health > 0.0)
        .map(|(p, _)| p.ctrl.body.x as f32 + 6.0);
    for (entity, mut dancer, mut f) in &mut dancers {
        let Some(asset) = weapons.get(&f.weapon) else {
            continue;
        };
        let d = &mut *dancer;
        d.prev = d.feet();
        let mut weapon = asset.0.clone();
        if !f.combat.is_frozen() && f.combat.health > 0.0 {
            let phase = Phase::at(f.combat.health);
            if phase != d.phase {
                d.phase = phase;
                d.beat = 0;
                d.pause = 132;
                f.combat.stagger();
                f.flash = 0.22;
                sfx.write(crate::audio::PlaySfx::new(crate::audio::Sfx::DancerPhase));
                let mut flare = crate::effects::SpawnFx::new(
                    pixelfx::EffectId::Poof,
                    Vec2::new(d.feet().x, -d.feet().y + 48.0),
                );
                flare.options.radius = 48.0;
                flare.options.light = 2.5;
                fx.write(flare);
                shake.kick(Vec2::new(0.0, 1.0), 3.0);
            }
            d.age += world::combat::DT as f64;
            d.pause = d.pause.saturating_sub(1);
            f.combat.poise_regen_per_second = d.phase.regen();
            if d.pause == 0
                && !f.combat.helpless()
                && f.combat.attacker.attack().is_none()
                && let Some(px) = target
            {
                d.ctrl.facing = if px < d.feet().x { -1 } else { 1 };
                let id = d.phase.next(d.beat);
                let index = match id {
                    AttackId::Combo2 => 1,
                    AttackId::Combo3 => 2,
                    _ => 0,
                };
                weapon.combo[index].recovery = d.phase.recovery(d.beat);
                if d.phase != Phase::Waltz {
                    weapon.combo[index].windup = (weapon.combo[index].windup * 3 / 4).max(30);
                }
                if d.phase == Phase::Waltz
                    && id == AttackId::Combo1
                    && (px - d.feet().x).abs() > 76.0
                {
                    let pulse =
                        0.45 + 0.55 * (d.age * std::f64::consts::TAU * 2.0).cos().abs() as f32;
                    d.ctrl.body.move_x(
                        room,
                        d.ctrl.facing as f32 * 115.0 * pulse * world::combat::DT,
                    );
                } else {
                    f.combat.attacker.start(&weapon, id, d.ctrl.facing);
                    d.beat += 1;
                    use crate::audio::{PlaySfx, Sfx};
                    sfx.write(PlaySfx::new(match id {
                        AttackId::Combo2 => Sfx::DancerLeap,
                        AttackId::Combo3 => Sfx::DancerDash,
                        _ if d.phase == Phase::Burnout => Sfx::DancerFlurry,
                        _ => Sfx::DancerSweep,
                    }));
                }
            }
        }
        if !f.combat.is_frozen()
            && f.combat.attacker.phase() == AttackPhase::Windup
            && f.combat.attacker.phase_tick().is_multiple_of(6)
        {
            let a = &f.combat.attacker;
            let feet = Vec2::new(d.feet().x, -d.feet().y);
            let u = a.phase_tick() as f32 / a.data().map_or(1.0, |a| a.windup as f32);
            match a.attack() {
                Some(AttackId::Combo2) => {
                    fx.write(crate::effects::SpawnFx::dust(
                        feet,
                        pixelfx::effects::DustVariant::Jump,
                    ));
                }
                Some(AttackId::Combo1) => {
                    let flurry = d.phase == Phase::Burnout;
                    for i in 0..if flurry { 2 } else { 8 } {
                        let angle = i as f32 * std::f32::consts::TAU / 8.0;
                        let offset = if flurry {
                            Vec2::new(if i == 0 { -18.0 } else { 18.0 }, 45.0)
                        } else {
                            Vec2::new(angle.cos(), angle.sin() * 0.2) * (12.0 + u * 44.0)
                        };
                        crate::combat::glow_glint(
                            &mut commands,
                            feet + offset,
                            Vec2::ZERO,
                            if flurry { 0.035 } else { 0.08 },
                        );
                    }
                }
                _ => {}
            }
        }
        let before = f.combat.attacker.phase();
        let airborne_leap = f.combat.attacker.attack() == Some(AttackId::Combo2)
            && before == AttackPhase::Active
            && f.combat.attacker.phase_tick() < 72;
        let mut movement = tuning.movement.clone();
        if airborne_leap {
            movement.gravity = 0.0;
            d.ctrl.vy = 0.0;
        }
        let (_, events) = f.combat.tick_controller(
            combat_id(entity),
            &mut d.ctrl,
            room,
            &movement,
            &weapon,
            Controls::default(),
        );
        log.0.extend(events);
        if !f.combat.is_frozen() && !f.combat.helpless() && d.pause == 0 {
            let phase = f.combat.attacker.phase();
            let tick = f.combat.attacker.phase_tick();
            if phase == AttackPhase::Active {
                match f.combat.attacker.attack() {
                    Some(AttackId::Combo1) => {
                        let speed = if d.phase == Phase::Waltz { 85.0 } else { 135.0 };
                        d.ctrl
                            .body
                            .move_x(room, d.ctrl.facing as f32 * speed * world::combat::DT);
                    }
                    Some(AttackId::Combo2) => {
                        // A 0.6-second arc; the final active ticks are the floor shockwave.
                        if tick < 72 {
                            d.ctrl.body.move_y(room, world::boss::leap_step(tick));
                            d.ctrl.vy = 0.0;
                        }
                        if tick == 72 {
                            fx.write(crate::effects::SpawnFx::new(
                                pixelfx::EffectId::HeavyLanding,
                                Vec2::new(d.feet().x, -d.feet().y),
                            ));
                            shake.kick(Vec2::Y, 2.0);
                        }
                    }
                    Some(AttackId::Combo3) => {
                        d.ctrl.body.move_x(
                            room,
                            d.ctrl.facing as f32 * room.width as f32 * world::TILE as f32
                                / f.combat
                                    .attacker
                                    .data()
                                    .map_or(1.0, |a| a.active as f32 * world::combat::DT)
                                * world::combat::DT,
                        );
                    }
                    _ => {}
                }
            } else if phase == AttackPhase::Windup && before != AttackPhase::Active {
                d.ctrl.vx = 0.0;
            }
        }
        crate::combat::fall_dead(&f, &mut d.ctrl, room, &tuning.movement);
        if f.combat.reaction() == Reaction::Dead && !d.announced {
            d.announced = true;
            defeated.write(BossDefeated {
                kind: "dancer".into(),
                at: Vec2::new(d.feet().x, -d.feet().y),
            });
        }
    }
}

fn draw(
    fixed: Res<Time<Fixed>>,
    mut dancers: Query<(&Dancer, &Fighting, &mut Look, &mut Transform)>,
    mut images: ResMut<Assets<Image>>,
    mut frame: Local<u32>,
) {
    use anim::boss::PoseKind::*;
    *frame = frame.wrapping_add(1);
    for (d, f, mut look, mut transform) in &mut dancers {
        let feet = d.prev.lerp(d.feet(), fixed.overstep_fraction()).round();
        transform.translation.x = feet.x;
        transform.translation.y = -feet.y;
        let a = &f.combat.attacker;
        let kind = if f.combat.reaction() == Reaction::Dead {
            Death
        } else if d.pause > 0 && d.phase != Phase::Waltz {
            PhaseChange
        } else if f.combat.helpless() || d.pause > 0 {
            Stagger
        } else {
            match (a.attack(), a.phase()) {
                (Some(AttackId::Combo2), AttackPhase::Windup) => LeapWindup,
                (Some(AttackId::Combo2), AttackPhase::Active) if a.phase_tick() < 72 => Leap,
                (Some(AttackId::Combo2), AttackPhase::Active) => Exhausted,
                (Some(AttackId::Combo3), AttackPhase::Windup) => DashWindup,
                (Some(AttackId::Combo3), AttackPhase::Active) => Dash,
                (Some(AttackId::Combo1), AttackPhase::Windup) if d.phase == Phase::Burnout => {
                    FlurryWindup
                }
                (Some(_), AttackPhase::Windup) => SweepWindup,
                (Some(_), AttackPhase::Active) => Sweep,
                (_, AttackPhase::Recovery) => Exhausted,
                _ => Idle,
            }
        };
        let time = if f.combat.reaction() == Reaction::Dead {
            f.reaction_time() as f64
        } else if a.attack().is_some() {
            a.phase_tick() as f64 / 120.0
        } else {
            d.age
        };
        let mut pose = anim::boss::pose(kind, time);
        pose.face = d.ctrl.facing;
        look.rig.ribbons = if d.pause > 0 && d.phase != Phase::Waltz {
            52.0
        } else {
            30.0
        };
        look.draw(
            &pose,
            f.flash > 0.0,
            f.combat.shivering(),
            &mut images,
            *frame,
        );
    }
}

#[derive(Component)]
struct BossHud(u8);
fn spawn_hud(mut commands: Commands) {
    for i in 0..4 {
        commands.spawn((
            BossHud(i),
            Sprite::from_color(
                if i < 2 {
                    Color::srgb(0.15, 0.08, 0.14)
                } else if i == 2 {
                    Color::srgb(1.0, 0.22, 0.55)
                } else {
                    Color::srgb(1.0, 0.78, 0.3)
                },
                Vec2::new(160.0, if i % 2 == 0 { 3.0 } else { 1.0 }),
            ),
            Anchor::CENTER_LEFT,
            Transform::default(),
            RenderLayers::layer(crate::render::OVERLAY_LAYER),
        ));
    }
    commands.spawn((
        BossHud(4),
        Text2d::new("THE DANCER"),
        TextFont {
            font_size: bevy::text::FontSize::Px(9.0),
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.82, 0.4)),
        TextLayout::justify(Justify::Center),
        Transform::default(),
        RenderLayers::layer(crate::render::OVERLAY_LAYER),
    ));
}
fn hud(
    focus: Res<crate::render::CameraFocus>,
    dancers: Query<&Fighting, With<Dancer>>,
    mut bars: Query<(&BossHud, &mut Transform, &mut Visibility)>,
) {
    let fighter = dancers.iter().find(|f| f.combat.health > 0.0);
    for (bar, mut t, mut visibility) in &mut bars {
        *visibility = if fighter.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let Some(f) = fighter else { continue };
        let offset = if bar.0 == 4 {
            Vec2::new(0.0, 23.0)
        } else {
            Vec2::new(-80.0, if bar.0 % 2 == 0 { 14.0 } else { 9.0 })
        };
        t.translation =
            (focus.0.round() + Vec2::new(0.0, -(crate::render::HEIGHT as f32) / 2.0) + offset)
                .extend(6.0 + bar.0 as f32 * 0.01);
        let k = match bar.0 {
            2 => f.combat.health / HEALTH,
            3 => f.combat.poise / f.combat.max_poise,
            _ => 1.0,
        };
        t.scale.x = (k.clamp(0.0, 1.0) * 160.0).round() / 160.0;
    }
}

#[derive(Component)]
struct Shockwave;
fn draw_wave(
    active: Res<ActiveRoom>,
    dancers: Query<(&Dancer, &Fighting)>,
    mut waves: Query<(&ChildOf, &mut Visibility, &mut Sprite, &mut Transform), With<Shockwave>>,
) {
    for (parent, mut visible, mut sprite, mut transform) in &mut waves {
        let Ok((d, f)) = dancers.get(parent.parent()) else {
            continue;
        };
        let a = &f.combat.attacker;
        let leap = a.attack() == Some(AttackId::Combo2);
        let dash = a.attack() == Some(AttackId::Combo3);
        let windup = a.phase() == AttackPhase::Windup;
        let landing = leap && a.phase() == AttackPhase::Active && a.phase_tick() >= 72;
        let marker = leap && (windup || a.phase() == AttackPhase::Active);
        let on = landing || marker || (dash && windup);
        *visible = if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let width = if dash {
            active
                .room
                .as_ref()
                .map_or(300.0, |r| r.width as f32 * world::TILE as f32)
        } else if landing {
            300.0
        } else {
            96.0
        };
        sprite.custom_size = Some(Vec2::new(width, if landing { 8.0 } else { 2.0 }));
        transform.translation.x = if dash {
            d.ctrl.facing as f32 * width / 2.0
        } else {
            0.0
        };
        let tick = a.phase_tick();
        let height = if leap && a.phase() == AttackPhase::Active && tick < 72 {
            -4.0 * 48.0 * (tick as f32 / 72.0) * (1.0 - tick as f32 / 72.0)
        } else {
            0.0
        };
        transform.translation.y = 2.0 + height;
        let alpha = if landing {
            1.0 - (tick - 72) as f32 / 30.0
        } else {
            0.4 + 0.6 * (tick as f32 * 0.2).sin().abs()
        };
        sprite.color = Color::srgba(1.0, 0.85, 0.55, alpha);
    }
}

fn separate(
    active: Res<ActiveRoom>,
    dancers: Query<(&Dancer, &Fighting), Without<Player>>,
    mut players: Query<(&mut Player, &Fighting), Without<Dancer>>,
) {
    let Some(room) = &active.room else { return };
    for (mut player, fighting) in &mut players {
        if player.ctrl.lunging > 0.0 || fighting.combat.helpless() {
            continue;
        }
        for (d, f) in &dancers {
            if f.combat.health <= 0.0
                || (f.combat.attacker.attack() == Some(AttackId::Combo3)
                    && f.combat.attacker.phase() == AttackPhase::Active)
            {
                continue;
            }
            let a = &mut player.ctrl.body;
            let b = &d.ctrl.body;
            let overlap = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
            if overlap > 0 && a.y < b.y + b.h && b.y < a.y + a.h {
                let dir = if a.x * 2 + a.w < b.x * 2 + b.w {
                    -1.0
                } else {
                    1.0
                };
                a.move_x(room, dir * overlap as f32);
            }
        }
    }
}
