use bevy::prelude::*;

use crate::{
    action_points::ActionPoints,
    health::Health,
    side::Side,
    stats::Stats,
    turn::{Action, apply_action, ATTACK_AP_COST},
};

/// Hard cap on rounds to prevent infinite loops.
pub const MAX_ROUNDS: u32 = 1000;

#[derive(Debug, Clone, PartialEq)]
pub enum BattleOutcome {
    PlayerVictory,
    PlayerDefeated,
    /// Neither side was eliminated within `MAX_ROUNDS`.
    Draw,
}

/// Run a battle to completion. Each round the player side acts first, then the
/// enemy side. Within a side, combatants act in descending speed order. Each
/// combatant's AP is refreshed at the start of their turn and spent on actions
/// until exhausted or they choose to pass.
///
/// All combatants must carry `Side`, `Health`, `Stats`, and `ActionPoints`.
pub fn simulate_battle(world: &mut World) -> BattleOutcome {
    for _ in 0..MAX_ROUNDS {
        run_side(world, Side::Player);
        if all_defeated(world, Side::Enemy) {
            return BattleOutcome::PlayerVictory;
        }
        if all_defeated(world, Side::Player) {
            return BattleOutcome::PlayerDefeated;
        }

        run_side(world, Side::Enemy);
        if all_defeated(world, Side::Player) {
            return BattleOutcome::PlayerDefeated;
        }
        if all_defeated(world, Side::Enemy) {
            return BattleOutcome::PlayerVictory;
        }
    }
    BattleOutcome::Draw
}

fn run_side(world: &mut World, side: Side) {
    for actor in living_side(world, side) {
        refresh_actor(world, actor);
        loop {
            match choose_action(world, actor, side) {
                Some(Action::Pass) | None => break,
                Some(action) => {
                    apply_action(world, actor, &action);
                }
            }
        }
    }
}

fn refresh_actor(world: &mut World, actor: Entity) {
    if let Some(mut ap) = world.get_mut::<ActionPoints>(actor) {
        ap.refresh();
    }
}

/// Returns living entities on `side`, sorted by descending speed.
fn living_side(world: &mut World, side: Side) -> Vec<Entity> {
    let mut query = world.query::<(Entity, &Side, &Health, &Stats)>();
    let mut entities: Vec<(Entity, i32)> = query
        .iter(world)
        .filter(|(_, s, h, _)| **s == side && h.is_alive())
        .map(|(e, _, _, stats)| (e, stats.speed))
        .collect();
    entities.sort_by(|a, b| b.1.cmp(&a.1));
    entities.into_iter().map(|(e, _)| e).collect()
}

/// `true` if every entity on `side` is dead, or no such entities exist.
fn all_defeated(world: &mut World, side: Side) -> bool {
    let mut query = world.query::<(&Side, &Health)>();
    let combatants: Vec<bool> = query
        .iter(world)
        .filter(|(s, _)| **s == side)
        .map(|(_, h)| h.is_alive())
        .collect();
    combatants.is_empty() || combatants.iter().all(|alive| !alive)
}

/// Simple AI: attack the first living opponent, or pass if out of AP / no targets.
fn choose_action(world: &mut World, actor: Entity, actor_side: Side) -> Option<Action> {
    let ap = world.get::<ActionPoints>(actor)?.current;
    if ap < ATTACK_AP_COST {
        return Some(Action::Pass);
    }
    let opponent_side = match actor_side {
        Side::Player => Side::Enemy,
        Side::Enemy => Side::Player,
    };
    let mut query = world.query::<(Entity, &Side, &Health)>();
    let target = query
        .iter(world)
        .find(|(_, s, h)| **s == opponent_side && h.is_alive())
        .map(|(e, _, _)| e);
    Some(match target {
        Some(t) => Action::Attack { target: t },
        None => Action::Pass,
    })
}
