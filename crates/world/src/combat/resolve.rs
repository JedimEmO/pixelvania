use std::collections::BTreeSet;

use crate::Body;

use super::{AttackId, Combatant, Phase, Reaction, Rect, Weight};

/// Impact changes without changes to frame data or input windows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImpactScale {
    pub hit_stop: f32,
    pub knockback: f32,
    pub poise_damage: f32,
    pub shake: f32,
}

impl ImpactScale {
    pub const NEUTRAL: Self = Self {
        hit_stop: 1.0,
        knockback: 1.0,
        poise_damage: 1.0,
        shake: 1.0,
    };
    pub const HIGH: Self = Self {
        hit_stop: 1.2,
        knockback: 1.15,
        poise_damage: 1.2,
        shake: 1.25,
    };
    pub const COMEDOWN: Self = Self {
        hit_stop: 0.7,
        knockback: 0.75,
        poise_damage: 0.75,
        shake: 0.6,
    };
}

impl Default for ImpactScale {
    fn default() -> Self {
        Self::NEUTRAL
    }
}

/// A resolved hit. Knockback is already mirrored and scaled; hit-stop is in seconds.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub attacker: u64,
    pub target: u64,
    pub attack: AttackId,
    pub direction: i32,
    pub damage: f32,
    pub poise_damage: f32,
    pub hit_stop: f32,
    pub knockback: (f32, f32),
    pub weight: Weight,
    pub launches: bool,
    pub shake: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CombatEvent {
    Hit(Hit),
    Parry {
        attacker: u64,
        defender: u64,
        hit_stop: f32,
    },
    PogoBounce {
        attacker: u64,
        velocity: f32,
    },
    WallSplat {
        target: u64,
        damage: f32,
        incoming_speed: f32,
        bounce_velocity: f32,
    },
    BodyHit {
        projectile: u64,
        target: u64,
        velocity: (f32, f32),
    },
}

/// A temporary view for contact resolution. Hurtboxes use absolute world coordinates.
/// Equal teams cannot hit each other with weapons. Thrown bodies can hit either team.
pub struct Fighter<'a> {
    pub id: u64,
    pub team: u32,
    pub combat: &'a mut Combatant,
    pub body: &'a Body,
    pub hurtboxes: &'a [Rect],
}

struct Contact {
    source: usize,
    target: usize,
    hit: Hit,
    parried: bool,
    armoured: bool,
}

/// Resolves a snapshot of all active swings. Trades land together, independent of slice order.
/// Parries stop the entire incoming swing before its other contacts are applied.
/// Call once after movement; repeated calls cannot hit an already-hit target again.
pub fn resolve(fighters: &mut [Fighter<'_>], mood: ImpactScale) -> Vec<CombatEvent> {
    check_ids(fighters);
    check_mood(mood);
    let mut contacts = Vec::new();
    for (source, attacker) in fighters.iter().enumerate() {
        let state = &attacker.combat.attacker;
        if attacker.combat.is_frozen()
            || attacker.combat.health <= 0.0
            || state.phase() != Phase::Active
        {
            continue;
        }
        let body = attacker.body;
        let boxes = state.active_hitboxes((
            body.x as f32 + body.w as f32 / 2.0,
            (body.y + body.h) as f32,
        ));
        for (target, victim) in fighters.iter().enumerate() {
            if source == target
                || attacker.team == victim.team
                || victim.combat.health <= 0.0
                || state.has_hit(victim.id)
                || !boxes
                    .iter()
                    .any(|h| victim.hurtboxes.iter().any(|v| h.overlaps(*v)))
            {
                continue;
            }
            let data = state.data().expect("active swing");
            let direction = state.facing().expect("active swing");
            contacts.push(Contact {
                source,
                target,
                parried: victim.combat.parrying(),
                armoured: victim.combat.attacker.armoured(),
                hit: Hit {
                    attacker: attacker.id,
                    target: victim.id,
                    attack: state.attack().expect("active swing"),
                    direction,
                    damage: data.damage,
                    poise_damage: data.poise_damage * mood.poise_damage,
                    hit_stop: data.hit_stop * mood.hit_stop,
                    knockback: (
                        data.knockback.0 * direction as f32 * mood.knockback,
                        data.knockback.1 * mood.knockback,
                    ),
                    weight: data.weight,
                    launches: data.launches,
                    shake: mood.shake,
                },
            });
        }
    }
    contacts.sort_by_key(|c| (c.hit.attacker, c.hit.target));
    let mut events = Vec::new();
    let mut parried = BTreeSet::new();
    for c in contacts.iter().filter(|c| c.parried) {
        if !parried.insert(c.source) {
            continue;
        }
        let hit_stop = 0.18 * mood.hit_stop;
        fighters[c.source].combat.stagger();
        fighters[c.source].combat.freeze(hit_stop, true);
        fighters[c.target].combat.freeze(hit_stop, false);
        events.push(CombatEvent::Parry {
            attacker: c.hit.attacker,
            defender: c.hit.target,
            hit_stop,
        });
    }
    // Mark contacts before any damage interrupts a simultaneous attack.
    for c in contacts.iter().filter(|c| !parried.contains(&c.source)) {
        fighters[c.source]
            .combat
            .attacker
            .contact(c.hit.target, true);
    }
    let mut bounced = BTreeSet::new();
    for c in contacts
        .into_iter()
        .filter(|c| !parried.contains(&c.source))
    {
        fighters[c.source].combat.freeze(c.hit.hit_stop, false);
        fighters[c.target].combat.freeze(c.hit.hit_stop, true);
        fighters[c.target].combat.receive(&c.hit, c.armoured);
        if c.hit.attack == AttackId::Pogo
            && bounced.insert(c.source)
            && !fighters[c.source].combat.helpless()
        {
            fighters[c.source].combat.pogo();
            events.push(CombatEvent::PogoBounce {
                attacker: c.hit.attacker,
                velocity: -super::POGO_SPEED,
            });
        }
        events.push(CombatEvent::Hit(c.hit));
    }
    events
}

/// Transfers a launched body's momentum once per target in that flight.
/// Uses overlap after movement. Weapon teams do not stop a body from bowling into its allies.
pub fn resolve_body_collisions(
    fighters: &mut [Fighter<'_>],
    mood: ImpactScale,
) -> Vec<CombatEvent> {
    check_ids(fighters);
    check_mood(mood);
    let mut contacts = Vec::new();
    for (source, projectile) in fighters.iter().enumerate() {
        if projectile.combat.reaction() != Reaction::Launched || projectile.combat.is_frozen() {
            continue;
        }
        for (target, victim) in fighters.iter().enumerate() {
            if source == target
                || victim.combat.health <= 0.0
                || projectile.combat.body_contacts.contains(&victim.id)
                || !Rect::from_body(projectile.body).overlaps(Rect::from_body(victim.body))
            {
                continue;
            }
            contacts.push((
                source,
                target,
                projectile.combat.velocity(),
                victim.combat.attacker.armoured(),
            ));
        }
    }
    contacts.sort_by_key(|&(s, t, _, _)| (fighters[s].id, fighters[t].id));
    let mut events = Vec::new();
    // Record all contacts first, so two launched bodies cannot repeatedly knock each other.
    for &(source, target, _, _) in &contacts {
        let target_id = fighters[target].id;
        fighters[source].combat.body_contacts.insert(target_id);
    }
    for (source, target, velocity, armoured) in contacts {
        let hit = Hit {
            attacker: fighters[source].id,
            target: fighters[target].id,
            attack: AttackId::Rising,
            direction: if velocity.0 < 0.0 { -1 } else { 1 },
            damage: 6.0,
            poise_damage: 20.0 * mood.poise_damage,
            hit_stop: 0.04 * mood.hit_stop,
            knockback: (
                velocity.0 * 0.6 * mood.knockback,
                velocity.1.min(-160.0) * 0.6 * mood.knockback,
            ),
            weight: Weight::Heavy,
            launches: true,
            shake: mood.shake,
        };
        fighters[target].combat.receive(&hit, armoured);
        fighters[source].combat.freeze(hit.hit_stop, false);
        fighters[target].combat.freeze(hit.hit_stop, true);
        events.push(CombatEvent::BodyHit {
            projectile: hit.attacker,
            target: hit.target,
            velocity: hit.knockback,
        });
    }
    events
}

fn check_ids(fighters: &[Fighter<'_>]) {
    let ids: BTreeSet<_> = fighters.iter().map(|f| f.id).collect();
    assert_eq!(ids.len(), fighters.len(), "fighter IDs must be unique");
}

fn check_mood(mood: ImpactScale) {
    assert!(
        [mood.hit_stop, mood.knockback, mood.poise_damage, mood.shake]
            .iter()
            .all(|n| n.is_finite() && *n >= 0.0)
    );
}
