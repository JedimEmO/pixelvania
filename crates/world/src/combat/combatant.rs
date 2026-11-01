use std::collections::BTreeSet;

use crate::{Body, Controller, Controls, Events, MoveTuning, MovementLock, Room};

use super::{
    Attacker, CombatEvent, DT, Hit, PARRY_SECONDS, Phase, Reaction::*, WALL_SPLAT_DAMAGE,
    WALL_SPLAT_SPEED, Weapon, Weight, seconds_to_ticks,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeightClass {
    Light,
    Medium,
    Heavy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reaction {
    Ready,
    Flinch,
    Stagger,
    Knockdown,
    Launched,
    Dead,
}

/// Health, poise and impact timers. The body stays with its movement controller.
#[derive(Debug, Clone, PartialEq)]
pub struct Combatant {
    pub attacker: Attacker,
    pub health: f32,
    /// Outgoing health damage.
    pub damage_scale: f32,
    /// Outgoing poise damage.
    pub poise_scale: f32,
    /// Incoming health damage; lower values resist more.
    pub defence: f32,
    pub poise: f32,
    pub max_poise: f32,
    pub weight: WeightClass,
    pub poise_regen_per_second: f32,
    pub poise_regen_delay: u32,
    /// Passing through bodies this tick, as in a Lunge: a hit that is a body does not land.
    pub passing: bool,
    reaction: Reaction,
    reaction_ticks: u32,
    regen_wait: u32,
    freeze_ticks: u32,
    frozen_this_tick: bool,
    victim_stop: bool,
    parry_ticks: u32,
    velocity: (f32, f32),
    bounce: Option<f32>,
    floor_bounced: bool,
    pub(crate) body_contacts: BTreeSet<u64>,
}

impl Combatant {
    pub fn new(health: f32, poise: f32, weight: WeightClass) -> Self {
        assert!(health.is_finite() && health > 0.0 && poise.is_finite() && poise > 0.0);
        Self {
            attacker: Attacker::default(),
            health,
            damage_scale: 1.0,
            poise_scale: 1.0,
            defence: 1.0,
            poise,
            max_poise: poise,
            weight,
            poise_regen_per_second: 30.0,
            poise_regen_delay: 120,
            passing: false,
            reaction: Ready,
            reaction_ticks: 0,
            regen_wait: 0,
            freeze_ticks: 0,
            frozen_this_tick: false,
            victim_stop: false,
            parry_ticks: 0,
            velocity: (0.0, 0.0),
            bounce: None,
            floor_bounced: false,
            body_contacts: BTreeSet::new(),
        }
    }

    pub fn reaction(&self) -> Reaction {
        self.reaction
    }
    pub fn reaction_ticks(&self) -> u32 {
        self.reaction_ticks
    }
    pub fn velocity(&self) -> (f32, f32) {
        self.velocity
    }
    pub fn helpless(&self) -> bool {
        self.reaction != Ready
    }
    pub fn is_frozen(&self) -> bool {
        self.freeze_ticks > 0 || self.frozen_this_tick
    }
    pub fn freeze_ticks(&self) -> u32 {
        self.freeze_ticks
    }
    pub fn shivering(&self) -> bool {
        self.is_frozen() && self.victim_stop
    }
    pub fn parrying(&self) -> bool {
        self.parry_ticks > 0 && !self.helpless()
    }
    pub fn can_lunge_cancel(&self) -> bool {
        !self.is_frozen() && self.attacker.can_lunge_cancel()
    }

    /// Samples controls and advances timers. False means movement must skip this entire tick.
    /// Use this once per tick, or use [`Self::tick_controller`] to include player movement.
    pub fn tick(&mut self, weapon: &Weapon, input: Controls, grounded: bool, facing: i32) -> bool {
        self.frozen_this_tick = self.freeze_ticks > 0;
        if self.frozen_this_tick {
            self.freeze_ticks -= 1;
            if input.parry_pressed && !self.helpless() {
                self.parry_ticks = seconds_to_ticks(PARRY_SECONDS);
            }
            self.attacker.tick(weapon, input, grounded, facing, true);
            return false;
        }
        self.victim_stop = false;
        if self.health <= 0.0 {
            self.die();
            return false;
        }
        self.parry_ticks = self.parry_ticks.saturating_sub(1);
        if self.reaction_ticks > 0 {
            self.reaction_ticks -= 1;
            if self.reaction_ticks == 0 {
                self.reaction = if self.poise == 0.0 && self.reaction != Stagger {
                    Stagger
                } else {
                    Ready
                };
                if self.reaction == Stagger {
                    self.reaction_ticks = 108;
                } else if self.poise == 0.0 {
                    self.poise = self.max_poise;
                }
                self.velocity = (0.0, 0.0);
                self.body_contacts.clear();
            }
        }
        if self.regen_wait > 0 {
            self.regen_wait -= 1;
        } else if self.poise > 0.0 || self.reaction == Ready {
            self.poise = (self.poise + self.poise_regen_per_second * DT).min(self.max_poise);
        }
        if self.helpless() {
            self.attacker.interrupt();
        } else {
            if input.parry_pressed {
                self.parry_ticks = seconds_to_ticks(PARRY_SECONDS);
            }
            self.attacker.tick(weapon, input, grounded, facing, false);
        }
        true
    }

    /// Drives one player's combat and movement tick, including freezes and forced reactions.
    /// Resolve contacts only after every fighter has moved. Returned events include wall splats.
    pub fn tick_controller(
        &mut self,
        id: u64,
        controller: &mut Controller,
        room: &Room,
        tuning: &MoveTuning,
        weapon: &Weapon,
        input: Controls,
    ) -> (Events, Vec<CombatEvent>) {
        let mut events = Vec::new();
        if !self.tick(
            weapon,
            input,
            controller.body.on_ground(room),
            controller.facing,
        ) {
            return (Events::default(), events);
        }
        if self.helpless() {
            controller.stop_actions();
            events = self.move_body(id, &mut controller.body, room, tuning.gravity);
            (controller.vx, controller.vy) = self.velocity;
            controller.grounded = controller.body.on_ground(room);
            return (Events::default(), events);
        }
        if let Some(vy) = self.take_bounce() {
            controller.vy = vy;
        }
        let motion = self.attacker.motion();
        let swing_facing = self.attacker.facing();
        if let Some(facing) = swing_facing {
            controller.facing = facing;
        }
        let mobile = self.attacker.mobile();
        let lock = (motion.locked && !mobile).then_some(MovementLock {
            vx: motion.step_velocity,
            air_hang: motion.air_hang,
            allow_lunge: self.can_lunge_cancel(),
        });
        if self.velocity.0.abs() > 0.0 {
            controller.body.move_x(room, self.velocity.0 * DT);
            self.velocity.0 *= 0.86;
            if self.velocity.0.abs() < 1.0 {
                self.velocity.0 = 0.0;
            }
        }
        let moved = controller.tick_with_lock(room, tuning, input, DT, lock);
        if mobile {
            // The stick may have turned the body; the swing keeps the way it was facing.
            if let Some(facing) = swing_facing {
                controller.facing = facing;
            }
            let jumped = moved.jumped || moved.swooned || moved.wall_jumped.is_some();
            if moved.lunged || (jumped && self.attacker.phase() == Phase::Recovery) {
                self.attacker.interrupt();
            }
        } else if moved.lunged {
            self.attacker.lunge_happened();
        }
        (moved, events)
    }

    /// Takes damage that hurts but does not stagger or interrupt: fire on the floor, the room.
    /// Poise is untouched, and a body already down stays down.
    pub fn burn(&mut self, amount: f32) {
        if self.reaction == Dead || !(amount.is_finite() && amount > 0.0) {
            return;
        }
        self.health = (self.health - amount).max(0.0);
        if self.health == 0.0 {
            self.die();
        }
    }

    /// Takes the pogo impulse after hit-stop ends. Apply it before normal vertical movement.
    pub fn take_bounce(&mut self) -> Option<f32> {
        if self.is_frozen() {
            None
        } else {
            self.bounce.take()
        }
    }

    /// Moves a helpless body with tile collision. Normal movement belongs to its controller.
    pub fn move_body(
        &mut self,
        id: u64,
        body: &mut Body,
        room: &Room,
        gravity: f32,
    ) -> Vec<CombatEvent> {
        let mut events = Vec::new();
        if self.is_frozen() || matches!(self.reaction, Ready | Dead) {
            return events;
        }
        let launched = self.reaction == Launched;
        self.velocity.1 += gravity * DT;
        let vx = self.velocity.0;
        if body.move_x(room, vx * DT) {
            if launched && vx.abs() >= WALL_SPLAT_SPEED {
                self.health = (self.health - WALL_SPLAT_DAMAGE).max(0.0);
                self.velocity.0 = -vx * 0.45;
                events.push(CombatEvent::WallSplat {
                    target: id,
                    damage: WALL_SPLAT_DAMAGE,
                    incoming_speed: vx,
                    bounce_velocity: self.velocity.0,
                });
                if self.health == 0.0 {
                    self.die();
                }
            } else {
                self.velocity.0 = 0.0;
            }
        }
        if self.reaction == Dead {
            return events;
        }
        if body.move_y(room, self.velocity.1 * DT) {
            if self.velocity.1 > 0.0 && launched {
                if !self.floor_bounced && self.velocity.1 > 100.0 {
                    self.floor_bounced = true;
                    self.velocity.1 *= -0.35;
                    return events;
                }
                self.reaction = Knockdown;
                self.reaction_ticks = 48;
                self.body_contacts.clear();
            }
            self.velocity.1 = 0.0;
        }
        if !launched {
            self.velocity.0 =
                self.velocity.0.signum() * (self.velocity.0.abs() - 500.0 * DT).max(0.0);
        }
        events
    }

    pub(crate) fn freeze(&mut self, seconds: f32, victim: bool) {
        self.freeze_ticks = self.freeze_ticks.max(seconds_to_ticks(seconds));
        self.victim_stop |= victim;
    }

    /// Pushes a body that is not helpless along x, as a swing's recoil does; it eases off over a few ticks.
    pub(crate) fn recoil(&mut self, vx: f32) {
        if !self.helpless() {
            self.velocity.0 = vx;
        }
    }

    pub(crate) fn pogo(&mut self) {
        self.bounce = Some(-super::POGO_SPEED);
    }

    /// Interrupts an attack and opens a 108-tick stagger window.
    pub fn stagger(&mut self) {
        if self.health <= 0.0 {
            return;
        }
        self.reaction = Stagger;
        self.reaction_ticks = 108;
        self.velocity = (0.0, 0.0);
        self.parry_ticks = 0;
        self.bounce = None;
        self.attacker.interrupt();
    }

    pub(crate) fn receive(&mut self, hit: &Hit, armoured: bool) {
        self.health = (self.health - hit.damage).max(0.0);
        self.poise = (self.poise - hit.poise_damage).max(0.0);
        self.regen_wait = self.poise_regen_delay;
        let push = if self.weight == WeightClass::Heavy {
            0.18
        } else {
            1.0
        };
        self.velocity = (hit.knockback.0 * push, hit.knockback.1 * push);
        self.floor_bounced = false;
        if self.health == 0.0 {
            self.die();
            return;
        }
        if armoured {
            self.velocity.1 = 0.0;
            return;
        }
        let heavy = hit.weight != Weight::Light || hit.launches;
        self.reaction = match self.weight {
            WeightClass::Light if heavy => Launched,
            WeightClass::Medium if hit.weight != Weight::Light => Knockdown,
            _ if self.poise == 0.0 => Stagger,
            WeightClass::Heavy => {
                self.velocity.1 = 0.0;
                return;
            }
            _ => Flinch,
        };
        self.reaction_ticks = match self.reaction {
            Flinch => 18,
            Stagger => 108,
            Knockdown => 48,
            _ => 0,
        };
        self.velocity = if self.weight == WeightClass::Heavy {
            (hit.knockback.0 * 0.18, 0.0)
        } else if self.reaction == Launched {
            (hit.knockback.0, hit.knockback.1.min(-120.0))
        } else if self.reaction == Knockdown && hit.launches {
            (hit.knockback.0, hit.knockback.1.min(0.0) * 0.35)
        } else {
            (hit.knockback.0, 0.0)
        };
        self.body_contacts.clear();
        self.body_contacts.insert(hit.attacker);
        self.parry_ticks = 0;
        self.bounce = None;
        self.attacker.interrupt();
    }

    fn die(&mut self) {
        self.reaction = Dead;
        self.reaction_ticks = 0;
        self.parry_ticks = 0;
        self.bounce = None;
        self.attacker.interrupt();
    }
}
