use std::collections::VecDeque;

use bevy::prelude::*;
use rand::Rng;

use crate::{
    action_points::ActionPoints,
    character::{Aggression, Character},
    health::Health,
    player_input::{PlayerActionChoice, available_player_actions},
    position::Position,
    stats::Stats,
    terrain::{CoverLevel, Direction, LevelMap},
    turn::{ATTACK_AP_COST, Action, apply_action},
};

pub use crate::turn::TurnAction;

// ── Pure combat math ─────────────────────────────────────────────────────────

/// Calculates physical damage dealt, factoring in attacker's attack and defender's defense.
/// Minimum of 1 damage always applies.
pub fn calc_damage(attack: i32, defense: i32) -> i32 {
    (attack - defense / 2).max(1)
}

/// Base probability that an attack connects (no cover).
pub const BASE_HIT_CHANCE: f32 = 0.90;

/// Returns the hit probability for an attack against a defender at the given cover level.
/// Partial cover reduces hit chance to 65%; full cover to 35%.
pub fn calc_hit_chance(cover: CoverLevel) -> f32 {
    match cover {
        CoverLevel::None => BASE_HIT_CHANCE,
        CoverLevel::Partial => 0.65,
        CoverLevel::Full => 0.35,
    }
}

/// Rolls to determine whether an attack hits. Returns `true` on a hit.
pub fn roll_hit(hit_chance: f32, rng: &mut impl Rng) -> bool {
    rng.r#gen::<f32>() < hit_chance
}

/// Determines turn order by speed (highest speed acts first).
/// Returns indices into the provided speed slice, sorted descending.
pub fn turn_order(speeds: &[i32]) -> Vec<usize> {
    let mut indexed: Vec<(usize, i32)> = speeds.iter().copied().enumerate().collect();
    indexed.sort_by(|a, b| b.1.cmp(&a.1));
    indexed.into_iter().map(|(i, _)| i).collect()
}

// ── Battle types ─────────────────────────────────────────────────────────────

/// Hard cap on rounds to prevent infinite loops.
pub const MAX_ROUNDS: u32 = 1000;

/// Whose turn it is in the current combat round.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Turn {
    Player,
    Enemy,
}

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
    pub turn: Turn,
    pub actions: Vec<TurnAction>,
    /// Set when the battle has ended after this step.
    pub outcome: Option<BattleOutcome>,
}

/// Result of executing one player action via [`BattleStep::step_player_action`].
#[derive(Debug)]
pub struct PlayerTurnStep {
    /// The entity that acted.
    pub actor: Entity,
    /// The logged action (None if the choice was Pass or otherwise invalid).
    pub action: Option<TurnAction>,
    /// Whether the actor's turn has now ended (AP exhausted or Pass chosen).
    /// When `true`, the next call to `player_choices` returns choices for the
    /// following player actor, or an empty vec if all players have acted.
    pub turn_ended: bool,
    /// Set when the battle ends as a result of this action.
    pub outcome: Option<BattleOutcome>,
}

/// Incremental battle driver: call `step()` once per key-press.
/// All combatants must carry `Health`, `Stats`, `ActionPoints`, and `Character`.
pub struct BattleStep {
    pub round: u32,
    pub turn: Turn,
    actor_queue: VecDeque<Entity>,
    /// Whether the current player actor's AP has been refreshed this turn.
    player_actor_ready: bool,
}

impl BattleStep {
    pub fn new(world: &mut World) -> Self {
        let queue = VecDeque::from(living_players(world));
        Self {
            round: 1,
            turn: Turn::Player,
            actor_queue: queue,
            player_actor_ready: false,
        }
    }

    /// Returns available actions for the next queued player actor.
    ///
    /// Refreshes the actor's AP automatically the first time this is called for
    /// a given actor. Returns an empty vec when:
    /// * it is not the player's turn (`self.turn == Turn::Enemy`),
    /// * all player actors have already acted (queue empty), or
    /// * the battle is already over.
    pub fn player_choices(&mut self, world: &mut World) -> Vec<PlayerActionChoice> {
        if self.turn != Turn::Player || check_outcome(world).is_some() {
            return vec![];
        }
        let actor = match self.actor_queue.front().copied() {
            Some(e) => e,
            None => return vec![],
        };
        if !self.player_actor_ready {
            refresh_actor(world, actor);
            self.player_actor_ready = true;
        }
        available_player_actions(world, actor)
    }

    /// Execute one player-chosen action for the current queued player actor.
    ///
    /// When `result.turn_ended` is `true`, the actor has been removed from the
    /// queue. Call `player_choices` again to get options for the next player
    /// actor. Once the player queue is exhausted, `self.turn` switches to
    /// `Turn::Enemy` automatically; call `step()` for each enemy actor then.
    pub fn step_player_action(
        &mut self,
        world: &mut World,
        choice: &PlayerActionChoice,
    ) -> PlayerTurnStep {
        let actor = match self.actor_queue.front().copied() {
            Some(e) => e,
            None => {
                return PlayerTurnStep {
                    actor: Entity::PLACEHOLDER,
                    action: None,
                    turn_ended: true,
                    outcome: check_outcome(world),
                };
            }
        };

        if !self.player_actor_ready {
            refresh_actor(world, actor);
            self.player_actor_ready = true;
        }

        let action = choice.to_action();
        let logged = apply_action(world, actor, &action);

        let is_pass = matches!(choice, PlayerActionChoice::Pass);
        let ap_remaining = world
            .get::<ActionPoints>(actor)
            .map(|ap| ap.current)
            .unwrap_or(0);
        let turn_ended = is_pass || ap_remaining == 0;

        if turn_ended {
            self.actor_queue.pop_front();
            self.player_actor_ready = false;

            // When all players have acted, switch to the enemy turn.
            if self.actor_queue.is_empty() {
                self.turn = Turn::Enemy;
                self.actor_queue = VecDeque::from(living_enemies(world));
            }
        }

        PlayerTurnStep {
            actor,
            action: logged,
            turn_ended,
            outcome: check_outcome(world),
        }
    }

    /// The next entity waiting to act this turn, if any.
    pub fn next_actor(&self) -> Option<Entity> {
        self.actor_queue.front().copied()
    }

    /// Advance one actor's full turn (all AP spent). Returns what happened.
    pub fn step(&mut self, world: &mut World) -> TurnEvent {
        if let Some(outcome) = check_outcome(world) {
            return TurnEvent {
                actor: None,
                turn: self.turn,
                actions: vec![],
                outcome: Some(outcome),
            };
        }

        // Refill queue when the current side is exhausted.
        if self.actor_queue.is_empty() {
            match self.turn {
                Turn::Player => {
                    self.turn = Turn::Enemy;
                    self.actor_queue = VecDeque::from(living_enemies(world));
                }
                Turn::Enemy => {
                    self.round += 1;
                    if self.round > MAX_ROUNDS {
                        return TurnEvent {
                            actor: None,
                            turn: self.turn,
                            actions: vec![],
                            outcome: Some(BattleOutcome::Draw),
                        };
                    }
                    self.turn = Turn::Player;
                    self.actor_queue = VecDeque::from(living_players(world));
                }
            }
            // Re-check after switching (e.g. all enemies already dead).
            if let Some(outcome) = check_outcome(world) {
                return TurnEvent {
                    actor: None,
                    turn: self.turn,
                    actions: vec![],
                    outcome: Some(outcome),
                };
            }
        }

        let Some(actor) = self.actor_queue.pop_front() else {
            return TurnEvent {
                actor: None,
                turn: self.turn,
                actions: vec![],
                outcome: Some(BattleOutcome::Draw),
            };
        };

        refresh_actor(world, actor);
        let mut actions = Vec::new();
        loop {
            let actor_turn = self.turn;
            match choose_action(world, actor, actor_turn) {
                Some(Action::Pass) | None => break,
                Some(action) => {
                    if let Some(ev) = apply_action(world, actor, &action) {
                        actions.push(ev);
                    }
                }
            }
        }

        TurnEvent {
            actor: Some(actor),
            turn: self.turn,
            actions,
            outcome: check_outcome(world),
        }
    }
}

// ── Full-run simulation ──────────────────────────────────────────────────────

/// Run a battle to completion. Each round the player side acts first, then the
/// enemy side. Within a side, combatants act in descending speed order.
pub fn simulate_battle(world: &mut World) -> BattleOutcome {
    for _ in 0..MAX_ROUNDS {
        run_side(world, Turn::Player);
        if let Some(o) = check_outcome(world) {
            return o;
        }

        run_side(world, Turn::Enemy);
        if let Some(o) = check_outcome(world) {
            return o;
        }
    }
    BattleOutcome::Draw
}

// ── Shared helpers ───────────────────────────────────────────────────────────

fn run_side(world: &mut World, turn: Turn) {
    let actors = match turn {
        Turn::Player => living_players(world),
        Turn::Enemy => living_enemies(world),
    };
    for actor in actors {
        refresh_actor(world, actor);
        loop {
            match choose_action(world, actor, turn) {
                Some(Action::Pass) | None => break,
                Some(action) => {
                    apply_action(world, actor, &action);
                }
            }
        }
    }
}

fn check_outcome(world: &mut World) -> Option<BattleOutcome> {
    if all_enemies_defeated(world) {
        return Some(BattleOutcome::PlayerVictory);
    }
    if all_players_defeated(world) {
        return Some(BattleOutcome::PlayerDefeated);
    }
    None
}

fn refresh_actor(world: &mut World, actor: Entity) {
    if let Some(mut ap) = world.get_mut::<ActionPoints>(actor) {
        ap.refresh();
    }
}

/// Returns living player characters, sorted by descending speed.
fn living_players(world: &mut World) -> Vec<Entity> {
    let mut query = world.query::<(Entity, &Character, &Health, &Stats)>();
    let mut entities: Vec<(Entity, i32)> = query
        .iter(world)
        .filter(|(_, c, h, _)| c.kind.is_player() && h.is_alive())
        .map(|(e, _, _, stats)| (e, stats.speed))
        .collect();
    entities.sort_by(|a, b| b.1.cmp(&a.1));
    entities.into_iter().map(|(e, _)| e).collect()
}

/// Returns living non-friendly NPCs, sorted by descending speed.
fn living_enemies(world: &mut World) -> Vec<Entity> {
    let mut query = world.query::<(Entity, &Character, &Health, &Stats)>();
    let mut entities: Vec<(Entity, i32)> = query
        .iter(world)
        .filter(|(_, c, h, _)| {
            !c.kind.is_player() && c.aggression != Aggression::Friendly && h.is_alive()
        })
        .map(|(e, _, _, stats)| (e, stats.speed))
        .collect();
    entities.sort_by(|a, b| b.1.cmp(&a.1));
    entities.into_iter().map(|(e, _)| e).collect()
}

/// `true` if every player character is dead, or none exist.
fn all_players_defeated(world: &mut World) -> bool {
    let mut query = world.query::<(&Character, &Health)>();
    let combatants: Vec<bool> = query
        .iter(world)
        .filter(|(c, _)| c.kind.is_player())
        .map(|(_, h)| h.is_alive())
        .collect();
    combatants.is_empty() || combatants.iter().all(|alive| !alive)
}

/// `true` if every non-friendly NPC is dead, or none exist.
fn all_enemies_defeated(world: &mut World) -> bool {
    let mut query = world.query::<(&Character, &Health)>();
    let combatants: Vec<bool> = query
        .iter(world)
        .filter(|(c, _)| !c.kind.is_player() && c.aggression != Aggression::Friendly)
        .map(|(_, h)| h.is_alive())
        .collect();
    combatants.is_empty() || combatants.iter().all(|alive| !alive)
}

// ── Smart AI ─────────────────────────────────────────────────────────────────

/// AI entry point: seek cover first, then attack the best target.
fn choose_action(world: &mut World, actor: Entity, turn: Turn) -> Option<Action> {
    let ap = world.get::<ActionPoints>(actor)?.current;
    if ap == 0 {
        return Some(Action::Pass);
    }

    // Phase 1: move to cover if not already well-covered from nearest enemy.
    if let Some(mv) = seek_cover_action(world, actor, turn, ap) {
        return Some(mv);
    }

    // Phase 2: attack the target most likely to take significant damage.
    if ap >= ATTACK_AP_COST
        && let Some(target) = best_attack_target(world, actor, turn)
    {
        return Some(Action::Attack { target });
    }

    Some(Action::Pass)
}

/// Returns a `Move` action toward the best available cover tile.
///
/// Phase 1 — reserve AP for attack: look for better cover within `ap - ATTACK_AP_COST` tiles.
///   If found, move there so the actor can still attack this turn.
/// Phase 2 — advance toward cover: if no in-range cover exists, spend ALL AP to advance toward
///   the best reachable cover tile (skipping the attack this turn).
/// Returns `None` only if already at Full cover or no better cover exists anywhere in range.
fn seek_cover_action(world: &mut World, actor: Entity, turn: Turn, ap: i32) -> Option<Action> {
    if ap == 0 {
        return None;
    }

    let actor_pos = world.get::<Position>(actor).copied()?;

    // Find the nearest living opponent position (collect then drop query borrow).
    let opponent_positions: Vec<Position> = match turn {
        Turn::Player => {
            let mut q = world.query::<(&Character, &Health, &Position)>();
            q.iter(world)
                .filter(|(c, h, _)| {
                    !c.kind.is_player() && c.aggression != Aggression::Friendly && h.is_alive()
                })
                .map(|(_, _, pos)| *pos)
                .collect()
        }
        Turn::Enemy => {
            let mut q = world.query::<(&Character, &Health, &Position)>();
            q.iter(world)
                .filter(|(c, h, _)| c.kind.is_player() && h.is_alive())
                .map(|(_, _, pos)| *pos)
                .collect()
        }
    };

    let nearest_opponent = opponent_positions
        .iter()
        .min_by_key(|p| (p.x - actor_pos.x).abs() + (p.y - actor_pos.y).abs())
        .copied()?;

    // Attack comes from the opponent's direction.
    let attack_dir = Direction::from_attack(
        (nearest_opponent.x, nearest_opponent.y),
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

    // Scan all passable tiles within the full AP budget for better cover.
    let (cols, rows) = world
        .get_resource::<LevelMap>()
        .map(|m| (m.cols as i32, m.rows as i32))
        .unwrap_or((0, 0));

    let mut candidates: Vec<(i32, i32, i32, CoverLevel)> = Vec::new(); // (dist, x, y, cover)
    if let Some(map) = world.get_resource::<LevelMap>() {
        for dy in -ap..=ap {
            for dx in -ap..=ap {
                let dist = dx.abs() + dy.abs();
                if dist == 0 || dist > ap {
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

    if candidates.is_empty() {
        return None;
    }

    // Sort: best cover first, then closest.
    candidates.sort_by(|a, b| b.3.cmp(&a.3).then(a.0.cmp(&b.0)));

    // Phase 1: prefer a tile reachable while keeping enough AP to attack after.
    let attack_budget = ap - ATTACK_AP_COST;
    if attack_budget > 0
        && let Some(&(_, tx, ty, _)) = candidates
            .iter()
            .find(|&&(dist, _, _, _)| dist <= attack_budget)
    {
        return Some(Action::Move {
            destination: Position::new(tx, ty, actor_pos.z),
        });
    }

    // Phase 2: advance toward the best cover using all AP (no attack this turn).
    candidates.first().map(|&(_, tx, ty, _)| Action::Move {
        destination: Position::new(tx, ty, actor_pos.z),
    })
}

/// Returns the entity that gives the highest expected damage (hit_chance × damage),
/// preferring closer targets on ties.
fn best_attack_target(world: &mut World, actor: Entity, turn: Turn) -> Option<Entity> {
    let actor_pos = world.get::<Position>(actor).copied()?;
    let actor_attack = world.get::<Stats>(actor).map(|s| s.attack).unwrap_or(0);

    // Collect target data (drop query borrow before accessing resources).
    let targets: Vec<(Entity, i32, i32, i32)> = match turn {
        Turn::Player => {
            let mut q = world.query::<(Entity, &Character, &Health, &Stats, &Position)>();
            q.iter(world)
                .filter(|(_, c, h, _, _)| {
                    !c.kind.is_player() && c.aggression != Aggression::Friendly && h.is_alive()
                })
                .map(|(e, _, _, stats, pos)| (e, stats.defense, pos.x, pos.y))
                .collect()
        }
        Turn::Enemy => {
            let mut q = world.query::<(Entity, &Character, &Health, &Stats, &Position)>();
            q.iter(world)
                .filter(|(_, c, h, _, _)| c.kind.is_player() && h.is_alive())
                .map(|(e, _, _, stats, pos)| (e, stats.defense, pos.x, pos.y))
                .collect()
        }
    };

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
