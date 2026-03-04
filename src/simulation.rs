use std::collections::VecDeque;

use bevy::prelude::*;

use crate::{
    action_points::ActionPoints,
    combat::calc_damage,
    health::Health,
    position::Position,
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

// ── Step-based simulation ────────────────────────────────────────────────────

/// One action that occurred during a combatant's turn.
#[derive(Debug, Clone)]
pub enum TurnAction {
    Attack { target: Entity, damage: i32 },
    Move { to: Position },
}

/// Result returned by `BattleStep::step()`.
#[derive(Debug)]
pub struct TurnEvent {
    /// The entity that just acted (`None` if the step only changed sides/round).
    pub actor: Option<Entity>,
    pub side: Side,
    pub actions: Vec<TurnAction>,
    /// Set when the battle has ended after this step.
    pub outcome: Option<BattleOutcome>,
}

/// Incremental battle driver: call `step()` once per key-press.
/// All combatants must carry `Side`, `Health`, `Stats`, and `ActionPoints`.
pub struct BattleStep {
    pub round: u32,
    pub side: Side,
    actor_queue: VecDeque<Entity>,
}

impl BattleStep {
    pub fn new(world: &mut World) -> Self {
        let queue = VecDeque::from(living_side(world, Side::Player));
        Self { round: 1, side: Side::Player, actor_queue: queue }
    }

    /// The next entity waiting to act this side, if any.
    pub fn next_actor(&self) -> Option<Entity> {
        self.actor_queue.front().copied()
    }

    /// Advance one actor's full turn (all AP spent). Returns what happened.
    pub fn step(&mut self, world: &mut World) -> TurnEvent {
        if let Some(outcome) = check_outcome(world) {
            return TurnEvent { actor: None, side: self.side, actions: vec![], outcome: Some(outcome) };
        }

        // Refill queue when the current side is exhausted.
        if self.actor_queue.is_empty() {
            match self.side {
                Side::Player => {
                    self.side = Side::Enemy;
                    self.actor_queue = VecDeque::from(living_side(world, Side::Enemy));
                }
                Side::Enemy => {
                    self.round += 1;
                    if self.round > MAX_ROUNDS {
                        return TurnEvent {
                            actor: None,
                            side: self.side,
                            actions: vec![],
                            outcome: Some(BattleOutcome::Draw),
                        };
                    }
                    self.side = Side::Player;
                    self.actor_queue = VecDeque::from(living_side(world, Side::Player));
                }
            }
            // Re-check after switching (e.g. all enemies already dead).
            if let Some(outcome) = check_outcome(world) {
                return TurnEvent { actor: None, side: self.side, actions: vec![], outcome: Some(outcome) };
            }
        }

        let Some(actor) = self.actor_queue.pop_front() else {
            return TurnEvent {
                actor: None,
                side: self.side,
                actions: vec![],
                outcome: Some(BattleOutcome::Draw),
            };
        };

        refresh_actor(world, actor);
        let mut actions = Vec::new();
        loop {
            let actor_side = self.side;
            match choose_action(world, actor, actor_side) {
                Some(Action::Pass) | None => break,
                Some(action) => {
                    if let Some(ev) = preview_action(world, actor, &action) {
                        actions.push(ev);
                    }
                    apply_action(world, actor, &action);
                }
            }
        }

        TurnEvent { actor: Some(actor), side: self.side, actions, outcome: check_outcome(world) }
    }
}

// ── Full-run simulation ──────────────────────────────────────────────────────

/// Run a battle to completion. Each round the player side acts first, then the
/// enemy side. Within a side, combatants act in descending speed order.
pub fn simulate_battle(world: &mut World) -> BattleOutcome {
    for _ in 0..MAX_ROUNDS {
        run_side(world, Side::Player);
        if let Some(o) = check_outcome(world) { return o; }

        run_side(world, Side::Enemy);
        if let Some(o) = check_outcome(world) { return o; }
    }
    BattleOutcome::Draw
}

// ── Shared helpers ───────────────────────────────────────────────────────────

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

fn check_outcome(world: &mut World) -> Option<BattleOutcome> {
    if all_defeated(world, Side::Enemy) {
        return Some(BattleOutcome::PlayerVictory);
    }
    if all_defeated(world, Side::Player) {
        return Some(BattleOutcome::PlayerDefeated);
    }
    None
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

/// Compute what an action will do (for logging), without applying it.
fn preview_action(world: &World, actor: Entity, action: &Action) -> Option<TurnAction> {
    match action {
        Action::Attack { target } => {
            let atk = world.get::<Stats>(actor).map(|s| s.attack).unwrap_or(0);
            let def = world.get::<Stats>(*target).map(|s| s.defense).unwrap_or(0);
            Some(TurnAction::Attack { target: *target, damage: calc_damage(atk, def) })
        }
        Action::Move { destination } => Some(TurnAction::Move { to: *destination }),
        Action::Pass => None,
    }
}
