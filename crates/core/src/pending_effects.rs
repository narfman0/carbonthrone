use bevy::prelude::{Entity, Resource};

/// Tracks the current battle round so delayed effects can resolve at the right time.
/// Inserted by [`crate::combat::BattleStep::new`] and updated each round.
#[derive(Resource, Default)]
pub struct CurrentRound(pub u32);

/// A single deferred combat effect that resolves on a specific round.
pub enum PendingEffect {
    /// Damage applied to a target on a later round (e.g. from Displacement).
    DelayedDamage {
        target: Entity,
        damage: i32,
        resolve_on_round: u32,
    },
    /// AP drain applied to a target on a later round (e.g. from Acceleration recoil).
    DrainAP {
        target: Entity,
        amount: i32,
        resolve_on_round: u32,
    },
}

/// Bevy resource holding all pending deferred combat effects.
#[derive(Resource, Default)]
pub struct PendingEffects(pub Vec<PendingEffect>);

impl PendingEffects {
    /// Remove and return all effects scheduled to resolve on `current_round`.
    pub fn drain_ready(&mut self, current_round: u32) -> Vec<PendingEffect> {
        let mut ready = Vec::new();
        let mut i = 0;
        while i < self.0.len() {
            let resolve_on = match &self.0[i] {
                PendingEffect::DelayedDamage {
                    resolve_on_round, ..
                } => *resolve_on_round,
                PendingEffect::DrainAP {
                    resolve_on_round, ..
                } => *resolve_on_round,
            };
            if resolve_on == current_round {
                ready.push(self.0.remove(i));
            } else {
                i += 1;
            }
        }
        ready
    }
}
