use bevy::prelude::*;
use carbonthrone::{
    action_points::ActionPoints,
    character::{Character, CharacterKind},
    combat::BASE_HIT_CHANCE,
    combat::{BattleOutcome, BattleStep},
    health::Health,
    player_input::{PlayerActionChoice, available_player_actions},
    position::Position,
    stats::Stats,
    terrain::{CoverLevel, LevelMap, Tile},
    zone::ZoneKind,
};

// ── Helpers ────────────────────────────────────────────────────────────────────

/// Spawns a Kaleo player (ranged abilities → no adjacency requirement for attack tests).
fn spawn_player(world: &mut World, pos: (i32, i32), attack: i32, defense: i32) -> Entity {
    let ch = Character::new_character(CharacterKind::Kaleo, 1);
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
            Position::new(pos.0, pos.1),
        ))
        .id()
}

fn spawn_enemy(world: &mut World, pos: (i32, i32), defense: i32) -> Entity {
    let ch = Character::new_character(CharacterKind::Scavenger, 1);
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
            Position::new(pos.0, pos.1),
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
fn ability_choice_listed_for_each_living_enemy() {
    let mut world = World::new();
    let actor = spawn_player(&mut world, (0, 0), 10, 5);
    spawn_enemy(&mut world, (5, 0), 4);
    spawn_enemy(&mut world, (0, 5), 2);

    let choices = available_player_actions(&mut world, actor);
    // Kaleo at level 1 has one ranged ability (Aimed Shot): one UseAbility per enemy.
    let ability_choices: Vec<_> = choices
        .iter()
        .filter(|c| {
            matches!(
                c,
                PlayerActionChoice::UseAbility {
                    target: Some(_),
                    ..
                }
            )
        })
        .collect();
    assert_eq!(
        ability_choices.len(),
        2,
        "one ability choice per living enemy"
    );
}

#[test]
fn ability_choice_has_correct_hit_chance_no_cover() {
    let mut world = World::new();
    let actor = spawn_player(&mut world, (0, 0), 10, 5);
    spawn_enemy(&mut world, (5, 0), 4);

    let choices = available_player_actions(&mut world, actor);
    for choice in &choices {
        if let PlayerActionChoice::UseAbility {
            hit_chance: Some(hc),
            cover: Some(cover),
            ..
        } = choice
        {
            assert_eq!(*cover, CoverLevel::None);
            assert!(
                (hc - BASE_HIT_CHANCE).abs() < 0.001,
                "no-cover hit chance should be {BASE_HIT_CHANCE}"
            );
        }
    }
}

#[test]
fn ability_choice_reflects_partial_cover() {
    // Target at (5,5); obstacle diagonally from North-West → partial cover from West.
    let mut world = World::new();
    let actor = spawn_player(&mut world, (0, 5), 10, 5); // attacker approaches from West
    spawn_enemy(&mut world, (5, 5), 4);

    let mut map = LevelMap::new(10, 10, ZoneKind::CommandDeck);
    map.set(4, 4, Tile::Obstacle);
    map.recompute_cover();
    world.insert_resource(map);

    let choices = available_player_actions(&mut world, actor);
    let ability_opt = choices.iter().find_map(|c| {
        if let PlayerActionChoice::UseAbility {
            hit_chance: Some(hc),
            cover: Some(cover),
            ..
        } = c
        {
            Some((*hc, *cover))
        } else {
            None
        }
    });
    let (hit_chance, cover) = ability_opt.expect("expected an ability choice with cover info");
    assert_eq!(cover, CoverLevel::Partial);
    assert!(
        (hit_chance - 0.65).abs() < 0.001,
        "partial cover → 65% hit chance"
    );
}

#[test]
fn ability_choice_reflects_full_cover() {
    // Target at (5,5); obstacle directly north at (5,4) — Full cover from North.
    let mut world = World::new();
    let actor = spawn_player(&mut world, (5, 0), 10, 5); // approaches from North
    spawn_enemy(&mut world, (5, 5), 4);

    let mut map = LevelMap::new(10, 10, ZoneKind::CommandDeck);
    map.set(5, 4, Tile::Obstacle);
    map.recompute_cover();
    world.insert_resource(map);

    let choices = available_player_actions(&mut world, actor);
    let (hit_chance, cover) = choices
        .iter()
        .find_map(|c| {
            if let PlayerActionChoice::UseAbility {
                hit_chance: Some(hc),
                cover: Some(cover),
                ..
            } = c
            {
                Some((*hc, *cover))
            } else {
                None
            }
        })
        .expect("expected an ability choice with cover info");

    assert_eq!(cover, CoverLevel::Full);
    assert!(
        (hit_chance - 0.35).abs() < 0.001,
        "full cover → 35% hit chance"
    );
}

#[test]
fn ability_choice_reflects_correct_damage() {
    // Kaleo Aimed Shot: calc_damage(attack=10, defense=4) + bonus 5 = 8 + 5 = 13
    let mut world = World::new();
    let actor = spawn_player(&mut world, (0, 0), 10, 5);
    spawn_enemy(&mut world, (5, 0), 4);

    let choices = available_player_actions(&mut world, actor);
    let damage = choices
        .iter()
        .find_map(|c| {
            if let PlayerActionChoice::UseAbility {
                damage: Some(dmg), ..
            } = c
            {
                Some(*dmg)
            } else {
                None
            }
        })
        .expect("expected ability choice with damage");
    assert_eq!(damage, 13); // calc_damage(10,4)=8, +5 bonus = 13
}

#[test]
fn no_ability_choices_when_no_character_component() {
    let mut world = World::new();
    // Actor without a Character component → no abilities offered.
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
            Position::new(0, 0),
        ))
        .id();
    spawn_enemy(&mut world, (5, 0), 4);

    let choices = available_player_actions(&mut world, actor);
    assert!(
        !choices
            .iter()
            .any(|c| matches!(c, PlayerActionChoice::UseAbility { .. }))
    );
}

#[test]
fn melee_ability_not_offered_when_target_not_adjacent() {
    // Doss has melee abilities only; enemy is far away → no offensive UseAbility choices.
    let mut world = World::new();
    let ch = Character::new_character(CharacterKind::Doss, 1);
    let actor = world
        .spawn((
            ch,
            Health::new(100),
            Stats {
                max_hp: 100,
                attack: 10,
                defense: 5,
                speed: 10,
            },
            ActionPoints::new(4),
            Position::new(0, 0),
        ))
        .id();
    spawn_enemy(&mut world, (5, 0), 4); // not adjacent

    let choices = available_player_actions(&mut world, actor);
    let offensive: Vec<_> = choices
        .iter()
        .filter(|c| {
            matches!(
                c,
                PlayerActionChoice::UseAbility {
                    target: Some(_),
                    damage: Some(_),
                    ..
                }
            )
        })
        .collect();
    assert!(
        offensive.is_empty(),
        "melee abilities should not be offered for non-adjacent targets"
    );
}

#[test]
fn melee_ability_offered_when_target_adjacent() {
    // Doss has melee abilities; enemy is adjacent (distance 1) → UseAbility offered.
    let mut world = World::new();
    let ch = Character::new_character(CharacterKind::Doss, 1);
    let actor = world
        .spawn((
            ch,
            Health::new(100),
            Stats {
                max_hp: 100,
                attack: 10,
                defense: 5,
                speed: 10,
            },
            ActionPoints::new(4),
            Position::new(0, 0),
        ))
        .id();
    spawn_enemy(&mut world, (1, 0), 4); // adjacent

    let choices = available_player_actions(&mut world, actor);
    let offensive: Vec<_> = choices
        .iter()
        .filter(|c| {
            matches!(
                c,
                PlayerActionChoice::UseAbility {
                    target: Some(_),
                    damage: Some(_),
                    ..
                }
            )
        })
        .collect();
    assert!(
        !offensive.is_empty(),
        "melee abilities should be offered for adjacent targets"
    );
}

#[test]
fn melee_ability_offered_for_diagonal_adjacency() {
    // Diagonal adjacency: Chebyshev distance = max(1,1) = 1.
    let mut world = World::new();
    let ch = Character::new_character(CharacterKind::Doss, 1);
    let actor = world
        .spawn((
            ch,
            Health::new(100),
            Stats {
                max_hp: 100,
                attack: 10,
                defense: 5,
                speed: 10,
            },
            ActionPoints::new(4),
            Position::new(0, 0),
        ))
        .id();
    spawn_enemy(&mut world, (1, 1), 4); // diagonal

    let choices = available_player_actions(&mut world, actor);
    let offensive: Vec<_> = choices
        .iter()
        .filter(|c| {
            matches!(
                c,
                PlayerActionChoice::UseAbility {
                    target: Some(_),
                    damage: Some(_),
                    ..
                }
            )
        })
        .collect();
    assert!(
        !offensive.is_empty(),
        "melee abilities should allow diagonal adjacency"
    );
}

#[test]
fn move_to_cover_option_listed_when_cover_available() {
    // Actor at (0,5); obstacle at (3,4) provides full cover from East (enemy at (9,5)).
    let mut world = World::new();
    let actor = spawn_player(&mut world, (0, 5), 10, 5);
    spawn_enemy(&mut world, (9, 5), 4);

    let mut map = LevelMap::new(10, 10, ZoneKind::CommandDeck);
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

    let mut map = LevelMap::new(10, 10, ZoneKind::CommandDeck);
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

    let mut map = LevelMap::new(10, 10, ZoneKind::CommandDeck);
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
        Character::new_character(CharacterKind::Kaleo, 1),
        Health::new(100),
        Stats {
            max_hp: 100,
            attack: 10,
            defense: 5,
            speed: 10,
        },
        ActionPoints::new(4),
        Position::new(0, 0),
    ));
    world.spawn((
        Character::new_character(CharacterKind::Scavenger, 1),
        Health::new(50),
        Stats {
            max_hp: 50,
            attack: 5,
            defense: 4,
            speed: 5,
        },
        ActionPoints::new(4),
        Position::new(5, 0),
    ));

    let mut bs = BattleStep::new(&mut world);
    let choices = bs.player_choices(&mut world);
    assert!(!choices.is_empty());
    assert!(
        choices
            .iter()
            .any(|c| matches!(c, PlayerActionChoice::UseAbility { .. }))
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
            Character::new_character(CharacterKind::Kaleo, 1),
            Health::new(100),
            Stats {
                max_hp: 100,
                attack: 10,
                defense: 5,
                speed: 10,
            },
            ActionPoints::new(4),
            Position::new(0, 0),
        ))
        .id();
    world.spawn((
        Character::new_character(CharacterKind::Scavenger, 1),
        Health::new(50),
        Stats {
            max_hp: 50,
            attack: 5,
            defense: 4,
            speed: 5,
        },
        ActionPoints::new(4),
        Position::new(5, 0),
    ));

    let mut bs = BattleStep::new(&mut world);
    bs.player_choices(&mut world); // initialise (refresh AP)

    let result = bs.step_player_action(&mut world, &PlayerActionChoice::Pass);
    assert_eq!(result.actor, player);
    assert!(result.turn_ended);
    assert!(result.action.is_none());
}

#[test]
fn step_player_action_ability_deals_damage() {
    let mut world = World::new();
    world.spawn((
        Character::new_character(CharacterKind::Kaleo, 1),
        Health::new(100),
        Stats {
            max_hp: 100,
            attack: 20,
            defense: 5,
            speed: 10,
        },
        ActionPoints::new(4),
        Position::new(0, 0),
    ));
    let enemy = world
        .spawn((
            Character::new_character(CharacterKind::Scavenger, 1),
            Health::new(50),
            Stats {
                max_hp: 50,
                attack: 5,
                defense: 0,
                speed: 5,
            },
            ActionPoints::new(4),
            Position::new(5, 0),
        ))
        .id();

    let mut bs = BattleStep::new(&mut world);
    let choices = bs.player_choices(&mut world);

    let ability_choice = choices
        .iter()
        .find(|c| {
            matches!(
                c,
                PlayerActionChoice::UseAbility {
                    target: Some(_),
                    ..
                }
            )
        })
        .expect("expected a UseAbility choice");

    bs.step_player_action(&mut world, ability_choice);

    let hp = world.get::<Health>(enemy).unwrap().current;
    assert!(hp < 50, "enemy should have taken damage");
}

#[test]
fn step_player_action_outcome_set_on_victory() {
    let mut world = World::new();
    world.spawn((
        Character::new_character(CharacterKind::Kaleo, 1),
        Health::new(100),
        Stats {
            max_hp: 100,
            attack: 100,
            defense: 5,
            speed: 10,
        },
        ActionPoints::new(4),
        Position::new(0, 0),
    ));
    world.spawn((
        Character::new_character(CharacterKind::Scavenger, 1),
        Health::new(1),
        Stats {
            max_hp: 1,
            attack: 5,
            defense: 0,
            speed: 5,
        },
        ActionPoints::new(4),
        Position::new(5, 0),
    ));

    let mut bs = BattleStep::new(&mut world);
    let choices = bs.player_choices(&mut world);
    let ability_choice = choices
        .iter()
        .find(|c| {
            matches!(
                c,
                PlayerActionChoice::UseAbility {
                    target: Some(_),
                    ..
                }
            )
        })
        .unwrap();

    let result = bs.step_player_action(&mut world, ability_choice);
    assert_eq!(result.outcome, Some(BattleOutcome::PlayerVictory));
}
