use std::collections::BTreeSet;

use crate::Controls;

use super::{AttackData, AttackId, BUFFER_TICKS, Rect, Weapon};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Windup,
    Active,
    Recovery,
}

/// What a swing asks of movement. Facing stays fixed until the swing ends.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AttackMotion {
    pub locked: bool,
    pub step_velocity: f32,
    pub air_hang: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct Swing {
    id: AttackId,
    data: AttackData,
    tick: u32,
    facing: i32,
    connected: bool,
    targets: BTreeSet<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Buffered {
    next: AttackId,
    remaining: u32,
}

/// A committed swing and its buffered input. No movement or rendering lives here.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Attacker {
    swing: Option<Swing>,
    buffer: Option<Buffered>,
    hold_ticks: u32,
    charge_armed: bool,
    charged_release: bool,
}

impl Attacker {
    pub fn attack(&self) -> Option<AttackId> {
        self.swing.as_ref().map(|s| s.id)
    }
    pub fn data(&self) -> Option<&AttackData> {
        self.swing.as_ref().map(|s| &s.data)
    }
    pub fn facing(&self) -> Option<i32> {
        self.swing.as_ref().map(|s| s.facing)
    }
    pub fn elapsed(&self) -> Option<u32> {
        self.swing.as_ref().map(|s| s.tick)
    }
    /// Turns a scripted attack without restarting its contact history.
    pub fn redirect(&mut self, facing: i32) {
        if let Some(s) = &mut self.swing {
            s.facing = if facing < 0 { -1 } else { 1 };
        }
    }

    pub fn held_ticks(&self) -> u32 {
        self.hold_ticks
    }

    pub fn phase(&self) -> Phase {
        match &self.swing {
            None => Phase::Idle,
            Some(s) if s.tick < s.data.windup => Phase::Windup,
            Some(s) if s.tick < s.data.windup + s.data.active => Phase::Active,
            Some(_) => Phase::Recovery,
        }
    }

    /// Zero-based tick within the current phase.
    pub fn phase_tick(&self) -> u32 {
        let Some(s) = &self.swing else {
            return 0;
        };
        match self.phase() {
            Phase::Windup => s.tick,
            Phase::Active => s.tick - s.data.windup,
            Phase::Recovery => s.tick - s.data.windup - s.data.active,
            Phase::Idle => 0,
        }
    }

    pub fn armoured(&self) -> bool {
        self.data().is_some_and(|d| d.armour)
    }

    pub fn motion(&self) -> AttackMotion {
        let Some(s) = &self.swing else {
            return AttackMotion::default();
        };
        AttackMotion {
            locked: true,
            step_velocity: if self.phase() == Phase::Recovery {
                0.0
            } else {
                s.data.step.unwrap_or(0.0) * s.facing as f32
            },
            air_hang: s.tick < s.data.air_hang,
        }
    }

    /// Starts a scripted move only when idle. Input-driven moves use [`Self::tick`].
    pub fn start(&mut self, weapon: &Weapon, id: AttackId, facing: i32) -> bool {
        if self.swing.is_some() {
            return false;
        }
        self.swing = Some(Swing {
            id,
            data: weapon.attack(id).clone(),
            tick: 0,
            facing: if facing < 0 { -1 } else { 1 },
            connected: false,
            targets: BTreeSet::new(),
        });
        true
    }

    /// Advances one tick and samples input. Frozen ticks accept presses without ageing timers.
    /// A press starts a normal swing immediately; a long hold adds a charged move on release.
    pub fn tick(
        &mut self,
        weapon: &Weapon,
        input: Controls,
        grounded: bool,
        facing: i32,
        frozen: bool,
    ) {
        if !frozen {
            if let Some(s) = &mut self.swing {
                s.tick += 1;
                if s.tick >= s.data.total_ticks() {
                    self.swing = None;
                }
            }
            if let Some(b) = &mut self.buffer {
                b.remaining = b.remaining.saturating_sub(1);
                if b.remaining == 0 {
                    self.buffer = None;
                }
            }
        }
        if input.attack_pressed {
            let next = self
                .swing
                .as_ref()
                .and_then(|s| s.data.chain.as_ref())
                .filter(|c| self.phase() == Phase::Recovery && c.window.contains(self.phase_tick()))
                .map(|c| c.next)
                .unwrap_or_else(|| Self::input_move(input, grounded));
            self.buffer = Some(Buffered {
                next,
                remaining: BUFFER_TICKS,
            });
            self.charge_armed = input.attack_held;
            self.hold_ticks = 0;
        }
        if self.charge_armed {
            if input.attack_held {
                if !frozen {
                    self.hold_ticks = self.hold_ticks.saturating_add(1);
                }
            } else {
                self.charged_release |= self.hold_ticks >= weapon.charge_ticks;
                self.charge_armed = false;
                self.hold_ticks = 0;
            }
        }
        if frozen || self.swing.is_some() {
            return;
        }
        if self.charged_release {
            self.charged_release = false;
            self.buffer = None;
            self.start(weapon, AttackId::Charged, facing);
        } else if let Some(buffer) = self.buffer.take() {
            self.start(weapon, buffer.next, facing);
        }
    }

    fn input_move(input: Controls, grounded: bool) -> AttackId {
        if !grounded {
            if input.y < -0.5 {
                AttackId::Pogo
            } else {
                AttackId::Air1
            }
        } else if input.y > 0.5 {
            AttackId::Rising
        } else {
            AttackId::Combo1
        }
    }

    pub fn can_lunge_cancel(&self) -> bool {
        self.swing.as_ref().is_some_and(|s| {
            s.connected
                && self.phase() == Phase::Recovery
                && self.phase_tick() >= s.data.cancel_window
        })
    }

    /// Ends the swing only after the movement controller actually starts an allowed Lunge.
    pub fn lunge_happened(&mut self) -> bool {
        if !self.can_lunge_cancel() {
            return false;
        }
        self.interrupt();
        true
    }

    pub fn active_hitboxes(&self, feet: (f32, f32)) -> Vec<Rect> {
        if self.phase() != Phase::Active {
            return Vec::new();
        }
        let s = self.swing.as_ref().expect("active swing");
        s.data
            .hitboxes
            .iter()
            .filter(|h| h.ticks.is_none_or(|r| r.contains(self.phase_tick())))
            .map(|h| h.rect.at_feet(feet, s.facing))
            .collect()
    }

    pub(crate) fn has_hit(&self, target: u64) -> bool {
        self.swing
            .as_ref()
            .is_some_and(|s| s.targets.contains(&target))
    }

    pub(crate) fn contact(&mut self, target: u64, connected: bool) {
        if let Some(s) = &mut self.swing {
            s.targets.insert(target);
            s.connected |= connected;
        }
    }

    pub(crate) fn interrupt(&mut self) {
        *self = Self::default();
    }
}
