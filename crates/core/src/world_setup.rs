use bevy::prelude::*;
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::action_points::{ActionPoints, ap_for_speed};
use crate::character::Character;
use crate::experience::Experience;
use crate::health::Health;
use crate::position::Position;
use crate::scripted_encounter::{
    PartyCompanion, ScriptedAlly, ScriptedEncounter, ScriptedFirstAction,
};
use crate::terrain::BattleRng;
use crate::zone::Zone;

/// Spawns the party into the exploration world. Returns the Researcher's entity
/// and a vec of companion entities (one per `party[1..]`).
pub fn setup_exploration(world: &mut World, party: &[Character]) -> (Entity, Vec<Entity>) {
    let ch = &party[0];
    let ap_max = ap_for_speed(ch.stats.speed);
    let player = world
        .spawn((
            ch.clone(),
            ch.stats.clone(),
            Health::new(ch.current_hp),
            ActionPoints::new(ap_max),
            Experience::new(),
            Position::new(0, 2),
        ))
        .id();

    let mut companions = vec![];
    for (i, companion) in party[1..].iter().enumerate() {
        let stats = companion.stats.clone();
        let ap_max = ap_for_speed(stats.speed);
        let pos = Position::new(i as i32 + 1, 2);
        let e = world
            .spawn((
                companion.clone(),
                stats,
                Health::new(companion.current_hp),
                ActionPoints::new(ap_max),
                Experience::new(),
                pos,
                PartyCompanion,
            ))
            .id();
        companions.push(e);
    }
    (player, companions)
}

/// Adds enemies and battle resources to the world. Party is already present from
/// `setup_exploration`.
///
/// When `script` is `Some`, enemy positions are taken from the scripted
/// encounter definition instead of being generated randomly.  Scripted allies
/// are spawned with the [`ScriptedAlly`] marker component so they are
/// despawned at the end of combat.
pub fn setup_battle(world: &mut World, zone: &Zone, script: Option<&ScriptedEncounter>) {
    let mut rng = StdRng::seed_from_u64(rand::random::<u64>());

    // Spawn enemies — either from the script or generated randomly.
    if let Some(s) = script {
        for placement in &s.enemies {
            let (character, raw_pos) = placement.to_character_and_pos(zone.cols, zone.rows);
            let (sx, sy) = zone.map.nearest_open_tile(raw_pos.x, raw_pos.y);
            let stats = character.stats.clone();
            let hp = character.current_hp;
            let ap_max = ap_for_speed(stats.speed);
            let mut entity_cmd = world.spawn((
                character,
                stats,
                Health::new(hp),
                ActionPoints::new(ap_max),
                Position::new(sx, sy),
            ));
            if let Some(ability_name) = placement.first_ability {
                entity_cmd.insert(ScriptedFirstAction {
                    ability_name,
                    executed: false,
                });
            }
        }
    } else {
        for (character, pos) in zone.generate_enemies(&mut rng) {
            let stats = character.stats.clone();
            let hp = character.current_hp;
            let ap_max = ap_for_speed(stats.speed);
            world.spawn((
                character,
                stats,
                Health::new(hp),
                ActionPoints::new(ap_max),
                pos,
            ));
        }
    }

    // Spawn scripted allies (temporary, fight on the player's side).
    if let Some(s) = script {
        for placement in &s.allies {
            let (character, raw_pos) = placement.to_character_and_pos(zone.cols, zone.rows);
            let (sx, sy) = zone.map.nearest_open_tile(raw_pos.x, raw_pos.y);
            let stats = character.stats.clone();
            let hp = character.current_hp;
            let ap_max = ap_for_speed(stats.speed);
            let mut entity_cmd = world.spawn((
                character,
                stats,
                Health::new(hp),
                ActionPoints::new(ap_max),
                Position::new(sx, sy),
                ScriptedAlly,
            ));
            if let Some(ability_name) = placement.first_ability {
                entity_cmd.insert(ScriptedFirstAction {
                    ability_name,
                    executed: false,
                });
            }
        }
    }

    let battle_rng = StdRng::seed_from_u64(rand::random::<u64>());
    world.insert_resource(zone.map.clone());
    world.insert_resource(BattleRng(battle_rng));
}
