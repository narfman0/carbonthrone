use bevy::prelude::*;
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::ability::character_abilities;
use crate::action_points::{ActionPoints, ap_for_speed};
use crate::character::Character;
use crate::combat::BattleStep;
use crate::game::{GamePhase, GameSession};
use crate::health::Health;
use crate::position::Position;
use crate::save::{BattleSnapshot, CombatantRole, CombatantSnapshot};
use crate::scripted_encounter::{ScriptedAlly, ScriptedFirstAction};
use crate::terrain::{BattleRng, LevelMap};

/// Restore a full battle state from a saved [`BattleSnapshot`].
///
/// Called from `from_save_data` when a snapshot is present.  The session must
/// already be in `GamePhase::Exploration` (setup_exploration already ran).
pub fn restore_battle(session: &mut GameSession, snapshot: &BattleSnapshot) {
    let GamePhase::Exploration(_) = &session.phase else {
        return;
    };
    let player_entity = match &session.phase {
        GamePhase::Exploration(e) => e.player_entity,
        _ => return,
    };
    let companion_entities: Vec<Entity> = match &session.phase {
        GamePhase::Exploration(e) => e.companion_entities.clone(),
        _ => return,
    };

    // Maps snapshot index → spawned/updated Entity.
    let mut entity_at_index: Vec<Option<Entity>> = vec![None; snapshot.combatants.len()];
    let mut companion_idx = 0usize;

    for (i, snap) in snapshot.combatants.iter().enumerate() {
        match snap.role {
            CombatantRole::Player => {
                update_combatant_from_snapshot(&mut session.world, player_entity, snap);
                entity_at_index[i] = Some(player_entity);
            }
            CombatantRole::Companion => {
                if let Some(&comp) = companion_entities.get(companion_idx) {
                    update_combatant_from_snapshot(&mut session.world, comp, snap);
                    entity_at_index[i] = Some(comp);
                    companion_idx += 1;
                }
            }
            CombatantRole::Enemy | CombatantRole::ScriptedAlly => {
                let entity = spawn_combatant_from_snapshot(&mut session.world, snap);
                entity_at_index[i] = Some(entity);
            }
        }
    }

    // Rebuild the LevelMap from saved tiles.
    let tiles: std::collections::HashMap<(i32, i32), crate::terrain::Tile> =
        snapshot.map_tiles.iter().copied().collect();
    session.world.insert_resource(LevelMap::from_tile_map(
        snapshot.map_cols,
        snapshot.map_rows,
        snapshot.zone_kind,
        tiles,
    ));
    session
        .world
        .insert_resource(BattleRng(StdRng::seed_from_u64(rand::random::<u64>())));

    // Reconstruct the actor queue from stable indices.
    let actor_queue: Vec<Entity> = snapshot
        .actor_queue
        .iter()
        .filter_map(|&idx| entity_at_index.get(idx).and_then(|e| *e))
        .collect();

    session.battle = Some(BattleStep::restore(
        snapshot.round,
        snapshot.turn,
        actor_queue,
    ));

    // Restore temporal systems.
    session
        .world
        .insert_resource(crate::temporal_flux::TemporalFlux {
            flux: snapshot.flux,
        });
    session
        .world
        .insert_resource(crate::pending_effects::PendingEffects::default());
    session
        .world
        .insert_resource(crate::pending_effects::CurrentRound(snapshot.round));
    // TODO: serialize pending_effects (pending delayed damage is dropped on load — MVP)

    // Transition phase to Battle.
    let GamePhase::Exploration(exploration) =
        std::mem::replace(&mut session.phase, GamePhase::Transitioning)
    else {
        return;
    };
    session.phase = GamePhase::Battle(exploration);
}

/// Update position, HP, and AP on an existing entity from a snapshot.
fn update_combatant_from_snapshot(world: &mut World, entity: Entity, snap: &CombatantSnapshot) {
    if let Some(mut pos) = world.get_mut::<Position>(entity) {
        pos.x = snap.x;
        pos.y = snap.y;
    }
    if let Some(mut h) = world.get_mut::<Health>(entity) {
        h.current = snap.current_hp;
    }
    if let Some(mut ap) = world.get_mut::<ActionPoints>(entity) {
        ap.current = snap.current_ap;
    }
}

/// Spawn a fresh enemy or scripted-ally entity from a snapshot.
fn spawn_combatant_from_snapshot(world: &mut World, snap: &CombatantSnapshot) -> Entity {
    let ch = Character::new_character(snap.kind.clone(), snap.level);
    let stats = ch.stats.clone();
    let max_hp = ch.stats.max_hp;
    let ap_max = ap_for_speed(ch.stats.speed);
    let mut cmd = world.spawn((
        ch,
        stats,
        Health {
            current: snap.current_hp,
            max: max_hp,
        },
        ActionPoints {
            current: snap.current_ap,
            max: ap_max,
        },
        Position::new(snap.x, snap.y),
    ));
    if matches!(snap.role, CombatantRole::ScriptedAlly) {
        cmd.insert(ScriptedAlly);
    }
    if let Some(ref name) = snap.scripted_first_action_name {
        if !snap.scripted_first_action_executed {
            if let Some(ability) = character_abilities(&snap.kind)
                .iter()
                .find(|a| a.name == name.as_str())
            {
                cmd.insert(ScriptedFirstAction {
                    ability_name: ability.name,
                    executed: false,
                });
            }
        }
    }
    cmd.id()
}
