use std::collections::VecDeque;

use bevy::prelude::*;

use crate::{
    action_points::ActionPoints,
    combat::{calc_damage, calc_hit_chance},
    health::Health,
    position::Position,
    side::Side,
    stats::Stats,
    terrain::{CoverLevel, Direction, LevelMap},
    turn::{Action, apply_action, ATTACK_AP_COST},
};

// Re-export so callers can import TurnAction from `simulation` as before.
pub use crate::turn::TurnAction;

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
                    if let Some(ev) = apply_action(world, actor, &action) {
                        actions.push(ev);
                    }
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

// ── Smart AI ─────────────────────────────────────────────────────────────────

/// AI entry point: seek cover first, then attack the best target.
fn choose_action(world: &mut World, actor: Entity, actor_side: Side) -> Option<Action> {
    let ap = world.get::<ActionPoints>(actor)?.current;
    if ap == 0 {
        return Some(Action::Pass);
    }

    // Phase 1: move to cover if not already well-covered from nearest enemy.
    if let Some(mv) = seek_cover_action(world, actor, actor_side, ap) {
        return Some(mv);
    }

    // Phase 2: attack the target most likely to take significant damage.
    let opponent_side = match actor_side {
        Side::Player => Side::Enemy,
        Side::Enemy => Side::Player,
    };
    if ap >= ATTACK_AP_COST
        && let Some(target) = best_attack_target(world, actor, opponent_side)
    {
        return Some(Action::Attack { target });
    }

    Some(Action::Pass)
}

/// Returns a `Move` action toward the best available cover tile, or `None` if
/// the actor is already in full cover or there's not enough AP to move and attack.
fn seek_cover_action(
    world: &mut World,
    actor: Entity,
    actor_side: Side,
    ap: i32,
) -> Option<Action> {
    // Reserve enough AP to attack after moving.
    let move_budget = ap - ATTACK_AP_COST;
    if move_budget <= 0 {
        return None;
    }

    let actor_pos = world.get::<Position>(actor).copied()?;
    let opponent_side = match actor_side {
        Side::Player => Side::Enemy,
        Side::Enemy => Side::Player,
    };

    // Find the nearest living enemy position (collect then drop query borrow).
    let mut q = world.query::<(Entity, &Side, &Health, &Position)>();
    let enemy_positions: Vec<Position> = q
        .iter(world)
        .filter(|(_, s, h, _)| **s == opponent_side && h.is_alive())
        .map(|(_, _, _, pos)| *pos)
        .collect();

    let nearest_enemy = enemy_positions
        .iter()
        .min_by_key(|p| (p.x - actor_pos.x).abs() + (p.y - actor_pos.y).abs())
        .copied()?;

    // Attack comes from the enemy's direction.
    let attack_dir = Direction::from_attack(
        (nearest_enemy.x, nearest_enemy.y),
        (actor_pos.x, actor_pos.y),
    );

    // Check current cover level; don't move if already fully covered.
    let current_cover = world
        .get_resource::<LevelMap>()
        .map(|m| m.get_cover(actor_pos.x, actor_pos.y, attack_dir))
        .unwrap_or(CoverLevel::None);

    if current_cover == CoverLevel::Full {
        return None;
    }

    // Scan all passable tiles within move_budget for better cover.
    let (cols, rows) = world
        .get_resource::<LevelMap>()
        .map(|m| (m.cols as i32, m.rows as i32))
        .unwrap_or((0, 0));

    let mut candidates: Vec<(i32, i32, i32, CoverLevel)> = Vec::new(); // (dist, x, y, cover)
    if let Some(map) = world.get_resource::<LevelMap>() {
        for dy in -move_budget..=move_budget {
            for dx in -move_budget..=move_budget {
                let dist = dx.abs() + dy.abs();
                if dist == 0 || dist > move_budget {
                    continue;
                }
                let tx = actor_pos.x + dx;
                let ty = actor_pos.y + dy;
                if tx < 0 || ty < 0 || tx >= cols || ty >= rows {
                    continue;
                }
                if !map.is_passable(tx, ty) {
                    continue;
                }
                let cover = map.get_cover(tx, ty, attack_dir);
                if cover > current_cover {
                    candidates.push((dist, tx, ty, cover));
                }
            }
        }
    }

    // Prefer full cover, then closer.
    candidates.sort_by(|a, b| b.3.cmp(&a.3).then(a.0.cmp(&b.0)));
    candidates
        .first()
        .map(|&(_, tx, ty, _)| Action::Move { destination: Position::new(tx, ty, actor_pos.z) })
}

/// Returns the entity that gives the highest expected damage (hit_chance × damage),
/// preferring closer targets on ties.
fn best_attack_target(world: &mut World, actor: Entity, opponent_side: Side) -> Option<Entity> {
    let actor_pos = world.get::<Position>(actor).copied()?;
    let actor_attack = world.get::<Stats>(actor).map(|s| s.attack).unwrap_or(0);

    // Collect target data (drop query borrow before accessing resources).
    let mut q = world.query::<(Entity, &Side, &Health, &Stats, &Position)>();
    let targets: Vec<(Entity, i32, i32, i32)> = q
        .iter(world)
        .filter(|(_, s, h, _, _)| **s == opponent_side && h.is_alive())
        .map(|(e, _, _, stats, pos)| (e, stats.defense, pos.x, pos.y))
        .collect();

    targets
        .iter()
        .map(|&(e, defense, tx, ty)| {
            let dir = Direction::from_attack((actor_pos.x, actor_pos.y), (tx, ty));
            let cover = world
                .get_resource::<LevelMap>()
                .map(|m| m.get_cover(tx, ty, dir))
                .unwrap_or(CoverLevel::None);
            let expected = calc_hit_chance(cover) * calc_damage(actor_attack, defense) as f32;
            let dist = (tx - actor_pos.x).abs() + (ty - actor_pos.y).abs();
            (e, expected, dist)
        })
        .max_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.2.cmp(&a.2)) // prefer closer on tie
        })
        .map(|(e, _, _)| e)
}
