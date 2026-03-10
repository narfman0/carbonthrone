use std::collections::VecDeque;

use bevy::prelude::*;
use rand::Rng;

use crate::{
    ability::{AbilityEffect, AbilityKind, character_abilities},
    action_points::ActionPoints,
    character::{Aggression, Character},
    health::Health,
    player_input::{PlayerActionChoice, available_player_actions},
    position::Position,
    scripted_encounter::{ScriptedAlly, ScriptedFirstAction},
    stats::Stats,
    terrain::{CoverLevel, Direction, LevelMap},
    turn::{Action, MOVE_AP_COST, apply_action, move_range_per_ap},
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
}

impl BattleStep {
    pub fn new(world: &mut World) -> Self {
        let players = living_players(world);
        // Refresh all player APs at battle start.
        for &e in &players {
            refresh_actor(world, e);
        }
        Self {
            round: 1,
            turn: Turn::Player,
            actor_queue: VecDeque::from(players),
        }
    }

    /// Returns available actions for the next queued player actor.
    ///
    /// Returns an empty vec when:
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

            // When all players have acted, switch to the enemy turn and refresh all enemies.
            if self.actor_queue.is_empty() {
                self.turn = Turn::Enemy;
                let enemies = living_enemies(world);
                for &e in &enemies {
                    refresh_actor(world, e);
                }
                self.actor_queue = VecDeque::from(enemies);
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

    /// All player entities still queued to act this round (front = currently active).
    pub fn player_queue(&self) -> &VecDeque<Entity> {
        &self.actor_queue
    }

    /// Move `entity` to the front of the player queue so they act next.
    /// Only meaningful when `self.turn == Turn::Player` and `entity` is in the queue.
    /// Returns `true` if the entity was found and moved.
    pub fn set_active_player(&mut self, entity: Entity) -> bool {
        if self.turn != Turn::Player {
            return false;
        }
        let Some(pos) = self.actor_queue.iter().position(|&e| e == entity) else {
            return false;
        };
        if pos != 0 {
            self.actor_queue.remove(pos);
            self.actor_queue.push_front(entity);
        }
        true
    }

    /// Returns `true` when the next queued player actor is a scripted
    /// (AI-controlled) ally rather than a true player character.
    ///
    /// Returns the entity that is currently acting (front of the queue), if any.
    pub fn current_actor(&self) -> Option<Entity> {
        self.actor_queue.front().copied()
    }

    /// Callers that drive combat interactively should auto-advance scripted
    /// allies by calling [`Self::step`] instead of waiting for player input.
    pub fn current_actor_is_scripted_ally(&self, world: &World) -> bool {
        self.actor_queue
            .front()
            .map(|&e| world.get::<ScriptedAlly>(e).is_some())
            .unwrap_or(false)
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
                    let enemies = living_enemies(world);
                    for &e in &enemies {
                        refresh_actor(world, e);
                    }
                    self.actor_queue = VecDeque::from(enemies);
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
                    let players = living_players(world);
                    for &e in &players {
                        refresh_actor(world, e);
                    }
                    self.actor_queue = VecDeque::from(players);
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
            // Return after side switch so the new side's input mechanism handles it.
            return TurnEvent {
                actor: None,
                turn: self.turn,
                actions: vec![],
                outcome: None,
            };
        }

        let Some(actor) = self.actor_queue.pop_front() else {
            return TurnEvent {
                actor: None,
                turn: self.turn,
                actions: vec![],
                outcome: Some(BattleOutcome::Draw),
            };
        };

        let mut actions = Vec::new();
        loop {
            let actor_turn = self.turn;
            match choose_action(world, actor, actor_turn) {
                Some(Action::Pass) | None => break,
                Some(action) => match apply_action(world, actor, &action) {
                    Some(ev) => actions.push(ev),
                    // Action failed without spending AP (e.g. occupied tile) — end turn.
                    None => break,
                },
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
    // Refresh all actors' AP at the start of their side's turn.
    for &actor in &actors {
        refresh_actor(world, actor);
    }
    for actor in actors {
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

/// Returns the minimum AP cost of any offensive (Melee or Ranged damage) ability
/// for the given actor. Falls back to 2 if no character or no abilities found.
fn min_offensive_ap_cost(world: &mut World, actor: Entity) -> i32 {
    let kind = match world.get::<Character>(actor) {
        Some(c) => c.kind.clone(),
        None => return 2,
    };
    character_abilities(&kind)
        .iter()
        .filter(|a| matches!(a.kind, AbilityKind::Melee | AbilityKind::Ranged))
        .filter(|a| is_damage_ability(&a.effect))
        .map(|a| a.ap_cost)
        .min()
        .unwrap_or(2)
}

/// Returns `true` if the ability effect deals direct damage.
fn is_damage_ability(effect: &AbilityEffect) -> bool {
    matches!(
        effect,
        AbilityEffect::BonusDamage { .. }
            | AbilityEffect::ArmorPiercing { .. }
            | AbilityEffect::ArmorPiercingStrike { .. }
    )
}

/// Returns `true` for abilities that can be directed at an enemy to impair them
/// (damage dealing or AP disruption).
fn is_offensive_ability(effect: &AbilityEffect) -> bool {
    matches!(
        effect,
        AbilityEffect::BonusDamage { .. }
            | AbilityEffect::ArmorPiercing { .. }
            | AbilityEffect::ArmorPiercingStrike { .. }
            | AbilityEffect::DrainAP { .. }
    )
}

/// AI entry point: execute a scripted first action if present, then seek cover
/// and choose an ability via normal tactics.
fn choose_action(world: &mut World, actor: Entity, turn: Turn) -> Option<Action> {
    let ap = world.get::<ActionPoints>(actor)?.current;
    if ap == 0 {
        return Some(Action::Pass);
    }

    // Check for a scripted first action before falling back to tactical AI.
    if let Some(scripted) = choose_scripted_action(world, actor, turn) {
        return Some(scripted);
    }

    let min_cost = min_offensive_ap_cost(world, actor);

    // Phase 1: move to cover if not already well-covered from nearest enemy.
    if let Some(mv) = seek_cover_action(world, actor, turn, ap, min_cost) {
        return Some(mv);
    }

    // Phase 2: use an offensive ability.
    if let Some(ability_action) = choose_offensive_ability_action(world, actor, turn, ap) {
        return Some(ability_action);
    }

    Some(Action::Pass)
}

/// If the actor has an unexecuted [`ScriptedFirstAction`], attempt to use that
/// ability and mark it as executed.  Returns `None` when the component is
/// absent, already executed, the ability is unknown, or no valid target exists.
fn choose_scripted_action(world: &mut World, actor: Entity, turn: Turn) -> Option<Action> {
    // Peek at the scripted component without holding a reference.
    let ability_name: &'static str = {
        let sfa = world.get::<ScriptedFirstAction>(actor)?;
        if sfa.executed {
            return None;
        }
        sfa.ability_name
    };

    let (kind, level) = {
        let c = world.get::<Character>(actor)?;
        (c.kind.clone(), c.level)
    };

    // Locate the ability by name in this character's ability table.
    let ability = character_abilities(&kind)
        .into_iter()
        .find(|a| a.name == ability_name && a.level_required <= level)?;

    let actor_pos = world.get::<Position>(actor).copied()?;

    // Resolve the target based on ability kind.
    let target: Option<Entity> = match ability.kind {
        AbilityKind::Melee => {
            // Collect adjacent enemy positions (borrow dropped before mutable access).
            let target_positions: Vec<(Entity, i32, i32)> = {
                let mut q = world.query::<(Entity, &Character, &Health, &Position)>();
                q.iter(world)
                    .filter(|(_, c, h, _)| match turn {
                        Turn::Player => {
                            !c.kind.is_player()
                                && c.aggression != Aggression::Friendly
                                && h.is_alive()
                        }
                        Turn::Enemy => c.kind.is_player() && h.is_alive(),
                    })
                    .map(|(e, _, _, pos)| (e, pos.x, pos.y))
                    .collect()
            };
            best_adjacent_target(&target_positions, actor_pos)
        }
        AbilityKind::Ranged => best_attack_target(world, actor, turn),
        // RangedAlly and Utility are not used by scripted actions.
        AbilityKind::RangedAlly | AbilityKind::Utility => None,
    };

    // For targeted abilities that need a target, skip if none available.
    if matches!(ability.kind, AbilityKind::Melee | AbilityKind::Ranged) && target.is_none() {
        // Can't execute yet — don't mark as executed so the normal AI can act.
        return None;
    }

    // Mark the scripted action as executed before returning.
    if let Some(mut sfa) = world.get_mut::<ScriptedFirstAction>(actor) {
        sfa.executed = true;
    }

    Some(Action::UseAbility { ability, target })
}

/// Chooses an offensive ability and target for the actor.
///
/// Prefers ranged abilities (usable from any distance), then melee (adjacent only).
/// Returns `None` if no valid target/ability combination is available.
fn choose_offensive_ability_action(
    world: &mut World,
    actor: Entity,
    turn: Turn,
    ap: i32,
) -> Option<Action> {
    let (kind, level) = {
        let c = world.get::<Character>(actor)?;
        (c.kind.clone(), c.level)
    };
    let actor_pos = world.get::<Position>(actor).copied()?;

    let abilities = character_abilities(&kind);
    let available: Vec<_> = abilities
        .iter()
        .filter(|a| a.level_required <= level && a.ap_cost <= ap && is_offensive_ability(&a.effect))
        .collect();

    // Collect target positions for adjacency checks.
    let target_positions: Vec<(Entity, i32, i32)> = {
        let mut q = world.query::<(Entity, &Character, &Health, &Position)>();
        q.iter(world)
            .filter(|(_, c, h, _)| match turn {
                Turn::Player => {
                    !c.kind.is_player() && c.aggression != Aggression::Friendly && h.is_alive()
                }
                Turn::Enemy => c.kind.is_player() && h.is_alive(),
            })
            .map(|(e, _, _, pos)| (e, pos.x, pos.y))
            .collect()
    };

    // Try ranged offensive abilities first.
    if let Some(ability) = available.iter().find(|a| a.kind == AbilityKind::Ranged) {
        if let Some(target) = best_attack_target(world, actor, turn) {
            return Some(Action::UseAbility {
                ability: (*ability).clone(),
                target: Some(target),
            });
        }
    }

    // Try melee offensive abilities if an adjacent target exists.
    if let Some(ability) = available.iter().find(|a| a.kind == AbilityKind::Melee) {
        if let Some(target) = best_adjacent_target(&target_positions, actor_pos) {
            return Some(Action::UseAbility {
                ability: (*ability).clone(),
                target: Some(target),
            });
        }
    }

    None
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

/// Returns the best adjacent target (Chebyshev distance ≤ 1) for a melee attack.
fn best_adjacent_target(targets: &[(Entity, i32, i32)], actor_pos: Position) -> Option<Entity> {
    targets
        .iter()
        .filter(|&&(_, tx, ty)| {
            let chebyshev = (actor_pos.x - tx).abs().max((actor_pos.y - ty).abs());
            chebyshev <= 1 && chebyshev > 0
        })
        .map(|&(e, tx, ty)| {
            let dist = (actor_pos.x - tx).abs() + (actor_pos.y - ty).abs();
            (e, dist)
        })
        .min_by_key(|&(_, dist)| dist)
        .map(|(e, _)| e)
}

/// Returns a `Move` action toward the best available cover tile.
///
/// Phase 1 — reserve AP for attack: look for better cover within `ap - min_attack_cost` tiles.
///   If found, move there so the actor can still attack this turn.
/// Phase 2 — advance toward cover: if no in-range cover exists, spend ALL AP to advance toward
///   the best reachable cover tile (skipping the attack this turn).
/// Returns `None` only if already at Full cover or no better cover exists anywhere in range.
fn seek_cover_action(
    world: &mut World,
    actor: Entity,
    turn: Turn,
    ap: i32,
    min_attack_cost: i32,
) -> Option<Action> {
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

    // Collect positions occupied by other living combatants.
    let occupied: std::collections::HashSet<(i32, i32)> = {
        let mut occ_q = world.query::<(Entity, &Position, &Health)>();
        occ_q
            .iter(world)
            .filter(|(e, _, h)| *e != actor && h.is_alive())
            .map(|(_, p, _)| (p.x, p.y))
            .collect()
    };

    let speed = world.get::<Stats>(actor).map(|s| s.speed).unwrap_or(8);
    let range = move_range_per_ap(speed);

    let mut candidates: Vec<(i32, i32, i32, CoverLevel)> = Vec::new(); // (ap_cost, x, y, cover)
    if let Some(map) = world.get_resource::<LevelMap>() {
        for dy in -ap..=ap {
            for dx in -ap..=ap {
                let dist = dx.abs() + dy.abs();
                let cost = MOVE_AP_COST * ((dist + range - 1) / range.max(1));
                if dist == 0 || cost > ap {
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
                if occupied.contains(&(tx, ty)) {
                    continue;
                }
                let cover = map.get_cover(tx, ty, attack_dir);
                if cover > current_cover {
                    candidates.push((cost, tx, ty, cover));
                }
            }
        }
    }

    if candidates.is_empty() {
        return None;
    }

    // Sort: best cover first, then cheapest AP cost.
    candidates.sort_by(|a, b| b.3.cmp(&a.3).then(a.0.cmp(&b.0)));

    // Phase 1: prefer a tile reachable while keeping enough AP to attack after.
    let attack_budget = ap - min_attack_cost;
    if attack_budget > 0
        && let Some(&(_, tx, ty, _)) = candidates
            .iter()
            .find(|&&(cost, _, _, _)| cost <= attack_budget)
    {
        return Some(Action::Move {
            destination: Position::new(tx, ty),
        });
    }

    // Phase 2: advance toward the best cover using all AP (no attack this turn).
    candidates.first().map(|&(_, tx, ty, _)| Action::Move {
        destination: Position::new(tx, ty),
    })
}
