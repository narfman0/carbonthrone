use bevy::prelude::*;
use carbonthrone::{
    ability::available_abilities,
    action_points::ActionPoints,
    character::CharacterKind,
    health::Health,
    position::Position,
    stats::Stats,
    terrain::{BattleRng, LevelMap, Tile},
    turn::{Action, MOVE_AP_COST, TurnAction, apply_action},
    zone::ZoneKind,
};
use rand::SeedableRng;
use rand::rngs::StdRng;

fn stats(attack: i32, defense: i32) -> Stats {
    Stats {
        max_hp: 50,
        attack,
        defense,
        speed: 10,
    }
}

// ── UseAbility action tests ────────────────────────────────────────────────

#[test]
fn ranged_ability_reduces_target_hp_and_spends_ap() {
    let mut world = World::new();
    // Aimed Shot: Ranged, ap_cost 2, BonusDamage { bonus: 5 }
    let ability = available_abilities(&CharacterKind::Kaleo, 1)
        .into_iter()
        .find(|a| a.name == "Aimed Shot")
        .unwrap();
    assert_eq!(ability.ap_cost, 2);

    // No BattleRng → always hits. calc_damage(10, 4) + 5 = 13.
    let attacker = world.spawn((stats(10, 5), ActionPoints::new(4))).id();
    let target = world.spawn((stats(5, 4), Health::new(50))).id();

    let result = apply_action(
        &mut world,
        attacker,
        &Action::UseAbility {
            ability: ability.clone(),
            target: Some(target),
        },
    );

    assert!(result.is_some());
    assert_eq!(world.get::<Health>(target).unwrap().current, 37); // 50 - 13
    assert_eq!(
        world.get::<ActionPoints>(attacker).unwrap().current,
        4 - ability.ap_cost
    );
}

#[test]
fn ability_fails_without_enough_ap() {
    let mut world = World::new();
    let ability = available_abilities(&CharacterKind::Kaleo, 1)
        .into_iter()
        .find(|a| a.name == "Aimed Shot")
        .unwrap();
    assert_eq!(ability.ap_cost, 2);

    let attacker = world.spawn((stats(10, 5), ActionPoints::new(1))).id(); // only 1 AP
    let target = world.spawn((stats(5, 4), Health::new(50))).id();

    let result = apply_action(
        &mut world,
        attacker,
        &Action::UseAbility {
            ability,
            target: Some(target),
        },
    );

    assert!(result.is_none());
    assert_eq!(world.get::<Health>(target).unwrap().current, 50);
    assert_eq!(world.get::<ActionPoints>(attacker).unwrap().current, 1);
}

#[test]
fn ability_on_dead_target_fails() {
    let mut world = World::new();
    let ability = available_abilities(&CharacterKind::Kaleo, 1)
        .into_iter()
        .find(|a| a.name == "Aimed Shot")
        .unwrap();

    let attacker = world.spawn((stats(10, 5), ActionPoints::new(4))).id();
    let mut target_hp = Health::new(50);
    target_hp.take_damage(50); // kill it
    let target = world.spawn((stats(5, 4), target_hp)).id();

    let result = apply_action(
        &mut world,
        attacker,
        &Action::UseAbility {
            ability,
            target: Some(target),
        },
    );

    assert!(result.is_none());
}

#[test]
fn melee_ability_blocked_when_target_not_adjacent() {
    let mut world = World::new();
    // Power Strike: Melee, ap_cost 3
    let ability = available_abilities(&CharacterKind::Doss, 1)
        .into_iter()
        .find(|a| a.name == "Power Strike")
        .unwrap();

    // Chebyshev distance = 5 → not adjacent → should fail.
    let attacker = world
        .spawn((stats(10, 5), ActionPoints::new(4), Position::new(0, 0)))
        .id();
    let target = world
        .spawn((stats(5, 4), Health::new(50), Position::new(5, 0)))
        .id();

    let result = apply_action(
        &mut world,
        attacker,
        &Action::UseAbility {
            ability,
            target: Some(target),
        },
    );

    assert!(
        result.is_none(),
        "melee ability should fail when not adjacent"
    );
    assert_eq!(world.get::<Health>(target).unwrap().current, 50);
    assert_eq!(world.get::<ActionPoints>(attacker).unwrap().current, 4);
}

#[test]
fn melee_ability_succeeds_when_adjacent() {
    let mut world = World::new();
    let ability = available_abilities(&CharacterKind::Doss, 1)
        .into_iter()
        .find(|a| a.name == "Power Strike")
        .unwrap();

    // Chebyshev distance = 1 → adjacent → should succeed.
    let attacker = world
        .spawn((stats(10, 5), ActionPoints::new(4), Position::new(0, 0)))
        .id();
    let target = world
        .spawn((stats(5, 4), Health::new(50), Position::new(1, 0)))
        .id();

    let result = apply_action(
        &mut world,
        attacker,
        &Action::UseAbility {
            ability,
            target: Some(target),
        },
    );

    assert!(
        result.is_some(),
        "melee ability should succeed when adjacent"
    );
    assert!(world.get::<Health>(target).unwrap().current < 50);
}

#[test]
fn melee_ability_succeeds_on_diagonal() {
    let mut world = World::new();
    let ability = available_abilities(&CharacterKind::Doss, 1)
        .into_iter()
        .find(|a| a.name == "Power Strike")
        .unwrap();

    // Diagonal adjacency: Chebyshev distance = max(1,1) = 1.
    let attacker = world
        .spawn((stats(10, 5), ActionPoints::new(4), Position::new(0, 0)))
        .id();
    let target = world
        .spawn((stats(5, 4), Health::new(50), Position::new(1, 1)))
        .id();

    let result = apply_action(
        &mut world,
        attacker,
        &Action::UseAbility {
            ability,
            target: Some(target),
        },
    );

    assert!(
        result.is_some(),
        "melee ability should allow diagonal adjacency"
    );
    assert!(world.get::<Health>(target).unwrap().current < 50);
}

#[test]
fn ranged_ability_works_without_positions() {
    // Ranged abilities have no positional restriction; no Position components needed.
    let mut world = World::new();
    let ability = available_abilities(&CharacterKind::Kaleo, 1)
        .into_iter()
        .find(|a| a.name == "Aimed Shot")
        .unwrap();

    let attacker = world.spawn((stats(10, 5), ActionPoints::new(4))).id();
    let target = world.spawn((stats(5, 4), Health::new(50))).id();

    let result = apply_action(
        &mut world,
        attacker,
        &Action::UseAbility {
            ability,
            target: Some(target),
        },
    );

    assert!(
        result.is_some(),
        "ranged ability should work without Position components"
    );
}

#[test]
fn move_changes_position_and_costs_ap() {
    let mut world = World::new();
    let mover = world
        .spawn((ActionPoints::new(4), Position::new(0, 0)))
        .id();
    let dest = Position::new(3, 0); // distance = 3

    let result = apply_action(&mut world, mover, &Action::Move { destination: dest });

    assert!(result.is_some());
    let pos = world.get::<Position>(mover).unwrap();
    assert_eq!(pos.x, 3);
    assert_eq!(
        world.get::<ActionPoints>(mover).unwrap().current,
        4 - MOVE_AP_COST * 3
    );
}

#[test]
fn move_fails_without_enough_ap() {
    let mut world = World::new();
    let mover = world
        .spawn((ActionPoints::new(2), Position::new(0, 0)))
        .id();
    let dest = Position::new(5, 0); // distance = 5, costs 5 AP

    let result = apply_action(&mut world, mover, &Action::Move { destination: dest });

    assert!(result.is_none());
    let pos = world.get::<Position>(mover).unwrap();
    assert_eq!(pos.x, 0); // unchanged
}

#[test]
fn move_to_same_position_fails() {
    let mut world = World::new();
    let mover = world
        .spawn((ActionPoints::new(4), Position::new(2, 3)))
        .id();

    let result = apply_action(
        &mut world,
        mover,
        &Action::Move {
            destination: Position::new(2, 3),
        },
    );

    assert!(result.is_none());
}

#[test]
fn pass_does_not_change_ap() {
    let mut world = World::new();
    let actor = world.spawn(ActionPoints::new(4)).id();

    let result = apply_action(&mut world, actor, &Action::Pass);

    assert!(result.is_none()); // Pass returns None (nothing to log)
    assert_eq!(world.get::<ActionPoints>(actor).unwrap().current, 4);
}

// ── Obstacle blocking ─────────────────────────────────────────────────────

#[test]
fn move_to_obstacle_is_blocked() {
    let mut world = World::new();
    let mover = world
        .spawn((ActionPoints::new(4), Position::new(0, 0)))
        .id();

    let mut map = LevelMap::new(10, 10, ZoneKind::CommandDeck);
    map.set(2, 0, Tile::Obstacle);
    world.insert_resource(map);

    let result = apply_action(
        &mut world,
        mover,
        &Action::Move {
            destination: Position::new(2, 0),
        },
    );

    assert!(result.is_none());
    assert_eq!(world.get::<Position>(mover).unwrap().x, 0); // unchanged
    assert_eq!(world.get::<ActionPoints>(mover).unwrap().current, 4);
}

// ── Hit / miss with cover ─────────────────────────────────────────────────

/// Build a world where the target at (5,5) has Full cover from North.
/// Attacker at (5,0) → attack direction North → obstacle at (5,4) provides Full cover.
fn seeded_world_with_full_cover(seed: u64) -> (World, Entity, Entity) {
    let mut world = World::new();
    // Use Aimed Shot (Ranged) so no adjacency requirement.
    let attacker = world
        .spawn((stats(10, 0), ActionPoints::new(4), Position::new(5, 0)))
        .id();
    let target = world
        .spawn((stats(0, 0), Health::new(100), Position::new(5, 5)))
        .id();

    let mut map = LevelMap::new(10, 10, ZoneKind::CommandDeck);
    map.set(5, 4, Tile::Obstacle); // directly north of target
    map.recompute_cover();
    world.insert_resource(map);
    world.insert_resource(BattleRng(StdRng::seed_from_u64(seed)));

    (world, attacker, target)
}

#[test]
fn ability_hit_on_open_tile_deals_damage() {
    // No LevelMap → CoverLevel::None by default → 90% hit chance.
    let mut world = World::new();
    let ability = available_abilities(&CharacterKind::Kaleo, 1)
        .into_iter()
        .find(|a| a.name == "Aimed Shot")
        .unwrap();

    let attacker = world.spawn((stats(10, 0), ActionPoints::new(4))).id();
    let target = world
        .spawn((stats(0, 0), Health::new(100), Position::new(5, 5)))
        .id();
    world.insert_resource(BattleRng(StdRng::seed_from_u64(0)));

    let result = apply_action(
        &mut world,
        attacker,
        &Action::UseAbility {
            ability,
            target: Some(target),
        },
    );

    match result.unwrap() {
        TurnAction::UseAbility { hit, value, .. } => {
            if hit {
                assert!(value > 0);
                assert!(world.get::<Health>(target).unwrap().current < 100);
            }
        }
        _ => panic!("expected UseAbility result"),
    }
}

#[test]
fn ability_miss_does_not_deal_damage() {
    // Full cover → 35% hit chance; scan seeds until we find a miss.
    let ability = available_abilities(&CharacterKind::Kaleo, 1)
        .into_iter()
        .find(|a| a.name == "Aimed Shot")
        .unwrap();

    let mut saw_miss = false;
    for seed in 0..200u64 {
        let (mut world, attacker, target) = seeded_world_with_full_cover(seed);
        let hp_before = world.get::<Health>(target).unwrap().current;

        if let Some(TurnAction::UseAbility { hit, .. }) = apply_action(
            &mut world,
            attacker,
            &Action::UseAbility {
                ability: ability.clone(),
                target: Some(target),
            },
        ) {
            if !hit {
                saw_miss = true;
                let hp_after = world.get::<Health>(target).unwrap().current;
                assert_eq!(hp_before, hp_after, "miss should not deal damage");
                break;
            }
        }
    }
    assert!(
        saw_miss,
        "expected at least one miss with Full cover over 200 seeds"
    );
}

#[test]
fn ap_spent_on_miss() {
    // Full cover; confirm AP is still spent even on a miss.
    let ability = available_abilities(&CharacterKind::Kaleo, 1)
        .into_iter()
        .find(|a| a.name == "Aimed Shot")
        .unwrap();
    let ap_cost = ability.ap_cost;

    for seed in 0..200u64 {
        let (mut world, attacker, target) = seeded_world_with_full_cover(seed);

        if let Some(TurnAction::UseAbility { hit, .. }) = apply_action(
            &mut world,
            attacker,
            &Action::UseAbility {
                ability: ability.clone(),
                target: Some(target),
            },
        ) {
            if !hit {
                assert_eq!(
                    world.get::<ActionPoints>(attacker).unwrap().current,
                    4 - ap_cost
                );
                return;
            }
        }
    }
    panic!("expected at least one miss with Full cover over 200 seeds");
}
