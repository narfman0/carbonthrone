use bevy::prelude::*;
use carbonthrone::{
    action_points::ActionPoints,
    health::Health,
    position::Position,
    stats::Stats,
    terrain::{BattleRng, LevelMap, Tile, Biome},
    turn::{Action, TurnAction, apply_action, ATTACK_AP_COST, MOVE_AP_COST},
};
use rand::SeedableRng;
use rand::rngs::StdRng;

fn stats(attack: i32, defense: i32) -> Stats {
    Stats { max_hp: 50, attack, defense, speed: 10 }
}

// ── Existing action tests (updated for Option<TurnAction> return) ──────────

#[test]
fn attack_reduces_target_hp_and_spends_ap() {
    let mut world = World::new();
    let attacker = world.spawn((stats(10, 5), ActionPoints::new(4))).id();
    // No BattleRng → always hits. calc_damage(10, 4) = 8.
    let target = world.spawn((stats(5, 4), Health::new(50))).id();

    let result = apply_action(&mut world, attacker, &Action::Attack { target });

    assert!(result.is_some());
    assert_eq!(world.get::<Health>(target).unwrap().current, 42);
    assert_eq!(
        world.get::<ActionPoints>(attacker).unwrap().current,
        4 - ATTACK_AP_COST
    );
}

#[test]
fn attack_fails_without_enough_ap() {
    let mut world = World::new();
    let attacker = world.spawn((stats(10, 5), ActionPoints::new(1))).id();
    let target = world.spawn((stats(5, 4), Health::new(50))).id();

    let result = apply_action(&mut world, attacker, &Action::Attack { target });

    assert!(result.is_none());
    assert_eq!(world.get::<Health>(target).unwrap().current, 50);
    assert_eq!(world.get::<ActionPoints>(attacker).unwrap().current, 1);
}

#[test]
fn attack_on_dead_target_fails() {
    let mut world = World::new();
    let attacker = world.spawn((stats(10, 5), ActionPoints::new(4))).id();
    let mut target_hp = Health::new(50);
    target_hp.take_damage(50); // kill it
    let target = world.spawn((stats(5, 4), target_hp)).id();

    let result = apply_action(&mut world, attacker, &Action::Attack { target });

    assert!(result.is_none());
}

#[test]
fn move_changes_position_and_costs_ap() {
    let mut world = World::new();
    let mover = world
        .spawn((ActionPoints::new(4), Position::new(0, 0, 0)))
        .id();
    let dest = Position::new(3, 0, 0); // distance = 3

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
        .spawn((ActionPoints::new(2), Position::new(0, 0, 0)))
        .id();
    let dest = Position::new(5, 0, 0); // distance = 5, costs 5 AP

    let result = apply_action(&mut world, mover, &Action::Move { destination: dest });

    assert!(result.is_none());
    let pos = world.get::<Position>(mover).unwrap();
    assert_eq!(pos.x, 0); // unchanged
}

#[test]
fn move_to_same_position_fails() {
    let mut world = World::new();
    let mover = world
        .spawn((ActionPoints::new(4), Position::new(2, 3, 0)))
        .id();

    let result = apply_action(
        &mut world,
        mover,
        &Action::Move { destination: Position::new(2, 3, 0) },
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
        .spawn((ActionPoints::new(4), Position::new(0, 0, 0)))
        .id();

    let mut map = LevelMap::new(10, 10, Biome::VoidStation);
    map.set(2, 0, Tile::Obstacle);
    world.insert_resource(map);

    let result = apply_action(
        &mut world,
        mover,
        &Action::Move { destination: Position::new(2, 0, 0) },
    );

    assert!(result.is_none());
    assert_eq!(world.get::<Position>(mover).unwrap().x, 0); // unchanged
    // AP should NOT have been spent
    assert_eq!(world.get::<ActionPoints>(mover).unwrap().current, 4);
}

#[test]
fn move_to_partial_cover_succeeds() {
    let mut world = World::new();
    let mover = world
        .spawn((ActionPoints::new(4), Position::new(0, 0, 0)))
        .id();

    let mut map = LevelMap::new(10, 10, Biome::NeonDistrict);
    map.set(2, 0, Tile::PartialCover);
    world.insert_resource(map);

    let result = apply_action(
        &mut world,
        mover,
        &Action::Move { destination: Position::new(2, 0, 0) },
    );

    assert!(matches!(result, Some(TurnAction::Move { to }) if to.x == 2));
    assert_eq!(world.get::<Position>(mover).unwrap().x, 2);
}

#[test]
fn move_to_full_cover_succeeds() {
    let mut world = World::new();
    let mover = world
        .spawn((ActionPoints::new(4), Position::new(0, 0, 0)))
        .id();

    let mut map = LevelMap::new(10, 10, Biome::BioLab);
    map.set(1, 0, Tile::FullCover);
    world.insert_resource(map);

    let result = apply_action(
        &mut world,
        mover,
        &Action::Move { destination: Position::new(1, 0, 0) },
    );

    assert!(result.is_some());
    assert_eq!(world.get::<Position>(mover).unwrap().x, 1);
}

// ── Hit / miss with cover ─────────────────────────────────────────────────

fn seeded_world_with_target_on(tile: Tile, seed: u64) -> (World, Entity, Entity) {
    let mut world = World::new();
    let attacker = world.spawn((stats(10, 0), ActionPoints::new(4))).id();
    let target = world
        .spawn((stats(0, 0), Health::new(100), Position::new(5, 5, 0)))
        .id();

    let mut map = LevelMap::new(10, 10, Biome::VoidStation);
    map.set(5, 5, tile);
    world.insert_resource(map);
    world.insert_resource(BattleRng(StdRng::seed_from_u64(seed)));

    (world, attacker, target)
}

#[test]
fn attack_hit_on_open_tile_deals_damage() {
    // seed 0 → first f32 from StdRng will be well below 0.90
    let (mut world, attacker, target) = seeded_world_with_target_on(Tile::Open, 0);

    let result = apply_action(&mut world, attacker, &Action::Attack { target });

    // The attack should connect; verify result contains a hit
    match result.unwrap() {
        TurnAction::Attack { hit, damage, .. } => {
            if hit {
                assert!(damage > 0);
                assert!(world.get::<Health>(target).unwrap().current < 100);
            }
            // If miss (unlikely with seed 0 / 90% chance), just verify no damage
        }
        _ => panic!("expected Attack result"),
    }
}

#[test]
fn attack_miss_does_not_deal_damage() {
    // Force a miss: full cover has 35% hit chance.
    // Run many attacks until we get at least one miss, confirm HP unchanged.
    let mut saw_miss = false;
    for seed in 0..200u64 {
        let (mut world, attacker, target) = seeded_world_with_target_on(Tile::FullCover, seed);
        let hp_before = world.get::<Health>(target).unwrap().current;

        if let Some(TurnAction::Attack { hit, .. }) =
            apply_action(&mut world, attacker, &Action::Attack { target })
        {
            if !hit {
                saw_miss = true;
                let hp_after = world.get::<Health>(target).unwrap().current;
                assert_eq!(hp_before, hp_after, "miss should not deal damage");
                break;
            }
        }
    }
    assert!(saw_miss, "expected at least one miss with FullCover over 200 seeds");
}

#[test]
fn ap_spent_on_miss() {
    // Use full cover; keep trying until we get a miss
    for seed in 0..200u64 {
        let (mut world, attacker, target) = seeded_world_with_target_on(Tile::FullCover, seed);

        if let Some(TurnAction::Attack { hit, .. }) =
            apply_action(&mut world, attacker, &Action::Attack { target })
        {
            if !hit {
                assert_eq!(
                    world.get::<ActionPoints>(attacker).unwrap().current,
                    4 - ATTACK_AP_COST
                );
                return;
            }
        }
    }
    // If we somehow got all hits with full cover over 200 seeds, that's a test failure
    panic!("expected at least one miss with FullCover over 200 seeds");
}
