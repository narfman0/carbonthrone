use bevy::prelude::*;
use carbonthrone::{
    action_points::ActionPoints,
    character::{Character, CharacterKind},
    combat::{BattleOutcome, BattleStep, simulate_battle},
    health::Health,
    position::Position,
    scripted_encounter::{
        CombatantPlacement, ScriptedAlly, ScriptedEncounter, ScriptedFirstAction, SpawnLocation,
        scripted_encounter_for,
    },
    stats::Stats,
    zone::ZoneKind,
};

// ── SpawnLocation ─────────────────────────────────────────────────────────────

#[test]
fn spawn_location_north_wall_is_near_top() {
    let (x, y) = SpawnLocation::NorthWall.to_grid(12, 10);
    assert_eq!(x, 6); // cols / 2
    assert!(y <= 2, "north wall should be near top (y={y})");
}

#[test]
fn spawn_location_south_wall_is_near_bottom() {
    let (x, y) = SpawnLocation::SouthWall.to_grid(12, 10);
    assert_eq!(x, 6);
    assert!(y >= 7, "south wall should be near bottom (y={y})");
}

#[test]
fn spawn_location_east_wall_is_near_right() {
    let (x, y) = SpawnLocation::EastWall.to_grid(12, 10);
    assert!(x >= 9, "east wall should be near right (x={x})");
    assert_eq!(y, 5); // rows / 2
}

#[test]
fn spawn_location_west_wall_is_near_left() {
    let (x, y) = SpawnLocation::WestWall.to_grid(12, 10);
    assert!(x <= 2, "west wall should be near left (x={x})");
    assert_eq!(y, 5);
}

#[test]
fn spawn_location_center_is_middle() {
    let (x, y) = SpawnLocation::Center.to_grid(12, 10);
    assert_eq!(x, 6);
    assert_eq!(y, 5);
}

#[test]
fn spawn_location_fraction_maps_correctly() {
    let (x, y) = SpawnLocation::Fraction(0.5, 0.1).to_grid(10, 10);
    assert_eq!(x, 5);
    assert_eq!(y, 1);
}

#[test]
fn spawn_location_fraction_clamps_to_bounds() {
    // Values beyond 1.0 should clamp to valid grid indices.
    let (x, y) = SpawnLocation::Fraction(2.0, 2.0).to_grid(8, 8);
    assert!(x < 8, "x should be clamped (got {x})");
    assert!(y < 8, "y should be clamped (got {y})");
}

// ── CombatantPlacement ────────────────────────────────────────────────────────

#[test]
fn combatant_placement_creates_character_at_location() {
    let placement = CombatantPlacement {
        kind: CharacterKind::Zealot,
        level: 1,
        location: SpawnLocation::NorthWall,
        aggression: None,
        first_ability: None,
    };
    let (ch, pos) = placement.to_character_and_pos(12, 10);
    assert_eq!(ch.kind, CharacterKind::Zealot);
    assert_eq!(ch.level, 1);
    assert_eq!(pos.x, 6); // cols / 2
    assert!(pos.y <= 2, "position should be near north wall");
}

#[test]
fn combatant_placement_overrides_aggression() {
    use carbonthrone::character::Aggression;
    let placement = CombatantPlacement {
        kind: CharacterKind::Zealot,
        level: 1,
        location: SpawnLocation::Center,
        aggression: Some(Aggression::Friendly),
        first_ability: None,
    };
    let (ch, _) = placement.to_character_and_pos(10, 10);
    assert_eq!(ch.aggression, Aggression::Friendly);
}

// ── scripted_encounter_for ────────────────────────────────────────────────────

#[test]
fn tutorial_encounter_returned_for_research_wing_loop1() {
    let enc = scripted_encounter_for(ZoneKind::ResearchWing, 1);
    assert!(enc.is_some(), "should have a scripted encounter");
    let enc = enc.unwrap();
    assert_eq!(enc.id, "tutorial_opening");
}

#[test]
fn tutorial_encounter_has_three_enemies_and_two_allies() {
    let enc = scripted_encounter_for(ZoneKind::ResearchWing, 1).unwrap();
    assert_eq!(enc.enemies.len(), 3, "tutorial should have 3 enemies");
    assert_eq!(enc.allies.len(), 2, "tutorial should have 2 allies");
}

#[test]
fn tutorial_enemies_are_constancy_faction() {
    let enc = scripted_encounter_for(ZoneKind::ResearchWing, 1).unwrap();
    let kinds: Vec<&CharacterKind> = enc.enemies.iter().map(|p| &p.kind).collect();
    assert!(
        kinds.contains(&&CharacterKind::Zealot),
        "tutorial should have Zealots"
    );
    assert!(
        kinds.contains(&&CharacterKind::Purifier),
        "tutorial should have a Purifier"
    );
}

#[test]
fn tutorial_allies_are_orin_and_doss() {
    let enc = scripted_encounter_for(ZoneKind::ResearchWing, 1).unwrap();
    let ally_kinds: Vec<&CharacterKind> = enc.allies.iter().map(|p| &p.kind).collect();
    assert!(
        ally_kinds.contains(&&CharacterKind::Orin),
        "tutorial allies should include Orin"
    );
    assert!(
        ally_kinds.contains(&&CharacterKind::Doss),
        "tutorial allies should include Doss"
    );
}

#[test]
fn tutorial_enemies_spawn_on_opposite_side_from_allies() {
    let enc = scripted_encounter_for(ZoneKind::ResearchWing, 1).unwrap();
    let cols = 12u32;
    let rows = 10u32;
    let enemy_y_avg: f32 = enc
        .enemies
        .iter()
        .map(|p| p.location.to_grid(cols, rows).1 as f32)
        .sum::<f32>()
        / enc.enemies.len() as f32;
    let ally_y_avg: f32 = enc
        .allies
        .iter()
        .map(|p| p.location.to_grid(cols, rows).1 as f32)
        .sum::<f32>()
        / enc.allies.len() as f32;
    assert!(
        enemy_y_avg < ally_y_avg,
        "enemies ({enemy_y_avg:.1}) should be on opposite end from allies ({ally_y_avg:.1})"
    );
}

#[test]
fn archon_encounter_returned_for_military_annex_loop4() {
    let enc = scripted_encounter_for(ZoneKind::MilitaryAnnex, 4);
    assert!(enc.is_some(), "should have a scripted encounter in loop 4");
    let enc = enc.unwrap();
    assert_eq!(enc.id, "archon_encounter");
}

#[test]
fn archon_encounter_returned_for_loop5_as_well() {
    let enc = scripted_encounter_for(ZoneKind::MilitaryAnnex, 5);
    assert!(enc.is_some());
    assert_eq!(enc.unwrap().id, "archon_encounter");
}

#[test]
fn archon_encounter_has_archon_as_enemy() {
    let enc = scripted_encounter_for(ZoneKind::MilitaryAnnex, 4).unwrap();
    assert!(
        enc.enemies.iter().any(|p| p.kind == CharacterKind::Archon),
        "archon encounter should include an Archon"
    );
}

#[test]
fn archon_encounter_has_no_allies() {
    let enc = scripted_encounter_for(ZoneKind::MilitaryAnnex, 4).unwrap();
    assert!(
        enc.allies.is_empty(),
        "archon encounter has no scripted allies"
    );
}

#[test]
fn research_wing_loop2_returns_none() {
    // After loop 1 the tutorial is done — encounters revert to random.
    assert!(scripted_encounter_for(ZoneKind::ResearchWing, 2).is_none());
}

#[test]
fn military_annex_loop3_returns_none() {
    // Archon only appears in loop 4+.
    assert!(scripted_encounter_for(ZoneKind::MilitaryAnnex, 3).is_none());
}

#[test]
fn hallway_returns_none() {
    assert!(scripted_encounter_for(ZoneKind::Hallway, 1).is_none());
}

// ── ScriptedFirstAction integration ──────────────────────────────────────────

/// Verify that a `ScriptedFirstAction` component causes the AI to use the
/// named ability on its first combat turn.
#[test]
fn scripted_first_action_is_executed_on_first_turn() {
    let mut world = World::new();

    // Spawn a Researcher with a scripted "Temporal Bolt" first action.
    let player = world
        .spawn((
            Character::new_character(CharacterKind::Researcher, 1),
            Health::new(75),
            Stats {
                max_hp: 75,
                attack: 20,
                defense: 4,
                speed: 16,
            },
            ActionPoints::new(4),
            Position::new(0, 5),
            ScriptedFirstAction {
                ability_name: "Temporal Bolt",
                executed: false,
            },
        ))
        .id();

    // Spawn an enemy directly adjacent so all ability kinds can reach it.
    world.spawn((
        Character::new_character(CharacterKind::Scavenger, 1),
        Health::new(40),
        Stats {
            max_hp: 40,
            attack: 5,
            defense: 2,
            speed: 5,
        },
        ActionPoints::new(4),
        Position::new(0, 6),
    ));

    // Run one full battle step to let the scripted action fire.
    let mut battle = BattleStep::new(&mut world);
    let event = battle.step(&mut world);

    // The player's turn should have produced at least one action.
    assert!(
        !event.actions.is_empty(),
        "scripted actor should have acted"
    );

    // The scripted first action should now be marked executed.
    let sfa = world.get::<ScriptedFirstAction>(player).unwrap();
    assert!(
        sfa.executed,
        "ScriptedFirstAction should be marked executed after first turn"
    );
}

// ── ScriptedAlly despawn ─────────────────────────────────────────────────────

/// Scripted allies should carry the ScriptedAlly marker component.
#[test]
fn scripted_allies_have_marker_component() {
    let mut world = World::new();
    let ally_entity = world
        .spawn((
            Character::new_character(CharacterKind::Orin, 1),
            Health::new(90),
            Stats {
                max_hp: 90,
                attack: 8,
                defense: 8,
                speed: 9,
            },
            ActionPoints::new(4),
            Position::new(5, 8),
            ScriptedAlly,
        ))
        .id();

    assert!(
        world.get::<ScriptedAlly>(ally_entity).is_some(),
        "scripted ally should have ScriptedAlly marker"
    );
}

// ── BattleStep::current_actor_is_scripted_ally ───────────────────────────────

#[test]
fn current_actor_is_scripted_ally_returns_false_for_normal_player() {
    let mut world = World::new();
    world.spawn((
        Character::new_character(CharacterKind::Researcher, 1),
        Health::new(75),
        Stats {
            max_hp: 75,
            attack: 10,
            defense: 4,
            speed: 10,
        },
        ActionPoints::new(4),
        Position::new(0, 0),
    ));

    let battle = BattleStep::new(&mut world);
    assert!(
        !battle.current_actor_is_scripted_ally(&world),
        "regular player should not be considered a scripted ally"
    );
}

#[test]
fn current_actor_is_scripted_ally_returns_true_for_tagged_ally() {
    let mut world = World::new();
    // Spawn a scripted ally with a higher speed so it is queued first.
    world.spawn((
        Character::new_character(CharacterKind::Orin, 1),
        Health::new(90),
        Stats {
            max_hp: 90,
            attack: 8,
            defense: 8,
            speed: 20, // fast — goes first
        },
        ActionPoints::new(4),
        Position::new(0, 0),
        ScriptedAlly,
    ));
    world.spawn((
        Character::new_character(CharacterKind::Researcher, 1),
        Health::new(75),
        Stats {
            max_hp: 75,
            attack: 10,
            defense: 4,
            speed: 10,
        },
        ActionPoints::new(4),
        Position::new(1, 0),
    ));

    let battle = BattleStep::new(&mut world);
    assert!(
        battle.current_actor_is_scripted_ally(&world),
        "scripted ally (fast Orin) should be detected as scripted"
    );
}

// ── End-to-end: scripted encounter simulation ─────────────────────────────────

/// Verify that a scripted encounter's enemies and allies survive to be spawned
/// and that a battle can be simulated to completion.
#[test]
fn scripted_tutorial_encounter_can_be_simulated() {
    use carbonthrone::character::Aggression;
    let mut world = World::new();

    // Player character.
    world.spawn((
        Character::new_character(CharacterKind::Researcher, 5),
        Health::new(110),
        Stats {
            max_hp: 110,
            attack: 18,
            defense: 9,
            speed: 20,
        },
        ActionPoints::new(4),
        Position::new(6, 8),
    ));

    let enc = scripted_encounter_for(ZoneKind::ResearchWing, 1).unwrap();
    let cols = 12u32;
    let rows = 10u32;

    // Spawn enemies from the scripted encounter.
    for placement in &enc.enemies {
        let (character, pos) = placement.to_character_and_pos(cols, rows);
        let stats = character.stats.clone();
        let hp = character.current_hp;
        let mut ecmd = world.spawn((character, stats, Health::new(hp), ActionPoints::new(4), pos));
        if let Some(name) = placement.first_ability {
            ecmd.insert(ScriptedFirstAction {
                ability_name: name,
                executed: false,
            });
        }
    }

    // Spawn allies from the scripted encounter.
    for placement in &enc.allies {
        let (mut character, pos) = placement.to_character_and_pos(cols, rows);
        character.aggression = Aggression::Friendly;
        let stats = character.stats.clone();
        let hp = character.current_hp;
        let mut acmd = world.spawn((
            character,
            stats,
            Health::new(hp),
            ActionPoints::new(4),
            pos,
            ScriptedAlly,
        ));
        if let Some(name) = placement.first_ability {
            acmd.insert(ScriptedFirstAction {
                ability_name: name,
                executed: false,
            });
        }
    }

    // The battle should reach an outcome (not run forever).
    let outcome = simulate_battle(&mut world);
    assert!(
        matches!(
            outcome,
            BattleOutcome::PlayerVictory | BattleOutcome::PlayerDefeated | BattleOutcome::Draw
        ),
        "battle should reach a definite outcome"
    );
}
