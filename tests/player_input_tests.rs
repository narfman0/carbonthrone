use bevy::prelude::*;
use carbonthrone::{
    action_points::ActionPoints,
    character::Character,
    combat::BASE_HIT_CHANCE,
    health::Health,
    npc::CharacterKind,
    player_input::{PlayerActionChoice, available_player_actions},
    position::Position,
    simulation::{BattleOutcome, BattleStep},
    stats::Stats,
    terrain::{Biome, CoverLevel, LevelMap, Tile},
};

// ── Helpers ────────────────────────────────────────────────────────────────────

fn spawn_player(world: &mut World, pos: (i32, i32), attack: i32, defense: i32) -> Entity {
    let ch = Character::new_player("Player", CharacterKind::Doss);
    world
        .spawn((
            ch,
            Health::new(100),
            Stats {
                max_hp: 100,
                attack,
                defense,
                speed: 10,
            },
            ActionPoints::new(4),
            Position::new(pos.0, pos.1, 0),
        ))
        .id()
}

fn spawn_enemy(world: &mut World, pos: (i32, i32), defense: i32) -> Entity {
    let ch = Character::new_npc(CharacterKind::Scavenger, 1);
    world
        .spawn((
            ch,
            Health::new(50),
            Stats {
                max_hp: 50,
                attack: 5,
                defense,
                speed: 5,
            },
            ActionPoints::new(4),
            Position::new(pos.0, pos.1, 0),
        ))
        .id()
}

// ── available_player_actions ───────────────────────────────────────────────────

#[test]
fn always_ends_with_pass() {
    let mut world = World::new();
    let actor = spawn_player(&mut world, (0, 0), 10, 5);
    let choices = available_player_actions(&mut world, actor);
    assert!(matches!(choices.last(), Some(PlayerActionChoice::Pass)));
}

#[test]
fn attack_choice_listed_for_each_living_enemy() {
    let mut world = World::new();
    let actor = spawn_player(&mut world, (0, 0), 10, 5);
    spawn_enemy(&mut world, (5, 0), 4);
    spawn_enemy(&mut world, (0, 5), 2);

    let choices = available_player_actions(&mut world, actor);
    let attacks: Vec<_> = choices
        .iter()
        .filter(|c| matches!(c, PlayerActionChoice::Attack { .. }))
        .collect();
    assert_eq!(attacks.len(), 2, "one attack choice per living enemy");
}

#[test]
fn attack_choice_has_correct_hit_chance_no_cover() {
    let mut world = World::new();
    let actor = spawn_player(&mut world, (0, 0), 10, 5);
    spawn_enemy(&mut world, (5, 0), 4);

    let choices = available_player_actions(&mut world, actor);
    for choice in &choices {
        if let PlayerActionChoice::Attack {
            hit_chance, cover, ..
        } = choice
        {
            assert_eq!(*cover, CoverLevel::None);
            assert!(
                (hit_chance - BASE_HIT_CHANCE).abs() < 0.001,
                "no-cover hit chance should be {BASE_HIT_CHANCE}"
            );
        }
    }
}

#[test]
fn attack_choice_reflects_partial_cover() {
    // Target at (5,5); obstacle diagonally from North-West → partial cover from West.
    let mut world = World::new();
    let actor = spawn_player(&mut world, (0, 5), 10, 5); // attacker approaches from West
    spawn_enemy(&mut world, (5, 5), 4);

    let mut map = LevelMap::new(10, 10, Biome::VoidStation);
    // Obstacle at (4,4) — diagonal neighbor of (5,5) from the west direction.
    map.set(4, 4, Tile::Obstacle);
    map.recompute_cover();
    world.insert_resource(map);

    let choices = available_player_actions(&mut world, actor);
    let attack_opt = choices.iter().find_map(|c| {
        if let PlayerActionChoice::Attack {
            hit_chance, cover, ..
        } = c
        {
            Some((*hit_chance, *cover))
        } else {
            None
        }
    });
    let (hit_chance, cover) = attack_opt.expect("expected an attack choice");
    assert_eq!(cover, CoverLevel::Partial);
    assert!(
        (hit_chance - 0.65).abs() < 0.001,
        "partial cover → 65% hit chance"
    );
}

#[test]
fn attack_choice_reflects_full_cover() {
    // Target at (5,5); obstacle directly north at (5,4) — Full cover from North.
    let mut world = World::new();
    let actor = spawn_player(&mut world, (5, 0), 10, 5); // approaches from North
    spawn_enemy(&mut world, (5, 5), 4);

    let mut map = LevelMap::new(10, 10, Biome::VoidStation);
    map.set(5, 4, Tile::Obstacle);
    map.recompute_cover();
    world.insert_resource(map);

    let choices = available_player_actions(&mut world, actor);
    let (hit_chance, cover) = choices
        .iter()
        .find_map(|c| {
            if let PlayerActionChoice::Attack {
                hit_chance, cover, ..
            } = c
            {
                Some((*hit_chance, *cover))
            } else {
                None
            }
        })
        .expect("expected an attack choice");

    assert_eq!(cover, CoverLevel::Full);
    assert!(
        (hit_chance - 0.35).abs() < 0.001,
        "full cover → 35% hit chance"
    );
}

#[test]
fn attack_choice_reflects_correct_damage() {
    // calc_damage(attack=10, defense=4) = 10 - 4/2 = 8
    let mut world = World::new();
    let actor = spawn_player(&mut world, (0, 0), 10, 5);
    spawn_enemy(&mut world, (5, 0), 4);

    let choices = available_player_actions(&mut world, actor);
    let damage = choices
        .iter()
        .find_map(|c| {
            if let PlayerActionChoice::Attack { damage, .. } = c {
                Some(*damage)
            } else {
                None
            }
        })
        .expect("expected attack choice");
    assert_eq!(damage, 8);
}

#[test]
fn no_attack_choices_when_ap_too_low() {
    let mut world = World::new();
    // Actor with only 1 AP — ATTACK_AP_COST is 2, so no attacks should appear.
    let actor = world
        .spawn((
            Health::new(100),
            Stats {
                max_hp: 100,
                attack: 10,
                defense: 5,
                speed: 10,
            },
            ActionPoints::new(1),
            Position::new(0, 0, 0),
        ))
        .id();
    spawn_enemy(&mut world, (5, 0), 4);

    let choices = available_player_actions(&mut world, actor);
    assert!(
        !choices
            .iter()
            .any(|c| matches!(c, PlayerActionChoice::Attack { .. }))
    );
}

#[test]
fn move_to_cover_option_listed_when_cover_available() {
    // Actor at (0,5); obstacle at (3,4) provides full cover from East (enemy at (9,5)).
    let mut world = World::new();
    let actor = spawn_player(&mut world, (0, 5), 10, 5);
    spawn_enemy(&mut world, (9, 5), 4);

    let mut map = LevelMap::new(10, 10, Biome::VoidStation);
    map.set(3, 4, Tile::Obstacle); // full cover at (3,5) from East
    map.recompute_cover();
    world.insert_resource(map);

    let choices = available_player_actions(&mut world, actor);
    let cover_moves: Vec<_> = choices
        .iter()
        .filter(|c| matches!(c, PlayerActionChoice::MoveToCover { .. }))
        .collect();
    assert!(
        !cover_moves.is_empty(),
        "expected at least one MoveToCover option"
    );
}

#[test]
fn no_move_to_cover_when_already_at_full_cover() {
    // Put obstacle directly East of actor so they already have full cover.
    let mut world = World::new();
    let actor = spawn_player(&mut world, (2, 5), 10, 5);
    spawn_enemy(&mut world, (9, 5), 4); // enemy to the East

    let mut map = LevelMap::new(10, 10, Biome::VoidStation);
    map.set(3, 5, Tile::Obstacle); // directly East of actor → full cover from East
    map.recompute_cover();
    world.insert_resource(map);

    let choices = available_player_actions(&mut world, actor);
    assert!(
        !choices
            .iter()
            .any(|c| matches!(c, PlayerActionChoice::MoveToCover { .. })),
        "already at full cover — no MoveToCover should be offered",
    );
}

#[test]
fn move_to_cover_ap_cost_is_correct() {
    let mut world = World::new();
    let actor = spawn_player(&mut world, (0, 5), 10, 5);
    spawn_enemy(&mut world, (9, 5), 4);

    let mut map = LevelMap::new(10, 10, Biome::VoidStation);
    map.set(3, 4, Tile::Obstacle); // cover for (3,5)
    map.recompute_cover();
    world.insert_resource(map);

    let choices = available_player_actions(&mut world, actor);
    for c in &choices {
        if let PlayerActionChoice::MoveToCover {
            destination,
            ap_cost,
            ..
        } = c
        {
            let dist = (destination.x - 0).abs() + (destination.y - 5).abs();
            assert_eq!(
                *ap_cost, dist,
                "ap_cost should equal Manhattan distance (MOVE_AP_COST=1)"
            );
        }
    }
}

#[test]
fn display_strings_are_non_empty() {
    let mut world = World::new();
    let actor = spawn_player(&mut world, (0, 0), 10, 5);
    spawn_enemy(&mut world, (5, 0), 4);

    let choices = available_player_actions(&mut world, actor);
    for choice in &choices {
        assert!(!choice.display().is_empty());
    }
}

// ── BattleStep player control ─────────────────────────────────────────────────

#[test]
fn player_choices_returns_options_on_player_turn() {
    let mut world = World::new();
    world.spawn((
        Character::new_player("Player", CharacterKind::Doss),
        Health::new(100),
        Stats {
            max_hp: 100,
            attack: 10,
            defense: 5,
            speed: 10,
        },
        ActionPoints::new(4),
        Position::new(0, 0, 0),
    ));
    world.spawn((
        Character::new_npc(CharacterKind::Scavenger, 1),
        Health::new(50),
        Stats {
            max_hp: 50,
            attack: 5,
            defense: 4,
            speed: 5,
        },
        ActionPoints::new(4),
        Position::new(5, 0, 0),
    ));

    let mut bs = BattleStep::new(&mut world);
    let choices = bs.player_choices(&mut world);
    assert!(!choices.is_empty());
    assert!(
        choices
            .iter()
            .any(|c| matches!(c, PlayerActionChoice::Attack { .. }))
    );
    assert!(
        choices
            .iter()
            .any(|c| matches!(c, PlayerActionChoice::Pass))
    );
}

#[test]
fn step_player_action_pass_ends_turn() {
    let mut world = World::new();
    let player = world
        .spawn((
            Character::new_player("Player", CharacterKind::Doss),
            Health::new(100),
            Stats {
                max_hp: 100,
                attack: 10,
                defense: 5,
                speed: 10,
            },
            ActionPoints::new(4),
            Position::new(0, 0, 0),
        ))
        .id();
    world.spawn((
        Character::new_npc(CharacterKind::Scavenger, 1),
        Health::new(50),
        Stats {
            max_hp: 50,
            attack: 5,
            defense: 4,
            speed: 5,
        },
        ActionPoints::new(4),
        Position::new(5, 0, 0),
    ));

    let mut bs = BattleStep::new(&mut world);
    bs.player_choices(&mut world); // initialise (refresh AP)

    let result = bs.step_player_action(&mut world, &PlayerActionChoice::Pass);
    assert_eq!(result.actor, player);
    assert!(result.turn_ended);
    assert!(result.action.is_none());
}

#[test]
fn step_player_action_attack_deals_damage() {
    let mut world = World::new();
    world.spawn((
        Character::new_player("Player", CharacterKind::Doss),
        Health::new(100),
        Stats {
            max_hp: 100,
            attack: 20,
            defense: 5,
            speed: 10,
        },
        ActionPoints::new(4),
        Position::new(0, 0, 0),
    ));
    let enemy = world
        .spawn((
            Character::new_npc(CharacterKind::Scavenger, 1),
            Health::new(50),
            Stats {
                max_hp: 50,
                attack: 5,
                defense: 0,
                speed: 5,
            },
            ActionPoints::new(4),
            Position::new(5, 0, 0),
        ))
        .id();

    let mut bs = BattleStep::new(&mut world);
    let choices = bs.player_choices(&mut world);

    // Pick the attack choice targeting the enemy.
    let attack_choice = choices
        .iter()
        .find(|c| matches!(c, PlayerActionChoice::Attack { .. }))
        .expect("expected an attack choice");

    bs.step_player_action(&mut world, attack_choice);

    // Enemy should have taken damage (no BattleRng → always hits).
    let hp = world.get::<Health>(enemy).unwrap().current;
    assert!(hp < 50, "enemy should have taken damage");
}

#[test]
fn step_player_action_outcome_set_on_victory() {
    let mut world = World::new();
    world.spawn((
        Character::new_player("Player", CharacterKind::Doss),
        Health::new(100),
        Stats {
            max_hp: 100,
            attack: 100,
            defense: 5,
            speed: 10,
        },
        ActionPoints::new(4),
        Position::new(0, 0, 0),
    ));
    world.spawn((
        Character::new_npc(CharacterKind::Scavenger, 1),
        Health::new(1),
        Stats {
            max_hp: 1,
            attack: 5,
            defense: 0,
            speed: 5,
        },
        ActionPoints::new(4),
        Position::new(5, 0, 0),
    ));

    let mut bs = BattleStep::new(&mut world);
    let choices = bs.player_choices(&mut world);
    let attack = choices
        .iter()
        .find(|c| matches!(c, PlayerActionChoice::Attack { .. }))
        .unwrap();

    let result = bs.step_player_action(&mut world, attack);
    assert_eq!(result.outcome, Some(BattleOutcome::PlayerVictory));
}
