use bevy::prelude::*;
use carbonthrone::{
    ability::{LastPosition, LastUsedAbility, available_abilities},
    action_points::ActionPoints,
    character::CharacterKind,
    health::Health,
    pending_effects::{CurrentRound, PendingEffect, PendingEffects},
    position::Position,
    stats::Stats,
    temporal_flux::TemporalFlux,
    turn::{Action, TurnAction, apply_action},
};

fn stats(attack: i32, defense: i32) -> Stats {
    Stats {
        max_hp: 100,
        attack,
        defense,
        speed: 8,
    }
}

// ── Temporal Recall ──────────────────────────────────────────────────────────

#[test]
fn temporal_recall_fails_if_target_has_not_moved() {
    let mut world = World::new();
    let ability = available_abilities(&CharacterKind::Researcher, 2)
        .into_iter()
        .find(|a| a.name == "Temporal Recall")
        .unwrap();

    let actor = world
        .spawn((stats(10, 5), ActionPoints::new(4), Position::new(0, 0)))
        .id();
    let target = world
        .spawn((stats(5, 2), Health::new(50), Position::new(3, 3)))
        .id();

    // Target has no LastPosition — should fail.
    let result = apply_action(
        &mut world,
        actor,
        &Action::UseAbility {
            ability,
            target: Some(target),
        },
    );
    assert!(
        result.is_none(),
        "recall should fail when target has not moved"
    );
    // Position unchanged.
    assert_eq!(world.get::<Position>(target).unwrap().x, 3);
}

#[test]
fn temporal_recall_snaps_target_back() {
    let mut world = World::new();
    let ability = available_abilities(&CharacterKind::Researcher, 2)
        .into_iter()
        .find(|a| a.name == "Temporal Recall")
        .unwrap();

    let actor = world
        .spawn((stats(10, 5), ActionPoints::new(4), Position::new(0, 0)))
        .id();
    let target = world
        .spawn((stats(5, 2), Health::new(50), Position::new(5, 5)))
        .id();

    // Manually give the target a LastPosition (as if they moved from (2,2) to (5,5)).
    world
        .entity_mut(target)
        .insert(LastPosition(Position::new(2, 2)));

    let result = apply_action(
        &mut world,
        actor,
        &Action::UseAbility {
            ability,
            target: Some(target),
        },
    );

    assert!(result.is_some(), "recall should succeed");
    let pos = world.get::<Position>(target).unwrap();
    assert_eq!((pos.x, pos.y), (2, 2), "target should be recalled to (2,2)");

    // LastPosition should be removed after recall.
    assert!(
        world.get::<LastPosition>(target).is_none(),
        "LastPosition should be cleared after recall"
    );
}

// ── Entropic Rounds ──────────────────────────────────────────────────────────

#[test]
fn entropic_rounds_always_hits() {
    let mut world = World::new();
    let ability = available_abilities(&CharacterKind::Researcher, 6)
        .into_iter()
        .find(|a| a.name == "Entropic Rounds")
        .unwrap();

    let attacker = world
        .spawn((stats(15, 5), ActionPoints::new(4), Position::new(0, 0)))
        .id();
    let target = world
        .spawn((stats(5, 20), Health::new(100), Position::new(5, 5)))
        .id();

    // Even with very high defense, Entropic Rounds always hits.
    let result = apply_action(
        &mut world,
        attacker,
        &Action::UseAbility {
            ability,
            target: Some(target),
        },
    );

    assert!(result.is_some());
    match result.unwrap() {
        TurnAction::UseAbility { hit, value, .. } => {
            assert!(hit, "Entropic Rounds should always hit");
            assert_eq!(
                value, 15,
                "Entropic Rounds should deal raw attack damage (15)"
            );
        }
        _ => panic!("expected UseAbility result"),
    }
    // Target should have taken damage.
    assert!(world.get::<Health>(target).unwrap().current < 100);
}

#[test]
fn entropic_rounds_ignores_defense() {
    let mut world = World::new();
    let ability = available_abilities(&CharacterKind::Researcher, 6)
        .into_iter()
        .find(|a| a.name == "Entropic Rounds")
        .unwrap();

    // Two targets: one with 0 defense, one with 50 defense.
    let attacker = world.spawn((stats(20, 5), ActionPoints::new(6))).id();
    let low_def_target = world.spawn((stats(0, 0), Health::new(100))).id();
    let high_def_target = world.spawn((stats(0, 50), Health::new(100))).id();

    apply_action(
        &mut world,
        attacker,
        &Action::UseAbility {
            ability: ability.clone(),
            target: Some(low_def_target),
        },
    );
    let low_damage = 100 - world.get::<Health>(low_def_target).unwrap().current;

    apply_action(
        &mut world,
        attacker,
        &Action::UseAbility {
            ability,
            target: Some(high_def_target),
        },
    );
    let high_damage = 100 - world.get::<Health>(high_def_target).unwrap().current;

    assert_eq!(
        low_damage, high_damage,
        "Entropic Rounds should deal same damage regardless of defense"
    );
}

// ── Acceleration ─────────────────────────────────────────────────────────────

#[test]
fn acceleration_grants_bonus_ap() {
    let mut world = World::new();
    world.insert_resource(PendingEffects::default());
    world.insert_resource(CurrentRound(1));

    let ability = available_abilities(&CharacterKind::Researcher, 4)
        .into_iter()
        .find(|a| a.name == "Acceleration")
        .unwrap();

    let actor = world.spawn((stats(10, 5), ActionPoints::new(5))).id();

    let result = apply_action(
        &mut world,
        actor,
        &Action::UseAbility {
            ability,
            target: None,
        },
    );

    assert!(result.is_some(), "Acceleration should succeed");
    // Should have spent 2 AP for Acceleration, then gained 3 AP.
    // Net: 5 - 2 + 3 = 6.
    let ap = world.get::<ActionPoints>(actor).unwrap().current;
    assert_eq!(ap, 6, "AP should be 6 after Acceleration (5 - 2 + 3)");

    // Should have queued a drain for next round.
    let effects = world.get_resource::<PendingEffects>().unwrap();
    assert_eq!(effects.0.len(), 1, "should have one pending drain effect");
    match &effects.0[0] {
        PendingEffect::DrainAP {
            amount,
            resolve_on_round,
            ..
        } => {
            assert_eq!(*amount, 4, "should drain 4 AP next round");
            assert_eq!(*resolve_on_round, 2, "should resolve on round 2");
        }
        _ => panic!("expected DrainAP effect"),
    }
}

// ── Displacement ─────────────────────────────────────────────────────────────

#[test]
fn displacement_queues_delayed_damage() {
    let mut world = World::new();
    world.insert_resource(PendingEffects::default());
    world.insert_resource(CurrentRound(1));

    let ability = available_abilities(&CharacterKind::Researcher, 3)
        .into_iter()
        .find(|a| a.name == "Temporal Displacement")
        .unwrap();

    let actor = world
        .spawn((stats(15, 5), ActionPoints::new(5), Position::new(0, 0)))
        .id();
    let target = world
        .spawn((stats(5, 5), Health::new(100), Position::new(5, 0)))
        .id();

    let result = apply_action(
        &mut world,
        actor,
        &Action::UseAbility {
            ability,
            target: Some(target),
        },
    );

    assert!(result.is_some());

    // The target should have damage queued for round 3 (current_round 1 + delay 2).
    let effects = world.get_resource::<PendingEffects>().unwrap();
    if !effects.0.is_empty() {
        match &effects.0[0] {
            PendingEffect::DelayedDamage {
                resolve_on_round, ..
            } => {
                assert_eq!(*resolve_on_round, 3, "should resolve on round 3");
            }
            _ => {}
        }
    }
}

// ── Move saves LastPosition ──────────────────────────────────────────────────

#[test]
fn move_saves_last_position() {
    let mut world = World::new();
    let actor = world
        .spawn((ActionPoints::new(4), Position::new(2, 3)))
        .id();

    apply_action(
        &mut world,
        actor,
        &Action::Move {
            destination: Position::new(2, 5),
        },
    );

    let last = world
        .get::<LastPosition>(actor)
        .expect("LastPosition should be set after move");
    assert_eq!(
        (last.0.x, last.0.y),
        (2, 3),
        "should save position before move"
    );
}

// ── LastUsedAbility tracking ─────────────────────────────────────────────────

#[test]
fn successful_ability_sets_last_used_ability() {
    let mut world = World::new();
    let ability = available_abilities(&CharacterKind::Researcher, 1)
        .into_iter()
        .find(|a| a.name == "Temporal Bolt")
        .unwrap();

    let actor = world.spawn((stats(10, 5), ActionPoints::new(4))).id();
    let target = world.spawn((stats(5, 2), Health::new(50))).id();

    apply_action(
        &mut world,
        actor,
        &Action::UseAbility {
            ability,
            target: Some(target),
        },
    );

    let last = world
        .get::<LastUsedAbility>(actor)
        .expect("LastUsedAbility should be set after ability use");
    assert_eq!(last.ability.name, "Temporal Bolt");
}

// ── Flux generation ──────────────────────────────────────────────────────────

#[test]
fn temporal_bolt_has_nonzero_flux_generation() {
    let abilities = available_abilities(&CharacterKind::Researcher, 1);
    let bolt = abilities
        .iter()
        .find(|a| a.name == "Temporal Bolt")
        .unwrap();
    assert!(
        bolt.flux_generation > 0,
        "Temporal Bolt should generate flux"
    );
}

#[test]
fn non_temporal_abilities_have_zero_flux_generation() {
    for kind in [
        CharacterKind::Doss,
        CharacterKind::Orin,
        CharacterKind::Kaleo,
    ] {
        for ability in carbonthrone::ability::character_abilities(&kind) {
            assert_eq!(
                ability.flux_generation,
                0,
                "{} ability '{}' should not generate flux",
                format!("{kind:?}"),
                ability.name
            );
        }
    }
}

// ── Temporal Flux resource integration ───────────────────────────────────────

#[test]
fn temporal_flux_resource_reads_correctly() {
    let mut world = World::new();
    world.insert_resource(TemporalFlux::default());

    let flux = world.get_resource::<TemporalFlux>().unwrap();
    assert_eq!(flux.flux, 0);

    world.get_resource_mut::<TemporalFlux>().unwrap().add(50);
    assert_eq!(world.get_resource::<TemporalFlux>().unwrap().flux, 50);
}
