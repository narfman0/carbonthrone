use bevy::prelude::*;
use carbonthrone::{
    action_points::ActionPoints,
    health::Health,
    position::Position,
    stats::Stats,
    turn::{Action, apply_action, ATTACK_AP_COST, MOVE_AP_COST},
};

fn stats(attack: i32, defense: i32) -> Stats {
    Stats { max_hp: 50, attack, defense, speed: 10 }
}

#[test]
fn attack_reduces_target_hp_and_spends_ap() {
    let mut world = World::new();
    let attacker = world.spawn((stats(10, 5), ActionPoints::new(4))).id();
    // calc_damage(10, 4) = (10 - 4/2).max(1) = 8
    let target = world.spawn((stats(5, 4), Health::new(50))).id();

    let result = apply_action(&mut world, attacker, &Action::Attack { target });

    assert!(result);
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

    assert!(!result);
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

    assert!(!result);
}

#[test]
fn move_changes_position_and_costs_ap() {
    let mut world = World::new();
    let mover = world
        .spawn((ActionPoints::new(4), Position::new(0, 0, 0)))
        .id();
    let dest = Position::new(3, 0, 0); // distance = 3

    let result = apply_action(&mut world, mover, &Action::Move { destination: dest });

    assert!(result);
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

    assert!(!result);
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

    assert!(!result);
}

#[test]
fn pass_always_succeeds() {
    let mut world = World::new();
    let actor = world.spawn(ActionPoints::new(4)).id();

    let result = apply_action(&mut world, actor, &Action::Pass);

    assert!(result);
    assert_eq!(world.get::<ActionPoints>(actor).unwrap().current, 4);
}
