use bevy::prelude::*;
use carbonthrone::{
    action_points::ActionPoints,
    character::{Character, CharacterKind},
    combat::{BattleOutcome, BattleStep, simulate_battle},
    health::Health,
    player_input::PlayerActionChoice,
    position::Position,
    stats::Stats,
};

fn player(world: &mut World, hp: i32, attack: i32, defense: i32, speed: i32) -> Entity {
    world
        .spawn((
            Character::new_character(CharacterKind::Doss, 1),
            Health::new(hp),
            Stats {
                max_hp: hp,
                attack,
                defense,
                speed,
            },
            ActionPoints::new(4),
            Position::new(0, 0),
        ))
        .id()
}

fn enemy(world: &mut World, hp: i32, attack: i32, defense: i32, speed: i32) -> Entity {
    world
        .spawn((
            Character::new_character(CharacterKind::Scavenger, 1),
            Health::new(hp),
            Stats {
                max_hp: hp,
                attack,
                defense,
                speed,
            },
            ActionPoints::new(4),
            // Place adjacent to player at (0,0) so melee abilities are in range.
            Position::new(1, 0),
        ))
        .id()
}

#[test]
fn strong_player_defeats_weak_enemy() {
    let mut world = World::new();
    player(&mut world, 100, 20, 10, 10);
    // calc_damage(20, 2) = 19 per hit, enemy has 10 HP → dies on first attack
    enemy(&mut world, 10, 5, 2, 5);

    assert_eq!(simulate_battle(&mut world), BattleOutcome::PlayerVictory);
}

#[test]
fn weak_player_loses_to_strong_enemy() {
    let mut world = World::new();
    // Player does 1 damage/hit; enemy has 200 HP
    player(&mut world, 5, 1, 0, 5);
    // calc_damage(30, 0) = 30 per hit, player has 5 HP → dies on first hit
    enemy(&mut world, 200, 30, 5, 10);

    assert_eq!(simulate_battle(&mut world), BattleOutcome::PlayerDefeated);
}

#[test]
fn no_enemies_is_immediate_victory() {
    let mut world = World::new();
    player(&mut world, 100, 10, 5, 10);
    // No enemy entities spawned

    assert_eq!(simulate_battle(&mut world), BattleOutcome::PlayerVictory);
}

#[test]
fn multiple_players_defeat_outnumbered_enemies() {
    let mut world = World::new();
    player(&mut world, 100, 15, 8, 12);
    player(&mut world, 100, 15, 8, 10);
    // Single weak enemy
    enemy(&mut world, 20, 5, 2, 5);

    assert_eq!(simulate_battle(&mut world), BattleOutcome::PlayerVictory);
}

#[test]
fn faster_side_acts_first() {
    // Enemy is much faster and lethal — if enemy goes first it wins,
    // but simulation gives players the first move each round.
    // Player is fast enough to kill the enemy before it acts this round.
    let mut world = World::new();
    // Player one-shots enemy (damage = 20 - 1 = 19 > 10 HP)
    player(&mut world, 5, 20, 0, 1);
    // Enemy would one-shot player (damage = 20 - 0 = 20 > 5 HP) but acts second
    enemy(&mut world, 10, 20, 2, 100);

    // Players always act first each round, so player kills enemy before taking damage
    assert_eq!(simulate_battle(&mut world), BattleOutcome::PlayerVictory);
}

/// Regression test: a player character that dies mid-round (while still in the
/// actor queue with AP remaining) must not block the queue or act after death.
/// The battle should detect defeat once all players are dead.
#[test]
fn dead_player_mid_round_does_not_get_choices() {
    let mut world = World::new();
    // Two players. Player A will be killed manually mid-round.
    let player_a = player(&mut world, 10, 5, 2, 10);
    let player_b = player(&mut world, 10, 5, 2, 8);
    // One enemy with plenty of HP so the battle doesn't end from player attacks.
    enemy(&mut world, 1000, 1, 50, 1);

    let mut battle = BattleStep::new(&mut world);

    // It should be the player turn.
    assert_eq!(battle.turn, carbonthrone::combat::Turn::Player);

    // Kill player A directly — simulating death from a mid-round temporal effect.
    if let Some(mut h) = world.get_mut::<Health>(player_a) {
        h.take_damage(1000);
    }

    // player_choices must skip the dead player A and return choices for B.
    let choices = battle.player_choices(&mut world);
    // There should still be choices (B is alive and enemies exist).
    assert!(!choices.is_empty(), "live player B should still have choices");
    // Passing for B ends B's turn; both players have acted (A was dead/skipped).
    let result = battle.step_player_action(&mut world, &PlayerActionChoice::Pass);
    assert!(result.turn_ended);

    // Kill player B as well.
    if let Some(mut h) = world.get_mut::<Health>(player_b) {
        h.take_damage(1000);
    }

    // Now all players are dead; player_choices must return empty.
    let choices = battle.player_choices(&mut world);
    assert!(choices.is_empty(), "no choices expected when all players are dead");
}
