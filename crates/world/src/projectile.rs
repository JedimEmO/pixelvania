//! Ballistic glass movement in pixels, with y down.

use crate::{Body, Room};

pub const GRAVITY: f32 = 500.0;

#[derive(Debug, Clone)]
pub struct Arc {
    pub body: Body,
    pub vx: f32,
    pub vy: f32,
}

impl Arc {
    /// Advances the arc and reports a tile impact. Body movement cannot tunnel.
    pub fn tick(&mut self, room: &Room, dt: f32) -> bool {
        let dy = self.vy * dt + 0.5 * GRAVITY * dt * dt;
        self.vy += GRAVITY * dt;
        let wall = self.body.move_x(room, self.vx * dt);
        let floor = self.body.move_y(room, dy);
        wall || floor
    }
}

/// An active glass attack carrying the throw's impact values.
pub fn combat(weapon: &crate::combat::Weapon, facing: i32) -> crate::combat::Combatant {
    use crate::combat::{AttackId, Combatant, Hitbox, Rect, WeightClass};
    let mut weapon = weapon.clone();
    let data = &mut weapon.combo[0];
    data.windup = 1;
    data.active = 480;
    data.hitboxes = vec![Hitbox {
        rect: Rect {
            x: -3.0,
            y: -6.0,
            w: 6.0,
            h: 6.0,
        },
        ticks: None,
    }];
    let mut combat = Combatant::new(1.0, 1.0, WeightClass::Light);
    combat.attacker.start(&weapon, AttackId::Combo1, facing);
    combat
        .attacker
        .tick(&weapon, crate::Controls::default(), false, facing, false);
    combat
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room() -> Room {
        Room::parse(
            "[legend]\n# stone\n[tiles]\n########\n#......#\n#......#\n#......#\n########\n",
        )
        .unwrap()
    }

    #[test]
    fn rises_then_falls_and_shatters_on_floor() {
        let mut glass = Arc {
            body: Body::new(40, 40, 5, 5),
            vx: 0.0,
            vy: -80.0,
        };
        for _ in 0..12 {
            assert!(!glass.tick(&room(), 1.0 / 120.0));
        }
        assert!(glass.body.y < 40);
        let mut hit = false;
        for _ in 0..120 {
            if glass.tick(&room(), 1.0 / 120.0) {
                hit = true;
                break;
            }
        }
        assert!(hit);
        assert_eq!(glass.body.y + glass.body.h, 64);
    }

    #[test]
    fn fast_glass_cannot_cross_a_wall() {
        let mut glass = Arc {
            body: Body::new(40, 40, 5, 5),
            vx: 20000.0,
            vy: 0.0,
        };
        assert!(glass.tick(&room(), 1.0 / 120.0));
        assert_eq!(glass.body.x + glass.body.w, 112);
    }
    #[test]
    fn glass_uses_combat_damage_stop_and_parry() {
        use crate::combat::*;
        let weapon =
            Weapon::from_ron(include_str!("../../../assets/combat/guest-toaster.ron")).unwrap();
        for parry in [false, true] {
            let mut glass = combat(&weapon, 1);
            let mut player = Combatant::new(100.0, 50.0, WeightClass::Medium);
            player.tick(
                &weapon,
                crate::Controls {
                    parry_pressed: parry,
                    ..Default::default()
                },
                true,
                -1,
            );
            let b = Body::new(40, 40, 6, 6);
            let events = resolve(
                &mut [
                    Fighter {
                        id: 1,
                        team: 1,
                        combat: &mut glass,
                        body: &b,
                        hurtboxes: &[],
                    },
                    Fighter {
                        id: 2,
                        team: 0,
                        combat: &mut player,
                        body: &b,
                        hurtboxes: &[Rect::from_body(&b)],
                    },
                ],
                ImpactScale::NEUTRAL,
            );
            assert_eq!(events.len(), 1);
            if parry {
                assert!(matches!(events[0], CombatEvent::Parry { .. }));
                assert_eq!(player.health, 100.0);
            } else {
                assert_eq!(player.health, 92.0);
                assert!(player.is_frozen());
                assert_eq!(player.velocity(), (90.0, 0.0));
            }
        }
    }
}
