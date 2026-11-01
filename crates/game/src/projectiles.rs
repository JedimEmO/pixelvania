//! Thrown coupes share combat impacts and shatter against tiles or a swing.

use bevy::prelude::*;
use world::Body;
use world::combat::{Combatant, Fighter, Rect, Weapon, resolve};

use crate::audio::{PlaySfx, Sfx};
use crate::combat::{CombatLog, Fighting, GUEST_TEAM, Mood, SimulateBodies, combat_id};
use crate::effects::SpawnFx;
use crate::player::Player;
use crate::rooms::{ActiveRoom, RoomPart};

pub struct ProjectilesPlugin;

impl Plugin for ProjectilesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            tick.in_set(crate::input::TickSet::Simulate)
                .after(SimulateBodies)
                .before(crate::combat::resolve_contacts),
        );
    }
}

#[derive(Component)]
struct Glass {
    arc: world::projectile::Arc,
    combat: Combatant,
    age: u32,
}

pub fn throw(
    commands: &mut Commands,
    feet: Vec2,
    target: Option<(f32, f32)>,
    facing: i32,
    weapon: &Weapon,
) {
    let x = feet.x + facing as f32 * 8.0;
    let y = feet.y - 39.0;
    let dx = target.map_or(facing as f32 * 150.0, |p| p.0 - x);
    let direction = if dx < 0.0 { -1 } else { 1 };
    let combat = world::projectile::combat(weapon, direction);
    commands.spawn((
        RoomPart,
        Glass {
            arc: world::projectile::Arc {
                body: Body::new(x.round() as i32 - 3, y.round() as i32 - 3, 6, 6),
                vx: (dx / 0.7).clamp(-280.0, 280.0),
                vy: -145.0,
            },
            combat,
            age: 0,
        },
        Transform::from_xyz(x, -y, 1.1),
        Visibility::default(),
        children![
            (
                Sprite::from_color(Color::srgb(0.8, 0.87, 0.94), Vec2::new(7.0, 2.0)),
                Transform::from_xyz(0.0, 2.0, 0.0)
            ),
            (
                Sprite::from_color(Color::srgb(0.68, 0.74, 0.83), Vec2::new(1.0, 4.0)),
                Transform::default()
            ),
            (
                Sprite::from_color(Color::srgb(0.8, 0.87, 0.94), Vec2::new(3.0, 1.0)),
                Transform::from_xyz(0.0, -2.0, 0.0)
            ),
        ],
    ));
}

fn tick(
    mut commands: Commands,
    active: Res<ActiveRoom>,
    input: Res<crate::input::TickInput>,
    mood: Res<Mood>,
    mut glasses: Query<(Entity, &mut Glass, &mut Transform)>,
    mut players: Query<(Entity, &Player, &mut Fighting)>,
    mut log: ResMut<CombatLog>,
    mut fx: MessageWriter<SpawnFx>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    let Some(room) = &active.room else { return };
    for (entity, mut glass, mut transform) in &mut glasses {
        if input.reset_pressed {
            commands.entity(entity).despawn();
            continue;
        }
        glass.age += 1;
        let mut shattered = glass.arc.tick(room, world::combat::DT) || glass.age >= 480;
        let body = glass.arc.body;
        let hurt = [Rect::from_body(&body)];
        for (pe, player, mut fighting) in &mut players {
            if shattered {
                break;
            }
            let pb = &player.ctrl.body;
            let feet = (pb.x as f32 + pb.w as f32 / 2.0, (pb.y + pb.h) as f32);
            if fighting
                .combat
                .attacker
                .active_hitboxes(feet)
                .iter()
                .any(|h| h.overlaps(hurt[0]))
            {
                shattered = true;
                break;
            }
            let events = resolve(
                &mut [
                    Fighter {
                        id: combat_id(entity),
                        team: GUEST_TEAM,
                        combat: &mut glass.combat,
                        body: &body,
                        hurtboxes: &[],
                    },
                    Fighter {
                        id: combat_id(pe),
                        team: fighting.team,
                        combat: &mut fighting.combat,
                        body: pb,
                        hurtboxes: &[Rect::from_body(pb)],
                    },
                ],
                mood.scale,
            );
            shattered |= !events.is_empty();
            if let Some(world::combat::CombatEvent::Hit(hit)) = events.first() {
                fighting.struck(hit.direction);
            }
            log.0.extend(events);
        }
        let at = Vec2::new(body.x as f32 + 3.0, -(body.y as f32 + 3.0));
        transform.translation = at.extend(1.1);
        transform.rotation = Quat::from_rotation_z(glass.age as f32 * 0.08);
        if shattered {
            let mut spark = SpawnFx::new(pixelfx::EffectId::HitSpark, at);
            spark.options.radius *= 0.35;
            fx.write(spark);
            sfx.write(PlaySfx {
                pitch: 1.65,
                volume: 0.5,
                ..PlaySfx::new(Sfx::HitLight)
            });
            commands.entity(entity).despawn();
        }
    }
}
