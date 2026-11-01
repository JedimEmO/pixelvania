//! Combat commitments and impact numbers, played headless at 120 Hz.

use world::combat::*;
use world::{Abilities, Body, Controller, Controls, MoveTuning, Room};

fn sabre() -> Weapon {
    Weapon::from_ron(include_str!("../../../assets/combat/sabre.ron")).unwrap()
}
fn jeroboam() -> Weapon {
    Weapon::from_ron(include_str!("../../../assets/combat/jeroboam.ron")).unwrap()
}
fn press() -> Controls {
    Controls {
        attack_pressed: true,
        ..Controls::default()
    }
}
fn hold() -> Controls {
    Controls {
        attack_held: true,
        ..Controls::default()
    }
}
fn guest(weight: WeightClass) -> Combatant {
    Combatant::new(500.0, 100.0, weight)
}
fn room() -> Room {
    let mut source = "[legend]\n# stone\n[tiles]\n".to_owned();
    for _ in 0..12 {
        source.push_str("#..................#\n");
    }
    source.push_str("####################\n");
    Room::parse(&source).unwrap()
}
fn player(x: f32, y: f32) -> Controller {
    Controller::standing_at(x, y, 12, 34, Abilities::ALL)
}

fn tick(a: &mut Attacker, weapon: &Weapon) {
    a.tick(weapon, Controls::default(), true, 1, false);
}
fn active(a: &mut Combatant, weapon: &Weapon, id: AttackId) {
    assert!(a.attacker.start(weapon, id, 1));
    for _ in 0..weapon.attack(id).windup {
        tick(&mut a.attacker, weapon);
    }
    assert_eq!(a.attacker.phase(), Phase::Active);
}
fn contact(a: &mut Combatant, b: &mut Combatant, mood: ImpactScale) -> Vec<CombatEvent> {
    contact_at(
        a,
        b,
        &Body::new(74, 66, 12, 34),
        &Body::new(94, 66, 12, 34),
        mood,
    )
}
fn contact_at(
    a: &mut Combatant,
    b: &mut Combatant,
    ab: &Body,
    bb: &Body,
    mood: ImpactScale,
) -> Vec<CombatEvent> {
    resolve(
        &mut [
            Fighter {
                id: 1,
                team: 0,
                combat: a,
                body: ab,
                hurtboxes: &[Rect::from_body(ab)],
            },
            Fighter {
                id: 2,
                team: 1,
                combat: b,
                body: bb,
                hurtboxes: &[Rect::from_body(bb)],
            },
        ],
        mood,
    )
}
fn hit(events: &[CombatEvent]) -> &Hit {
    events
        .iter()
        .find_map(|e| {
            if let CombatEvent::Hit(h) = e {
                Some(h)
            } else {
                None
            }
        })
        .expect("hit event")
}
fn thaw(a: &mut Combatant, weapon: &Weapon) {
    while a.is_frozen() {
        a.tick(weapon, Controls::default(), true, 1);
    }
}

#[test]
fn ron_files_validate_and_pin_the_weapon_numbers() {
    let expected = [
        (
            sabre(),
            48,
            [
                (5, 8, 14, 12.0, 8.0, 0.04),
                (6, 8, 16, 14.0, 10.0, 0.04),
                (22, 10, 30, 24.0, 26.0, 0.09),
                (28, 12, 40, 40.0, 55.0, 0.16),
                (16, 10, 26, 18.0, 20.0, 0.09),
                (5, 8, 14, 12.0, 8.0, 0.04),
                (16, 10, 24, 20.0, 20.0, 0.09),
                (6, 10, 20, 16.0, 16.0, 0.09),
            ],
        ),
        (
            jeroboam(),
            72,
            [
                (34, 14, 40, 28.0, 35.0, 0.09),
                (40, 14, 44, 34.0, 45.0, 0.09),
                (56, 18, 56, 52.0, 75.0, 0.18),
                (48, 18, 64, 70.0, 160.0, 0.20),
                (38, 14, 44, 32.0, 50.0, 0.09),
                (28, 12, 34, 24.0, 30.0, 0.09),
                (40, 16, 44, 40.0, 55.0, 0.18),
                (30, 16, 42, 32.0, 45.0, 0.18),
            ],
        ),
    ];
    for (weapon, charge, moves) in expected {
        weapon.validate().unwrap();
        assert_eq!(weapon.charge_ticks, charge);
        assert_eq!(
            Weapon::from_ron(&ron::to_string(&weapon).unwrap()).unwrap(),
            weapon
        );
        for (id, numbers) in AttackId::ALL.into_iter().zip(moves) {
            let d = weapon.attack(id);
            assert_eq!(
                (
                    d.windup,
                    d.active,
                    d.recovery,
                    d.damage,
                    d.poise_damage,
                    d.hit_stop
                ),
                numbers
            );
            println!(
                "{} {id:?}: {}/{}/{} ticks, damage {}, poise {}, stop {} s, knockback {:?}",
                weapon.name,
                d.windup,
                d.active,
                d.recovery,
                d.damage,
                d.poise_damage,
                d.hit_stop,
                d.knockback
            );
        }
    }
    let light_combo: f32 = sabre().combo.iter().map(|d| d.poise_damage).sum();
    assert_eq!(light_combo, 44.0);
    assert_eq!(jeroboam().charged.poise_damage, 160.0);
    assert!(jeroboam().charged.poise_damage > light_combo * 3.0);
}

#[test]
fn invalid_frame_data_is_rejected_before_play() {
    let mut weapon = sabre();
    weapon.combo[0].hitboxes[0].ticks = Some(TickRange { start: 3, end: 9 });
    assert!(weapon.validate().is_err());
    weapon = sabre();
    weapon.combo[0].cancel_window = weapon.combo[0].recovery;
    assert!(weapon.validate().is_err());
    weapon = sabre();
    weapon.combo[0].damage = f32::NAN;
    assert!(weapon.validate().is_err());
    weapon = sabre();
    weapon.combo[0].windup = u32::MAX;
    assert!(weapon.validate().is_err());
    assert!(Weapon::from_ron("(name: \"missing moves\")").is_err());
}

#[test]
fn recordings_round_trip_all_combat_flags_and_read_old_inputs() {
    for flags in 0..8 {
        let input = Controls {
            attack_pressed: flags & 1 != 0,
            attack_held: flags & 2 != 0,
            parry_pressed: flags & 4 != 0,
            ..Controls::default()
        };
        assert_eq!(Controls::decode(&input.encode()), Some(input));
    }
    assert_eq!(
        Controls::decode("-0.500 1.000 hjl"),
        Some(Controls {
            x: -0.5,
            y: 1.0,
            jump_held: true,
            jump_pressed: true,
            lunge_pressed: true,
            ..Controls::default()
        })
    );
    assert_eq!(Controls::decode("0 0 -"), Some(Controls::default()));
}

#[test]
fn a_miss_cannot_cancel_and_plays_every_recovery_tick() {
    let weapon = sabre();
    let mut a = Attacker::default();
    a.tick(&weapon, press(), true, 1, false);
    let d = &weapon.combo[0];
    for frame in 0..d.total_ticks() {
        assert_eq!(a.elapsed(), Some(frame));
        assert!(!a.can_lunge_cancel());
        assert!(!a.lunge_happened());
        assert!(!a.start(&weapon, AttackId::Charged, -1));
        assert!(a.motion().locked);
        assert_eq!(
            a.phase(),
            if frame < d.windup {
                Phase::Windup
            } else if frame < d.windup + d.active {
                Phase::Active
            } else {
                Phase::Recovery
            }
        );
        tick(&mut a, &weapon);
    }
    assert_eq!(a.phase(), Phase::Idle);
    assert!(!a.motion().locked);
    println!(
        "missed sabre opener: {} ticks of commitment",
        d.total_ticks()
    );
}

#[test]
fn a_connected_swing_only_allows_lunge_from_its_recovery_window() {
    let weapon = sabre();
    let mut a = guest(WeightClass::Light);
    let mut b = guest(WeightClass::Light);
    active(&mut a, &weapon, AttackId::Combo1);
    assert!(!a.attacker.can_lunge_cancel());
    contact(&mut a, &mut b, ImpactScale::NEUTRAL);
    assert!(!a.attacker.lunge_happened());
    thaw(&mut a, &weapon);
    let first_cancel =
        weapon.combo[0].windup + weapon.combo[0].active + weapon.combo[0].cancel_window;
    while a.attacker.elapsed().unwrap() < first_cancel {
        assert!(!a.attacker.can_lunge_cancel());
        tick(&mut a.attacker, &weapon);
    }
    assert!(a.can_lunge_cancel());
    assert!(a.attacker.lunge_happened());
    assert_eq!(a.attacker.phase(), Phase::Idle);
}

#[test]
fn buffered_recovery_presses_chain_three_committed_hits() {
    let weapon = sabre();
    let mut a = Attacker::default();
    a.tick(&weapon, press(), true, 1, false);
    for (current, next) in [
        (AttackId::Combo1, AttackId::Combo2),
        (AttackId::Combo2, AttackId::Combo3),
    ] {
        let total = weapon.attack(current).total_ticks();
        while a.elapsed().unwrap() < total - 4 {
            tick(&mut a, &weapon);
        }
        a.tick(&weapon, press(), true, 1, false);
        assert_eq!(a.attack(), Some(current));
        while a.attack() == Some(current) {
            tick(&mut a, &weapon);
        }
        assert_eq!(a.attack(), Some(next));
        assert_eq!(a.elapsed(), Some(0));
    }
    assert!(weapon.combo[2].windup > weapon.combo[0].windup);
    assert!(weapon.combo[2].total_ticks() > weapon.combo[0].total_ticks());
    assert_eq!(weapon.combo[2].weight, Weight::Heavy);
}

#[test]
fn early_buffers_expire_but_presses_during_freeze_are_kept() {
    let weapon = sabre();
    let mut a = Attacker::default();
    a.tick(&weapon, press(), true, 1, false);
    a.tick(&weapon, press(), true, 1, false);
    for _ in 0..weapon.combo[0].total_ticks() {
        tick(&mut a, &weapon);
    }
    assert_eq!(a.phase(), Phase::Idle);
    a.start(&weapon, AttackId::Combo1, 1);
    while a.elapsed().unwrap() < weapon.combo[0].total_ticks() - 3 {
        tick(&mut a, &weapon);
    }
    a.tick(&weapon, press(), true, 1, true);
    for _ in 0..60 {
        a.tick(&weapon, Controls::default(), true, 1, true);
    }
    for _ in 0..3 {
        tick(&mut a, &weapon);
    }
    assert_eq!(a.attack(), Some(AttackId::Combo2));
}

#[test]
fn charge_requires_the_hold_threshold_and_a_release() {
    for ticks in [47, 48, 60] {
        let weapon = sabre();
        let mut a = Attacker::default();
        a.tick(
            &weapon,
            Controls {
                attack_pressed: true,
                ..hold()
            },
            true,
            1,
            false,
        );
        for _ in 1..ticks {
            a.tick(&weapon, hold(), true, 1, false);
        }
        assert_ne!(a.attack(), Some(AttackId::Charged));
        a.tick(&weapon, Controls::default(), true, 1, false);
        assert_eq!(a.attack(), (ticks >= 48).then_some(AttackId::Charged));
    }
    // The maul's opener still owes recovery when the charge reaches its threshold.
    let weapon = jeroboam();
    let mut a = Attacker::default();
    a.tick(
        &weapon,
        Controls {
            attack_pressed: true,
            ..hold()
        },
        true,
        1,
        false,
    );
    for _ in 1..weapon.charge_ticks {
        a.tick(&weapon, hold(), true, 1, false);
    }
    a.tick(&weapon, Controls::default(), true, 1, false);
    assert_eq!(a.attack(), Some(AttackId::Combo1));
    for _ in weapon.charge_ticks..weapon.combo[0].total_ticks() {
        tick(&mut a, &weapon);
    }
    assert_eq!(a.attack(), Some(AttackId::Charged));
}

#[test]
fn armour_takes_damage_and_poise_loss_without_interrupting_the_charge() {
    let weapon = sabre();
    let mut a = guest(WeightClass::Light);
    let mut b = Combatant::new(100.0, 1.0, WeightClass::Light);
    active(&mut a, &weapon, AttackId::Combo3);
    b.attacker.start(&weapon, AttackId::Charged, -1);
    contact(&mut a, &mut b, ImpactScale::NEUTRAL);
    assert_eq!(b.health, 76.0);
    assert_eq!(b.poise, 0.0);
    assert_eq!(b.reaction(), Reaction::Ready);
    assert_eq!(b.attacker.attack(), Some(AttackId::Charged));
    assert_eq!(b.attacker.elapsed(), Some(0));
    assert!(b.shivering());
}

#[test]
fn each_swing_hits_each_target_once_even_with_overlapping_boxes() {
    let mut weapon = sabre();
    weapon.combo[0]
        .hitboxes
        .push(weapon.combo[0].hitboxes[0].clone());
    let mut a = guest(WeightClass::Light);
    let mut b = guest(WeightClass::Heavy);
    active(&mut a, &weapon, AttackId::Combo1);
    assert_eq!(contact(&mut a, &mut b, ImpactScale::NEUTRAL).len(), 1);
    for _ in 0..50 {
        a.tick(&weapon, Controls::default(), true, 1);
        b.tick(&weapon, Controls::default(), true, -1);
        assert!(contact(&mut a, &mut b, ImpactScale::NEUTRAL).is_empty());
    }
    assert_eq!(b.health, 488.0);
    active(&mut a, &weapon, AttackId::Combo1);
    assert_eq!(contact(&mut a, &mut b, ImpactScale::NEUTRAL).len(), 1);
    assert_eq!(b.health, 476.0);
}

#[test]
fn light_hits_flinch_and_broken_poise_staggers() {
    let weapon = sabre();
    for (poise, expected) in [(100.0, Reaction::Flinch), (8.0, Reaction::Stagger)] {
        let mut a = guest(WeightClass::Light);
        let mut b = Combatant::new(100.0, poise, WeightClass::Medium);
        b.attacker.start(&weapon, AttackId::Combo1, -1);
        active(&mut a, &weapon, AttackId::Combo1);
        contact(&mut a, &mut b, ImpactScale::NEUTRAL);
        assert_eq!(b.reaction(), expected);
        assert!(b.helpless());
        assert_eq!(b.attacker.phase(), Phase::Idle);
        assert_eq!(b.poise, (poise - 8.0).max(0.0));
    }
}

#[test]
fn heavy_blows_launch_light_knock_down_medium_and_never_launch_heavy() {
    let weapon = sabre();
    for (weight, expected) in [
        (WeightClass::Light, Reaction::Launched),
        (WeightClass::Medium, Reaction::Knockdown),
        (WeightClass::Heavy, Reaction::Ready),
    ] {
        let mut a = guest(WeightClass::Light);
        let mut b = guest(weight);
        active(&mut a, &weapon, AttackId::Combo3);
        contact(&mut a, &mut b, ImpactScale::NEUTRAL);
        assert_eq!(b.reaction(), expected);
        if expected == Reaction::Launched {
            assert!(b.velocity().1 < 0.0);
        }
        if weight == WeightClass::Heavy {
            assert_eq!(b.velocity(), (420.0 * 0.18, 0.0));
        }
    }
    let mut a = guest(WeightClass::Light);
    let mut b = Combatant::new(100.0, 20.0, WeightClass::Heavy);
    active(&mut a, &weapon, AttackId::Combo3);
    contact(&mut a, &mut b, ImpactScale::NEUTRAL);
    assert_eq!(b.reaction(), Reaction::Stagger);
    assert_eq!(b.velocity(), (420.0 * 0.18, 0.0));
}

#[test]
fn launched_guests_splat_against_tiles_take_damage_and_bounce() {
    let weapon = sabre();
    let mut a = guest(WeightClass::Light);
    let mut b = guest(WeightClass::Light);
    active(&mut a, &weapon, AttackId::Combo3);
    contact(&mut a, &mut b, ImpactScale::NEUTRAL);
    let after_hit = b.health;
    thaw(&mut b, &weapon);
    let mut body = Body::new(280, 66, 12, 34);
    let room = room();
    let mut splats = Vec::new();
    for _ in 0..20 {
        b.tick(&weapon, Controls::default(), false, 1);
        splats.extend(b.move_body(2, &mut body, &room, 1450.0));
        if !splats.is_empty() {
            break;
        }
    }
    assert!(matches!(
        splats.as_slice(),
        [CombatEvent::WallSplat { target: 2, .. }]
    ));
    assert_eq!(body.x + body.w, 19 * 16);
    assert_eq!(b.health, after_hit - WALL_SPLAT_DAMAGE);
    assert_eq!(b.velocity().0, -420.0 * 0.45);
    assert!(!body.collides_at(&room, 0, 0));
    println!(
        "wall splat: threshold {WALL_SPLAT_SPEED} px/s, damage {WALL_SPLAT_DAMAGE}, bounce {} px/s",
        b.velocity().0
    );
    for _ in 0..300 {
        b.tick(&weapon, Controls::default(), body.on_ground(&room), 1);
        b.move_body(2, &mut body, &room, 1450.0);
    }
    assert!(body.on_ground(&room));
    assert_eq!(b.reaction(), Reaction::Ready);
}

#[test]
fn a_launched_body_knocks_another_guest_once_and_respects_heavy_mass() {
    let weapon = sabre();
    for (weight, expected) in [
        (WeightClass::Light, Reaction::Launched),
        (WeightClass::Medium, Reaction::Knockdown),
        (WeightClass::Heavy, Reaction::Ready),
    ] {
        let mut a = guest(WeightClass::Light);
        let mut projectile = guest(WeightClass::Light);
        let mut victim = guest(weight);
        active(&mut a, &weapon, AttackId::Combo3);
        contact(&mut a, &mut projectile, ImpactScale::NEUTRAL);
        thaw(&mut projectile, &weapon);
        let body = Body::new(100, 100, 12, 34);
        let mut views = [
            Fighter {
                id: 2,
                team: 1,
                combat: &mut projectile,
                body: &body,
                hurtboxes: &[],
            },
            Fighter {
                id: 3,
                team: 1,
                combat: &mut victim,
                body: &body,
                hurtboxes: &[],
            },
        ];
        assert!(matches!(
            resolve_body_collisions(&mut views, ImpactScale::NEUTRAL).as_slice(),
            [CombatEvent::BodyHit {
                projectile: 2,
                target: 3,
                ..
            }]
        ));
        assert_eq!(views[1].combat.reaction(), expected);
        thaw(views[0].combat, &weapon);
        thaw(views[1].combat, &weapon);
        assert!(resolve_body_collisions(&mut views, ImpactScale::NEUTRAL).is_empty());
        assert_eq!(views[1].combat.health, 494.0);
    }
}

#[test]
fn hit_stop_freezes_attack_movement_and_reactions_for_the_scaled_duration() {
    let weapon = sabre();
    let room = room();
    let tuning = MoveTuning::default();
    for mood in [
        ImpactScale::NEUTRAL,
        ImpactScale::HIGH,
        ImpactScale::COMEDOWN,
    ] {
        let mut a = guest(WeightClass::Light);
        let mut b = guest(WeightClass::Light);
        let mut ac = player(80.0, 100.0);
        let mut bc = player(100.0, 100.0);
        active(&mut a, &weapon, AttackId::Combo1);
        let events = contact_at(&mut a, &mut b, &ac.body, &bc.body, mood);
        let h = hit(&events);
        let ticks = seconds_to_ticks(weapon.combo[0].hit_stop * mood.hit_stop);
        assert_eq!(a.freeze_ticks(), ticks);
        assert_eq!(b.freeze_ticks(), ticks);
        assert_eq!(h.poise_damage, 8.0 * mood.poise_damage);
        assert_eq!(h.knockback.0, 154.0 * mood.knockback);
        assert_eq!(h.shake, mood.shake);
        let before = (
            ac.clone(),
            bc.clone(),
            a.attacker.elapsed(),
            b.reaction_ticks(),
        );
        for _ in 0..ticks {
            a.tick_controller(
                1,
                &mut ac,
                &room,
                &tuning,
                &weapon,
                Controls {
                    x: -1.0,
                    jump_pressed: true,
                    ..Controls::default()
                },
            );
            b.tick_controller(2, &mut bc, &room, &tuning, &weapon, Controls::default());
            assert_eq!(
                (
                    ac.clone(),
                    bc.clone(),
                    a.attacker.elapsed(),
                    b.reaction_ticks()
                ),
                before
            );
            assert!(!a.shivering());
            assert!(b.shivering());
        }
        a.tick_controller(1, &mut ac, &room, &tuning, &weapon, Controls::default());
        b.tick_controller(2, &mut bc, &room, &tuning, &weapon, Controls::default());
        assert_eq!(a.attacker.elapsed(), before.2.map(|n| n + 1));
        assert_ne!(bc.body, before.1.body);
        assert!(!b.shivering());
        println!(
            "hit-stop {:?}: {} s rounds up to {ticks} ticks",
            mood, h.hit_stop
        );
    }
}

#[test]
fn toast_parries_inside_its_window_and_staggers_even_an_armoured_attacker() {
    let weapon = sabre();
    let window = seconds_to_ticks(PARRY_SECONDS);
    assert_eq!(window, 15);
    for elapsed in [0, window - 1, window] {
        let mut a = guest(WeightClass::Heavy);
        let mut b = guest(WeightClass::Light);
        active(&mut a, &weapon, AttackId::Charged);
        b.tick(
            &weapon,
            Controls {
                parry_pressed: true,
                ..Controls::default()
            },
            true,
            -1,
        );
        for _ in 0..elapsed {
            b.tick(&weapon, Controls::default(), true, -1);
        }
        let events = contact(&mut a, &mut b, ImpactScale::NEUTRAL);
        if elapsed < window {
            assert!(matches!(
                events.as_slice(),
                [CombatEvent::Parry {
                    attacker: 1,
                    defender: 2,
                    ..
                }]
            ));
            assert_eq!(a.reaction(), Reaction::Stagger);
            assert_eq!(a.attacker.phase(), Phase::Idle);
            assert_eq!(b.health, 500.0);
            assert_eq!(b.poise, 100.0);
            assert_eq!(a.freeze_ticks(), seconds_to_ticks(0.18));
        } else {
            assert!(matches!(events.as_slice(), [CombatEvent::Hit(_)]));
            assert_eq!(b.health, 460.0);
        }
    }
    println!("Toast: {PARRY_SECONDS} s, {window} simulation ticks, 0.18 s hit-stop");
}

#[test]
fn controller_lock_enforces_commitment_step_and_lunge_only_cancel() {
    // The sabre's first swing is mobile; held swings are what this pins.
    let mut weapon = sabre();
    weapon.combo[0].mobile = false;
    let room = room();
    let tuning = MoveTuning::default();
    let mut a = guest(WeightClass::Light);
    let mut c = player(80.0, 192.0);
    let input = Controls {
        x: -1.0,
        jump_pressed: true,
        lunge_pressed: true,
        ..press()
    };
    let (events, _) = a.tick_controller(1, &mut c, &room, &tuning, &weapon, input);
    assert!(!events.jumped && !events.lunged && !events.swooned);
    assert_eq!(c.facing, 1);
    assert_eq!(c.vx, 45.0);
    assert!(c.grounded);
    let start_x = c.body.x;
    for _ in 0..weapon.combo[0].windup {
        let (events, _) = a.tick_controller(
            1,
            &mut c,
            &room,
            &tuning,
            &weapon,
            Controls {
                attack_pressed: false,
                ..input
            },
        );
        assert!(!events.jumped && !events.lunged);
    }
    assert!(c.body.x > start_x);
    let mut b = guest(WeightClass::Light);
    let target = Body::new(c.body.x + 20, c.body.y, 12, 34);
    contact_at(&mut a, &mut b, &c.body, &target, ImpactScale::NEUTRAL);
    while !a.can_lunge_cancel() {
        a.tick_controller(1, &mut c, &room, &tuning, &weapon, Controls::default());
    }
    // A jump and a turn cannot spend the cancel window.
    let (events, _) = a.tick_controller(
        1,
        &mut c,
        &room,
        &tuning,
        &weapon,
        Controls {
            jump_pressed: true,
            x: -1.0,
            ..Controls::default()
        },
    );
    assert!(!events.jumped && !events.swooned);
    assert_eq!(c.facing, 1);
    assert_eq!(a.attacker.phase(), Phase::Recovery);
    let (events, _) = a.tick_controller(
        1,
        &mut c,
        &room,
        &tuning,
        &weapon,
        Controls {
            lunge_pressed: true,
            ..Controls::default()
        },
    );
    assert!(events.lunged);
    assert_eq!(a.attacker.phase(), Phase::Idle);
    assert_eq!(c.vx, tuning.lunge_speed);
}

#[test]
fn a_mobile_swing_steers_jumps_and_cancels_in_recovery_hit_or_miss() {
    let weapon = sabre();
    assert!(weapon.combo[0].mobile && !weapon.combo[2].mobile);
    let room = room();
    let tuning = MoveTuning::default();
    let mut a = guest(WeightClass::Light);
    let mut c = player(80.0, 192.0);
    // The stick steers through the windup, but the swing keeps facing the way it started.
    a.tick_controller(1, &mut c, &room, &tuning, &weapon, press());
    for _ in 0..weapon.combo[0].windup {
        a.tick_controller(
            1,
            &mut c,
            &room,
            &tuning,
            &weapon,
            Controls {
                x: -1.0,
                ..Controls::default()
            },
        );
    }
    assert!(c.vx < 0.0);
    assert_eq!(c.facing, 1);
    // A miss: once in recovery, a jump ends the swing.
    while a.attacker.phase() != Phase::Recovery {
        a.tick_controller(1, &mut c, &room, &tuning, &weapon, Controls::default());
    }
    let (events, _) = a.tick_controller(
        1,
        &mut c,
        &room,
        &tuning,
        &weapon,
        Controls {
            jump_pressed: true,
            jump_held: true,
            ..Controls::default()
        },
    );
    assert!(events.jumped);
    assert_eq!(a.attacker.phase(), Phase::Idle);
}

#[test]
fn a_landed_mobile_swing_pushes_its_attacker_back() {
    let weapon = sabre();
    let mut a = guest(WeightClass::Light);
    let mut b = guest(WeightClass::Light);
    active(&mut a, &weapon, AttackId::Combo1);
    let mut c = player(80.0, 192.0);
    let target = Body::new(c.body.x + 20, c.body.y, 12, 34);
    contact_at(&mut a, &mut b, &c.body, &target, ImpactScale::NEUTRAL);
    thaw(&mut a, &weapon);
    let x = c.body.x;
    for _ in 0..12 {
        a.tick_controller(
            1,
            &mut c,
            &room(),
            &MoveTuning::default(),
            &weapon,
            Controls::default(),
        );
    }
    assert!(
        c.body.x < x,
        "recoil moves the attacker away from the target"
    );
}

#[test]
fn an_unavailable_lunge_does_not_cancel_a_connected_attack() {
    let weapon = sabre();
    let mut a = guest(WeightClass::Light);
    let mut b = guest(WeightClass::Light);
    active(&mut a, &weapon, AttackId::Combo1);
    contact(&mut a, &mut b, ImpactScale::NEUTRAL);
    thaw(&mut a, &weapon);
    while !a.can_lunge_cancel() {
        tick(&mut a.attacker, &weapon);
    }
    let mut c = Controller::standing_at(80.0, 192.0, 12, 34, Abilities::default());
    let (events, _) = a.tick_controller(
        1,
        &mut c,
        &room(),
        &MoveTuning::default(),
        &weapon,
        Controls {
            lunge_pressed: true,
            ..Controls::default()
        },
    );
    assert!(!events.lunged);
    assert_eq!(a.attacker.phase(), Phase::Recovery);
}

#[test]
fn directional_attacks_air_chain_and_pogo_have_distinct_behaviour() {
    let weapon = sabre();
    for (grounded, y, id) in [
        (true, 1.0, AttackId::Rising),
        (false, 0.0, AttackId::Air1),
        (false, -1.0, AttackId::Pogo),
    ] {
        let mut a = Attacker::default();
        a.tick(&weapon, Controls { y, ..press() }, grounded, -1, false);
        assert_eq!(a.attack(), Some(id));
        assert_eq!(a.facing(), Some(-1));
    }
    let mut a = guest(WeightClass::Light);
    let mut c = player(80.0, 100.0);
    let room = room();
    let tuning = MoveTuning::default();
    c.vy = 200.0;
    a.tick_controller(1, &mut c, &room, &tuning, &weapon, press());
    let y = c.body.y;
    for _ in 1..weapon.air[0].air_hang {
        a.tick_controller(1, &mut c, &room, &tuning, &weapon, Controls::default());
        assert_eq!(c.body.y, y);
        assert_eq!(c.vy, 0.0);
    }
    a.tick_controller(1, &mut c, &room, &tuning, &weapon, Controls::default());
    assert!(c.vy > 0.0);
    while a.attacker.elapsed().unwrap() < weapon.air[0].total_ticks() - 3 {
        tick(&mut a.attacker, &weapon);
    }
    a.attacker.tick(&weapon, press(), false, 1, false);
    tick(&mut a.attacker, &weapon);
    tick(&mut a.attacker, &weapon);
    assert_eq!(a.attacker.attack(), Some(AttackId::Air2));
    assert!(a.attacker.motion().air_hang);

    let mut a = guest(WeightClass::Light);
    let mut b = guest(WeightClass::Light);
    active(&mut a, &weapon, AttackId::Pogo);
    c = player(80.0, 100.0);
    let target = Body::new(74, 106, 12, 34);
    let events = contact_at(&mut a, &mut b, &c.body, &target, ImpactScale::NEUTRAL);
    assert!(events.contains(&CombatEvent::PogoBounce {
        attacker: 1,
        velocity: -POGO_SPEED
    }));
    assert_eq!(a.take_bounce(), None);
    for _ in 0..seconds_to_ticks(weapon.pogo.hit_stop) {
        a.tick_controller(1, &mut c, &room, &tuning, &weapon, Controls::default());
        assert_eq!(c.body.y + c.body.h, 100);
    }
    a.tick_controller(1, &mut c, &room, &tuning, &weapon, Controls::default());
    assert!(c.vy < -300.0);
    assert!(c.body.y + c.body.h < 100);
    assert_eq!(a.take_bounce(), None);
}

#[test]
fn hitboxes_mirror_and_the_slam_has_a_delayed_shockwave() {
    let weapon = sabre();
    let mut a = Attacker::default();
    a.start(&weapon, AttackId::Combo1, -1);
    for _ in 0..weapon.combo[0].windup {
        tick(&mut a, &weapon);
    }
    let boxes = a.active_hitboxes((80.0, 100.0));
    assert_eq!(
        boxes,
        [Rect {
            x: 34.0,
            y: 68.0,
            w: 42.0,
            h: 30.0
        }]
    );
    assert!(!boxes[0].overlaps(Rect {
        x: 76.0,
        y: 68.0,
        w: 12.0,
        h: 34.0
    }));
    let mut source = guest(WeightClass::Light);
    source.attacker = a;
    let mut target = guest(WeightClass::Light);
    let events = contact_at(
        &mut source,
        &mut target,
        &Body::new(74, 66, 12, 34),
        &Body::new(44, 66, 12, 34),
        ImpactScale::NEUTRAL,
    );
    assert_eq!(hit(&events).direction, -1);
    assert_eq!(hit(&events).knockback.0, -154.0);

    let weapon = jeroboam();
    let mut a = Attacker::default();
    a.start(&weapon, AttackId::Charged, 1);
    for _ in 0..weapon.charged.windup {
        tick(&mut a, &weapon);
    }
    assert_eq!(a.active_hitboxes((80.0, 100.0))[0].w, 42.0);
    for _ in 0..6 {
        tick(&mut a, &weapon);
    }
    assert_eq!(
        a.active_hitboxes((80.0, 100.0)),
        [Rect {
            x: 34.0,
            y: 88.0,
            w: 92.0,
            h: 16.0
        }]
    );
    for _ in 6..weapon.charged.active {
        tick(&mut a, &weapon);
    }
    assert!(a.active_hitboxes((80.0, 100.0)).is_empty());
}

#[test]
fn poise_regenerates_only_after_the_delay_and_reactions_end() {
    let weapon = sabre();
    let mut a = guest(WeightClass::Light);
    let mut b = guest(WeightClass::Medium);
    active(&mut a, &weapon, AttackId::Combo1);
    contact(&mut a, &mut b, ImpactScale::NEUTRAL);
    for _ in 0..seconds_to_ticks(weapon.combo[0].hit_stop) {
        assert!(!b.tick(&weapon, Controls::default(), true, -1));
        assert_eq!(b.reaction_ticks(), 18);
        assert_eq!(b.poise, 92.0);
    }
    for i in 1..=120 {
        b.tick(&weapon, Controls::default(), true, -1);
        assert_eq!(b.poise, 92.0);
        if i < 18 {
            assert_eq!(b.reaction(), Reaction::Flinch);
        } else {
            assert_eq!(b.reaction(), Reaction::Ready);
        }
    }
    b.tick(&weapon, Controls::default(), true, -1);
    assert_eq!(b.poise, 92.25);
    for _ in 0..100 {
        b.tick(&weapon, Controls::default(), true, -1);
    }
    assert_eq!(b.poise, b.max_poise);
}

#[test]
fn simultaneous_trades_are_independent_of_fighter_order() {
    let weapon = sabre();
    let mut states = Vec::new();
    for reverse in [false, true] {
        let mut a = guest(WeightClass::Light);
        let mut b = guest(WeightClass::Light);
        active(&mut a, &weapon, AttackId::Combo1);
        b.attacker.start(&weapon, AttackId::Combo1, -1);
        for _ in 0..weapon.combo[0].windup {
            tick(&mut b.attacker, &weapon);
        }
        let ab = Body::new(74, 66, 12, 34);
        let bb = Body::new(94, 66, 12, 34);
        let ah = [Rect::from_body(&ab)];
        let bh = [Rect::from_body(&bb)];
        let mut views = [
            Fighter {
                id: 1,
                team: 0,
                combat: &mut a,
                body: &ab,
                hurtboxes: &ah,
            },
            Fighter {
                id: 2,
                team: 1,
                combat: &mut b,
                body: &bb,
                hurtboxes: &bh,
            },
        ];
        if reverse {
            views.reverse();
        }
        let events = resolve(&mut views, ImpactScale::NEUTRAL);
        assert_eq!(events.len(), 2);
        assert_eq!((a.health, b.health), (488.0, 488.0));
        states.push((a, b, events));
    }
    assert_eq!(states[0], states[1]);
}

#[test]
fn one_swing_can_hit_multiple_enemies_but_not_allies_or_dead_targets() {
    let weapon = sabre();
    let mut a = guest(WeightClass::Light);
    let mut enemies = [
        guest(WeightClass::Light),
        guest(WeightClass::Light),
        guest(WeightClass::Light),
        guest(WeightClass::Light),
    ];
    enemies[3].health = 0.0;
    active(&mut a, &weapon, AttackId::Combo1);
    let ab = Body::new(74, 66, 12, 34);
    let bb = Body::new(94, 66, 12, 34);
    let hurtboxes = [Rect::from_body(&bb)];
    let mut views = vec![Fighter {
        id: 1,
        team: 0,
        combat: &mut a,
        body: &ab,
        hurtboxes: &[],
    }];
    for (i, enemy) in enemies.iter_mut().enumerate() {
        views.push(Fighter {
            id: i as u64 + 2,
            team: if i == 2 { 0 } else { 1 },
            combat: enemy,
            body: &bb,
            hurtboxes: &hurtboxes,
        });
    }
    assert_eq!(resolve(&mut views, ImpactScale::NEUTRAL).len(), 2);
    assert_eq!(enemies.map(|e| e.health), [488.0, 488.0, 500.0, 0.0]);
}

#[test]
fn lethal_damage_ends_an_armoured_attack() {
    let weapon = sabre();
    let mut a = guest(WeightClass::Light);
    let mut b = Combatant::new(1.0, 100.0, WeightClass::Heavy);
    active(&mut a, &weapon, AttackId::Combo1);
    b.attacker.start(&weapon, AttackId::Charged, -1);
    contact(&mut a, &mut b, ImpactScale::NEUTRAL);
    assert_eq!(b.health, 0.0);
    assert_eq!(b.reaction(), Reaction::Dead);
    assert_eq!(b.attacker.phase(), Phase::Idle);
}

#[test]
fn active_swings_keep_their_frame_data_when_a_weapon_is_reloaded() {
    let mut weapon = sabre();
    let mut a = Attacker::default();
    a.start(&weapon, AttackId::Combo1, 1);
    weapon.combo[0].windup = 50;
    for _ in 0..10 {
        tick(&mut a, &weapon);
    }
    assert_eq!(a.phase(), Phase::Active);
    assert_eq!(a.data().unwrap().windup, 5);
}

#[test]
fn a_parry_stops_the_swing_before_it_can_hit_a_second_target() {
    let weapon = sabre();
    let mut a = guest(WeightClass::Light);
    let mut bystander = guest(WeightClass::Light);
    let mut defender = guest(WeightClass::Light);
    active(&mut a, &weapon, AttackId::Combo3);
    defender.tick(
        &weapon,
        Controls {
            parry_pressed: true,
            ..Controls::default()
        },
        true,
        -1,
    );
    let ab = Body::new(74, 66, 12, 34);
    let bb = Body::new(94, 66, 12, 34);
    let boxes = [Rect::from_body(&bb)];
    let events = resolve(
        &mut [
            Fighter {
                id: 1,
                team: 0,
                combat: &mut a,
                body: &ab,
                hurtboxes: &[],
            },
            Fighter {
                id: 2,
                team: 1,
                combat: &mut bystander,
                body: &bb,
                hurtboxes: &boxes,
            },
            Fighter {
                id: 3,
                team: 1,
                combat: &mut defender,
                body: &bb,
                hurtboxes: &boxes,
            },
        ],
        ImpactScale::NEUTRAL,
    );
    assert!(matches!(
        events.as_slice(),
        [CombatEvent::Parry {
            attacker: 1,
            defender: 3,
            ..
        }]
    ));
    assert_eq!(bystander.health, 500.0);
    assert_eq!(defender.health, 500.0);
}

#[test]
fn collateral_impacts_apply_mood_and_respect_armour() {
    let weapon = sabre();
    for mood in [ImpactScale::HIGH, ImpactScale::COMEDOWN] {
        let mut a = guest(WeightClass::Light);
        let mut projectile = guest(WeightClass::Light);
        let mut victim = guest(WeightClass::Light);
        active(&mut a, &weapon, AttackId::Combo3);
        contact(&mut a, &mut projectile, ImpactScale::NEUTRAL);
        thaw(&mut projectile, &weapon);
        victim.attacker.start(&weapon, AttackId::Charged, 1);
        let body = Body::new(100, 100, 12, 34);
        let events = resolve_body_collisions(
            &mut [
                Fighter {
                    id: 2,
                    team: 1,
                    combat: &mut projectile,
                    body: &body,
                    hurtboxes: &[],
                },
                Fighter {
                    id: 3,
                    team: 1,
                    combat: &mut victim,
                    body: &body,
                    hurtboxes: &[],
                },
            ],
            mood,
        );
        let [CombatEvent::BodyHit { velocity, .. }] = events.as_slice() else {
            panic!("body hit");
        };
        assert_eq!(velocity.0, 420.0 * 0.6 * mood.knockback);
        assert_eq!(victim.health, 494.0);
        assert_eq!(victim.poise, 100.0 - 20.0 * mood.poise_damage);
        assert_eq!(
            victim.freeze_ticks(),
            seconds_to_ticks(0.04 * mood.hit_stop)
        );
        assert_eq!(victim.reaction(), Reaction::Ready);
        assert_eq!(victim.attacker.attack(), Some(AttackId::Charged));
    }
}

#[test]
fn ordinary_movement_ignores_combat_controls_and_interrupts_clear_coyote_time() {
    let room = room();
    let tuning = MoveTuning::default();
    let mut plain = player(80.0, 192.0);
    let mut combat = plain.clone();
    for _ in 0..60 {
        let input = Controls {
            x: 1.0,
            ..Controls::default()
        };
        let expected = plain.tick(&room, &tuning, input, DT);
        let actual = combat.tick(
            &room,
            &tuning,
            Controls {
                attack_pressed: true,
                attack_held: true,
                parry_pressed: true,
                ..input
            },
            DT,
        );
        assert_eq!(actual, expected);
        assert_eq!(combat, plain);
    }
    let mut c = Controller::standing_at(80.0, 192.0, 12, 34, Abilities::default());
    c.tick(&room, &tuning, Controls::default(), DT);
    c.stop_actions();
    c.body.y -= 64;
    let events = c.tick(
        &room,
        &tuning,
        Controls {
            jump_pressed: true,
            ..Controls::default()
        },
        DT,
    );
    assert!(!events.jumped && !events.swooned);
}

#[test]
fn burning_hurts_without_interrupting() {
    let w = sabre();
    let mut c = guest(WeightClass::Light);
    let health = c.health;
    let poise = c.poise;
    c.tick(&w, press(), true, 1);
    assert!(c.attacker.attack().is_some());
    c.burn(5.0);
    assert_eq!(c.health, health - 5.0);
    assert_eq!(c.poise, poise, "burning does not touch poise");
    assert!(
        c.attacker.attack().is_some(),
        "burning does not interrupt a swing"
    );
    assert_eq!(c.reaction(), Reaction::Ready);
    c.burn(1000.0);
    assert_eq!(c.reaction(), Reaction::Dead);
}

fn slam() -> Hit {
    Hit {
        attacker: 1,
        target: 2,
        attack: AttackId::Charged,
        direction: -1,
        damage: 30.0,
        poise_damage: 80.0,
        hit_stop: 0.16,
        knockback: (-340.0, -340.0),
        weight: Weight::Charged,
        launches: true,
        shake: 1.0,
    }
}

#[test]
fn outgoing_scales_and_defence_apply_once_to_swings_and_slams() {
    for external in [false, true] {
        let mut source = guest(WeightClass::Light);
        let mut target = guest(WeightClass::Light);
        assert_eq!(
            (source.damage_scale, source.poise_scale, target.defence),
            (1.0, 1.0, 1.0)
        );
        source.damage_scale = 2.0;
        source.poise_scale = 0.5;
        target.defence = 0.25;
        let weapon = sabre();
        let events = if external {
            vec![slam_hit(&mut source, &mut target, slam(), ImpactScale::HIGH).unwrap()]
        } else {
            active(&mut source, &weapon, AttackId::Combo1);
            contact(&mut source, &mut target, ImpactScale::HIGH)
        };
        let h = hit(&events);
        let damage = if external { 30.0 } else { 12.0 };
        let poise = if external { 80.0 } else { 8.0 };
        assert_eq!(h.damage, damage * 2.0 * 0.25);
        assert_eq!(target.health, 500.0 - h.damage);
        assert_eq!(h.poise_damage, poise * 0.5 * 1.2);
        assert_eq!(target.poise, 100.0 - h.poise_damage);
        assert!(source.is_frozen() && target.shivering());
    }
}

#[test]
fn slam_launches_outward_and_rejects_dead_targets() {
    let mut source = guest(WeightClass::Light);
    let mut target = guest(WeightClass::Light);
    slam_hit(&mut source, &mut target, slam(), ImpactScale::NEUTRAL).unwrap();
    assert_eq!(target.reaction(), Reaction::Launched);
    assert_eq!(target.velocity(), (-340.0, -340.0));
    assert_eq!(target.freeze_ticks(), 20);
    target.health = 0.0;
    assert!(slam_hit(&mut source, &mut target, slam(), ImpactScale::NEUTRAL).is_none());
}

#[test]
fn flinch_interrupts_unarmoured_windups_but_heavy_guests_keep_swinging() {
    let weapon = sabre();
    for weight in [WeightClass::Light, WeightClass::Medium, WeightClass::Heavy] {
        let mut source = guest(WeightClass::Light);
        let mut target = guest(weight);
        target.attacker.start(&weapon, AttackId::Combo1, -1);
        active(&mut source, &weapon, AttackId::Combo1);
        contact(&mut source, &mut target, ImpactScale::NEUTRAL);
        assert!(target.velocity().0 > 0.0);
        assert!(target.shivering());
        if weight == WeightClass::Heavy {
            assert_eq!(target.reaction(), Reaction::Ready);
            assert_eq!(target.attacker.phase(), Phase::Windup);
        } else {
            assert_eq!(target.reaction(), Reaction::Flinch);
            assert_eq!(target.reaction_ticks(), 18);
            assert_eq!(target.attacker.phase(), Phase::Idle);
        }
    }
}

#[test]
fn a_launch_bounces_once_then_opens_the_broken_poise_window() {
    let weapon = sabre();
    let mut source = guest(WeightClass::Light);
    let mut target = Combatant::new(500.0, 50.0, WeightClass::Light);
    slam_hit(&mut source, &mut target, slam(), ImpactScale::NEUTRAL).unwrap();
    let room = room();
    let mut body = Body::new(150, 158, 12, 34);
    let mut bounced = false;
    let mut falling = false;
    for _ in 0..240 {
        if target.tick(&weapon, Controls::default(), body.on_ground(&room), 1) {
            target.move_body(2, &mut body, &room, 1450.0);
            if falling && target.velocity().1 < 0.0 {
                bounced = true;
            }
            falling = target.velocity().1 > 0.0;
            if target.reaction() == Reaction::Stagger {
                assert!(bounced);
                assert_eq!(target.reaction_ticks(), 108);
                return;
            }
        }
    }
    panic!("broken body never became finishable");
}

#[test]
fn lethal_hits_preserve_the_impulse_for_the_falling_body() {
    let mut source = guest(WeightClass::Light);
    let mut target = Combatant::new(10.0, 50.0, WeightClass::Light);
    slam_hit(&mut source, &mut target, slam(), ImpactScale::NEUTRAL).unwrap();
    assert_eq!(target.reaction(), Reaction::Dead);
    assert_eq!(target.velocity(), (-340.0, -340.0));
    thaw(&mut target, &sabre());
    assert_eq!(target.velocity(), (-340.0, -340.0));
}

#[test]
fn slam_respects_charge_armour_but_still_pushes_and_shivers() {
    let mut source = guest(WeightClass::Light);
    let mut target = guest(WeightClass::Medium);
    let weapon = sabre();
    target.attacker.start(&weapon, AttackId::Charged, 1);
    slam_hit(&mut source, &mut target, slam(), ImpactScale::NEUTRAL).unwrap();
    assert_eq!(target.reaction(), Reaction::Ready);
    assert_eq!(target.attacker.attack(), Some(AttackId::Charged));
    assert!(target.shivering());
    assert!(target.velocity().0 < 0.0);
}

#[test]
fn light_flinch_slides_a_visible_distance_on_the_floor() {
    let mut source = guest(WeightClass::Light);
    let mut target = guest(WeightClass::Medium);
    let weapon = sabre();
    active(&mut source, &weapon, AttackId::Combo1);
    contact(&mut source, &mut target, ImpactScale::NEUTRAL);
    let mut controller = player(100.0, 192.0);
    let start = controller.body.x;
    for _ in 0..24 {
        target.tick_controller(
            2,
            &mut controller,
            &room(),
            &MoveTuning::default(),
            &weapon,
            Controls::default(),
        );
    }
    assert!(controller.body.x - start >= 15);
    assert!(controller.grounded);
}
