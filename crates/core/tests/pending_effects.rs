use bevy::prelude::*;
use carbonthrone::pending_effects::{CurrentRound, PendingEffect, PendingEffects};

#[test]
fn pending_effects_starts_empty() {
    let effects = PendingEffects::default();
    assert!(effects.0.is_empty());
}

#[test]
fn drain_ready_returns_matching_round_effects() {
    let mut effects = PendingEffects::default();
    let entity = Entity::from_bits(0x0000_0001_0000_0001);
    effects.0.push(PendingEffect::DelayedDamage {
        target: entity,
        damage: 10,
        resolve_on_round: 3,
    });
    effects.0.push(PendingEffect::DrainAP {
        target: entity,
        amount: 2,
        resolve_on_round: 5,
    });

    let ready = effects.drain_ready(3);
    assert_eq!(ready.len(), 1);
    match &ready[0] {
        PendingEffect::DelayedDamage {
            damage,
            resolve_on_round,
            ..
        } => {
            assert_eq!(*damage, 10);
            assert_eq!(*resolve_on_round, 3);
        }
        _ => panic!("expected DelayedDamage"),
    }
    // Round 5 effect still pending.
    assert_eq!(effects.0.len(), 1);
}

#[test]
fn drain_ready_removes_effects_from_list() {
    let mut effects = PendingEffects::default();
    let entity = Entity::from_bits(0x0000_0001_0000_0001);
    for i in 0..5u32 {
        effects.0.push(PendingEffect::DelayedDamage {
            target: entity,
            damage: i as i32,
            resolve_on_round: i,
        });
    }

    let ready = effects.drain_ready(2);
    assert_eq!(ready.len(), 1);
    assert_eq!(effects.0.len(), 4);
}

#[test]
fn drain_ready_returns_empty_when_no_match() {
    let mut effects = PendingEffects::default();
    let entity = Entity::from_bits(0x0000_0001_0000_0001);
    effects.0.push(PendingEffect::DrainAP {
        target: entity,
        amount: 1,
        resolve_on_round: 10,
    });

    let ready = effects.drain_ready(1);
    assert!(ready.is_empty());
    assert_eq!(effects.0.len(), 1);
}

#[test]
fn current_round_starts_at_zero_default() {
    let cr = CurrentRound::default();
    assert_eq!(cr.0, 0);
}
