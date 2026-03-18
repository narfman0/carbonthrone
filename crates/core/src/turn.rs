use bevy::prelude::*;

use crate::{
    ability::{Ability, AbilityEffect, AbilityKind, LastPosition, LastUsedAbility},
    action_points::ActionPoints,
    combat::{calc_damage, calc_hit_chance, roll_hit},
    health::Health,
    pending_effects::{CurrentRound, PendingEffect, PendingEffects},
    position::Position,
    stats::Stats,
    temporal_flux::TemporalFlux,
    terrain::{BattleRng, CoverLevel, Direction, LevelMap},
};

pub const MOVE_AP_COST: i32 = 1;

/// Tiles a combatant with the given speed stat can traverse per AP spent.
/// speed 1-7 → 1 tile/AP, 8-15 → 2, 16-23 → 3, etc.
pub fn move_range_per_ap(speed: i32) -> i32 {
    (speed / 8).max(1)
}

/// AP cost to traverse `steps` BFS tiles for an actor with the given speed stat.
pub fn move_ap_cost(steps: i32, speed: i32) -> i32 {
    let range = move_range_per_ap(speed);
    MOVE_AP_COST * ((steps + range - 1) / range)
}

/// Grid positions occupied by every living combatant except `exclude`.
pub fn occupied_tiles(world: &mut World, exclude: Entity) -> std::collections::HashSet<(i32, i32)> {
    let mut q = world.query::<(Entity, &Position, &Health)>();
    q.iter(world)
        .filter(|(e, _, h)| *e != exclude && h.is_alive())
        .map(|(_, p, _)| (p.x, p.y))
        .collect()
}

/// BFS path from `actor`'s current tile to `dest`, avoiding obstacles and other living entities.
/// Returns an empty vec if `dest` equals the actor's position or is unreachable.
pub fn bfs_move_path(world: &mut World, actor: Entity, dest: (i32, i32)) -> Vec<(i32, i32)> {
    let from = match world.get::<Position>(actor) {
        Some(p) => (p.x, p.y),
        None => return vec![],
    };
    if from == dest {
        return vec![];
    }
    let occupied = occupied_tiles(world, actor);
    world
        .get_resource::<LevelMap>()
        .map(|map| map.bfs_path(from, dest, &occupied))
        .unwrap_or_default()
}

/// An action a combatant can take on their turn, each costing AP.
#[derive(Debug, Clone)]
pub enum Action {
    /// Move to a destination. Costs `MOVE_AP_COST` × Manhattan distance AP.
    Move { destination: Position },
    /// Use a class ability. `target` is required for targeted effects; `None` for self-targeted.
    UseAbility {
        ability: Ability,
        target: Option<Entity>,
    },
    /// End the turn without spending AP.
    Pass,
}

/// One action that occurred during a combatant's turn (for the event log).
#[derive(Debug, Clone)]
pub enum TurnAction {
    Move {
        to: Position,
    },
    /// An ability was used. `value` is the primary numeric result (damage, HP restored, AP drained/gained).
    UseAbility {
        ability_name: &'static str,
        target: Option<Entity>,
        value: i32,
        hit: bool,
    },
    /// Displacement hit queued: target was pushed and delayed damage is pending.
    DisplacementQueued {
        target: Entity,
        pending_damage: i32,
        resolve_round: u32,
        pushed_to: Option<Position>,
    },
    /// Delayed displacement damage resolved.
    DisplacementHit {
        target: Entity,
        damage: i32,
    },
    /// Acceleration AP burst applied to the actor.
    AccelerationApplied {
        actor: Entity,
        bonus_ap: i32,
    },
    /// AP drain resolved (from Acceleration recoil or similar).
    DrainApplied {
        target: Entity,
        drained: i32,
    },
    /// Temporal Collapse: flux reached 100, dealing damage and draining all combatants.
    TemporalCollapse {
        victim: Entity,
        damage: i32,
    },
    /// Temporal Recall: target snapped back to a previous position.
    TemporalRecalled {
        target: Entity,
        from: Position,
        to: Position,
    },
    /// Flux-induced glitch teleport.
    GlitchTeleport {
        entity: Entity,
        from: Position,
        to: Position,
    },
}

/// Execute an action for `actor`.
/// Returns `Some(TurnAction)` if the action was carried out, `None` if it
/// was invalid (insufficient AP, dead target, blocked tile, out of melee range, etc.) or was Pass.
pub fn apply_action(world: &mut World, actor: Entity, action: &Action) -> Option<TurnAction> {
    match action {
        Action::Move { destination } => {
            let speed = world.get::<Stats>(actor).map(|s| s.speed).unwrap_or(8);

            // Reject moves onto tiles occupied by other living combatants.
            let occupied = occupied_tiles(world, actor);
            if occupied.contains(&(destination.x, destination.y)) {
                return None;
            }

            // BFS gives obstacle-aware path length; empty = unreachable or same tile.
            let from = match world.get::<Position>(actor) {
                Some(p) => (p.x, p.y),
                None => return None,
            };
            if from == (destination.x, destination.y) {
                return None;
            }
            let steps = if world.get_resource::<LevelMap>().is_some() {
                let path = world
                    .get_resource::<LevelMap>()
                    .map(|map| map.bfs_path(from, (destination.x, destination.y), &occupied))
                    .unwrap_or_default();
                if path.is_empty() {
                    return None;
                }
                path.len() as i32
            } else {
                // No map: use Manhattan distance (no obstacle checking).
                (destination.x - from.0).abs() + (destination.y - from.1).abs()
            };

            let cost = move_ap_cost(steps, speed);
            let ap = world
                .get::<ActionPoints>(actor)
                .map(|ap| ap.current)
                .unwrap_or(0);
            if ap < cost {
                return None;
            }

            // Save current position as LastPosition before moving.
            world
                .entity_mut(actor)
                .insert(LastPosition(Position::new(from.0, from.1)));

            world.get_mut::<ActionPoints>(actor).unwrap().spend(cost);
            if let Some(mut p) = world.get_mut::<Position>(actor) {
                *p = *destination;
            }
            Some(TurnAction::Move { to: *destination })
        }
        Action::UseAbility { ability, target } => apply_ability(world, actor, ability, *target),
        Action::Pass => None,
    }
}

/// Execute an ability for `actor` against `target` (or self if `target` is `None`).
fn apply_ability(
    world: &mut World,
    actor: Entity,
    ability: &Ability,
    target: Option<Entity>,
) -> Option<TurnAction> {
    // Check AP before any validation so we never spend AP on invalid actions.
    let ap = world
        .get::<ActionPoints>(actor)
        .map(|ap| ap.current)
        .unwrap_or(0);
    if ap < ability.ap_cost {
        return None;
    }

    // Enforce melee range: target must be within Chebyshev distance 1 (adjacent or diagonal).
    // Only enforced when both actor and target have Position components.
    if ability.kind == AbilityKind::Melee
        && let Some(target_entity) = target
        && let (Some(actor_pos), Some(target_pos)) = (
            world.get::<Position>(actor).copied(),
            world.get::<Position>(target_entity).copied(),
        )
    {
        let chebyshev = (actor_pos.x - target_pos.x)
            .abs()
            .max((actor_pos.y - target_pos.y).abs());
        if chebyshev > 1 {
            return None;
        }
    }

    let current_round = world
        .get_resource::<CurrentRound>()
        .map(|r| r.0)
        .unwrap_or(0);

    let result = match &ability.effect {
        AbilityEffect::BonusDamage { bonus } => {
            let target_entity = target?;
            if !world
                .get::<Health>(target_entity)
                .map(|h| h.is_alive())
                .unwrap_or(false)
            {
                return None;
            }
            world.get_mut::<ActionPoints>(actor)?.spend(ability.ap_cost);
            let (hit, _cover) = roll_ability_hit(world, actor, target_entity);
            let attack = world.get::<Stats>(actor).map(|s| s.attack).unwrap_or(0);
            let defense = world
                .get::<Stats>(target_entity)
                .map(|s| s.defense)
                .unwrap_or(0);
            let damage = if hit {
                calc_damage(attack, defense) + bonus
            } else {
                0
            };
            if hit {
                world.get_mut::<Health>(target_entity)?.take_damage(damage);
            }
            Some(TurnAction::UseAbility {
                ability_name: ability.name,
                target,
                value: damage,
                hit,
            })
        }

        AbilityEffect::ArmorPiercing { pierce_fraction } => {
            let target_entity = target?;
            if !world
                .get::<Health>(target_entity)
                .map(|h| h.is_alive())
                .unwrap_or(false)
            {
                return None;
            }
            world.get_mut::<ActionPoints>(actor)?.spend(ability.ap_cost);
            let (hit, _cover) = roll_ability_hit(world, actor, target_entity);
            let attack = world.get::<Stats>(actor).map(|s| s.attack).unwrap_or(0);
            let defense = world
                .get::<Stats>(target_entity)
                .map(|s| s.defense)
                .unwrap_or(0);
            let effective_defense = (defense as f32 * (1.0 - pierce_fraction)) as i32;
            let damage = if hit {
                calc_damage(attack, effective_defense)
            } else {
                0
            };
            if hit {
                world.get_mut::<Health>(target_entity)?.take_damage(damage);
            }
            Some(TurnAction::UseAbility {
                ability_name: ability.name,
                target,
                value: damage,
                hit,
            })
        }

        AbilityEffect::ArmorPiercingStrike {
            pierce_fraction,
            bonus,
        } => {
            let target_entity = target?;
            if !world
                .get::<Health>(target_entity)
                .map(|h| h.is_alive())
                .unwrap_or(false)
            {
                return None;
            }
            world.get_mut::<ActionPoints>(actor)?.spend(ability.ap_cost);
            let (hit, _cover) = roll_ability_hit(world, actor, target_entity);
            let attack = world.get::<Stats>(actor).map(|s| s.attack).unwrap_or(0);
            let defense = world
                .get::<Stats>(target_entity)
                .map(|s| s.defense)
                .unwrap_or(0);
            let effective_defense = (defense as f32 * (1.0 - pierce_fraction)) as i32;
            let damage = if hit {
                calc_damage(attack, effective_defense) + bonus
            } else {
                0
            };
            if hit {
                world.get_mut::<Health>(target_entity)?.take_damage(damage);
            }
            Some(TurnAction::UseAbility {
                ability_name: ability.name,
                target,
                value: damage,
                hit,
            })
        }

        AbilityEffect::Heal { amount } => {
            let target_entity = target.unwrap_or(actor);
            world.get_mut::<ActionPoints>(actor)?.spend(ability.ap_cost);
            let restored = *amount;
            world.get_mut::<Health>(target_entity)?.heal(restored);
            Some(TurnAction::UseAbility {
                ability_name: ability.name,
                target: Some(target_entity),
                value: restored,
                hit: true,
            })
        }

        AbilityEffect::DrainAP { amount } => {
            let target_entity = target?;
            world.get_mut::<ActionPoints>(actor)?.spend(ability.ap_cost);
            let drained = (*amount).min(
                world
                    .get::<ActionPoints>(target_entity)
                    .map(|ap| ap.current)
                    .unwrap_or(0),
            );
            world.get_mut::<ActionPoints>(target_entity)?.spend(drained);
            Some(TurnAction::UseAbility {
                ability_name: ability.name,
                target,
                value: drained,
                hit: true,
            })
        }

        AbilityEffect::GrantAP { amount } => {
            world.get_mut::<ActionPoints>(actor)?.spend(ability.ap_cost);
            let granted = *amount;
            if let Some(mut ap) = world.get_mut::<ActionPoints>(actor) {
                ap.current += granted; // allow exceeding max for burst turns
            }
            Some(TurnAction::UseAbility {
                ability_name: ability.name,
                target: None,
                value: granted,
                hit: true,
            })
        }

        AbilityEffect::Displacement { delay_rounds } => {
            let target_entity = target?;
            if !world
                .get::<Health>(target_entity)
                .map(|h| h.is_alive())
                .unwrap_or(false)
            {
                return None;
            }
            world.get_mut::<ActionPoints>(actor)?.spend(ability.ap_cost);
            let (hit, _cover) = roll_ability_hit(world, actor, target_entity);
            if !hit {
                return Some(TurnAction::UseAbility {
                    ability_name: ability.name,
                    target,
                    value: 0,
                    hit: false,
                });
            }

            let attack = world.get::<Stats>(actor).map(|s| s.attack).unwrap_or(0);
            let defense = world
                .get::<Stats>(target_entity)
                .map(|s| s.defense)
                .unwrap_or(0);
            let damage = calc_damage(attack, defense) + 15;
            let resolve_round = current_round + *delay_rounds as u32;

            // Queue delayed damage.
            if let Some(mut effects) = world.get_resource_mut::<PendingEffects>() {
                effects.0.push(PendingEffect::DelayedDamage {
                    target: target_entity,
                    damage,
                    resolve_on_round: resolve_round,
                });
            }

            // Immediate pushback: push target away from actor.
            let actor_pos = world.get::<Position>(actor).copied();
            let target_pos = world.get::<Position>(target_entity).copied();
            let pushed_to = if let (Some(ap), Some(tp)) = (actor_pos, target_pos) {
                let push_dx = (tp.x - ap.x).signum();
                let push_dy = (tp.y - ap.y).signum();
                let occupied = occupied_tiles(world, target_entity);
                let (cols, rows) = world
                    .get_resource::<LevelMap>()
                    .map(|m| (m.cols as i32, m.rows as i32))
                    .unwrap_or((i32::MAX, i32::MAX));

                let mut dest = None;
                for dist in [2i32, 1i32] {
                    let tx = tp.x + push_dx * dist;
                    let ty = tp.y + push_dy * dist;
                    if tx < 0 || ty < 0 || tx >= cols || ty >= rows {
                        continue;
                    }
                    if occupied.contains(&(tx, ty)) {
                        continue;
                    }
                    let passable = world
                        .get_resource::<LevelMap>()
                        .map(|m| m.is_passable(tx, ty))
                        .unwrap_or(true);
                    if passable {
                        dest = Some(Position::new(tx, ty));
                        break;
                    }
                }

                if let Some(new_pos) = dest
                    && let Some(mut pos) = world.get_mut::<Position>(target_entity)
                {
                    *pos = new_pos;
                }
                dest
            } else {
                None
            };

            Some(TurnAction::DisplacementQueued {
                target: target_entity,
                pending_damage: damage,
                resolve_round,
                pushed_to,
            })
        }

        AbilityEffect::Acceleration {
            bonus_ap,
            drain_ap_next,
        } => {
            world.get_mut::<ActionPoints>(actor)?.spend(ability.ap_cost);
            // Grant bonus AP (allow exceeding max — burst turn).
            if let Some(mut ap) = world.get_mut::<ActionPoints>(actor) {
                ap.current += bonus_ap;
            }
            // Queue AP drain for next round.
            if let Some(mut effects) = world.get_resource_mut::<PendingEffects>() {
                effects.0.push(PendingEffect::DrainAP {
                    target: actor,
                    amount: *drain_ap_next,
                    resolve_on_round: current_round + 1,
                });
            }
            Some(TurnAction::AccelerationApplied {
                actor,
                bonus_ap: *bonus_ap,
            })
        }

        AbilityEffect::EntropicRounds => {
            let target_entity = target?;
            if !world
                .get::<Health>(target_entity)
                .map(|h| h.is_alive())
                .unwrap_or(false)
            {
                return None;
            }
            world.get_mut::<ActionPoints>(actor)?.spend(ability.ap_cost);
            // Unconditional hit, no defense, no cover.
            let damage = world
                .get::<Stats>(actor)
                .map(|s| s.attack)
                .unwrap_or(1)
                .max(1);
            world.get_mut::<Health>(target_entity)?.take_damage(damage);
            Some(TurnAction::UseAbility {
                ability_name: ability.name,
                target,
                value: damage,
                hit: true,
            })
        }

        AbilityEffect::EchoStrike => {
            let target_entity = target?;
            if !world
                .get::<Health>(target_entity)
                .map(|h| h.is_alive())
                .unwrap_or(false)
            {
                return None;
            }
            world.get_mut::<ActionPoints>(actor)?.spend(ability.ap_cost);

            // Check target's last used ability.
            let last_ability = world
                .get::<LastUsedAbility>(target_entity)
                .map(|l| Ability {
                    ap_cost: 0,
                    ..l.ability.clone()
                });

            let should_echo = last_ability
                .as_ref()
                .map(|a| !matches!(a.effect, AbilityEffect::EchoStrike))
                .unwrap_or(false);

            if should_echo {
                let echo_ability = last_ability.unwrap();
                // Recursive call with zero-cost copy of the echoed ability.
                apply_ability(world, actor, &echo_ability, Some(target_entity))
            } else {
                // Fallback: basic melee damage with normal hit roll.
                let (hit, _cover) = roll_ability_hit(world, actor, target_entity);
                let attack = world.get::<Stats>(actor).map(|s| s.attack).unwrap_or(0);
                let defense = world
                    .get::<Stats>(target_entity)
                    .map(|s| s.defense)
                    .unwrap_or(0);
                let damage = if hit { calc_damage(attack, defense) } else { 0 };
                if hit {
                    world.get_mut::<Health>(target_entity)?.take_damage(damage);
                }
                Some(TurnAction::UseAbility {
                    ability_name: ability.name,
                    target,
                    value: damage,
                    hit,
                })
            }
        }

        AbilityEffect::TemporalRecall => {
            let target_entity = target?;
            if !world
                .get::<Health>(target_entity)
                .map(|h| h.is_alive())
                .unwrap_or(false)
            {
                return None;
            }

            // Target must have moved (LastPosition must exist).
            let last_pos = match world.get::<LastPosition>(target_entity) {
                Some(lp) => lp.0,
                None => return None,
            };

            let from_pos = world.get::<Position>(target_entity).copied()?;

            // Validate destination: passable and unoccupied (excluding target itself).
            let occupied = occupied_tiles(world, target_entity);
            if occupied.contains(&(last_pos.x, last_pos.y)) {
                return None;
            }
            let passable = world
                .get_resource::<LevelMap>()
                .map(|m| m.is_passable(last_pos.x, last_pos.y))
                .unwrap_or(true);
            if !passable {
                return None;
            }

            world.get_mut::<ActionPoints>(actor)?.spend(ability.ap_cost);

            // Move target back to last position.
            if let Some(mut pos) = world.get_mut::<Position>(target_entity) {
                *pos = last_pos;
            }
            // Remove LastPosition so the target can only be recalled once per move.
            world.entity_mut(target_entity).remove::<LastPosition>();

            Some(TurnAction::TemporalRecalled {
                target: target_entity,
                from: from_pos,
                to: last_pos,
            })
        }
    };

    // Record the ability used by the actor (for EchoStrike targeting).
    if result.is_some() {
        world.entity_mut(actor).insert(LastUsedAbility {
            ability: ability.clone(),
        });
    }

    result
}

/// Rolls a hit check for an ability (uses BattleRng if present; otherwise always hits).
/// Applies a hit penalty when TemporalFlux is above the high-flux threshold.
fn roll_ability_hit(world: &mut World, actor: Entity, target: Entity) -> (bool, CoverLevel) {
    if world.get_resource::<BattleRng>().is_some() {
        world.resource_scope(|world, mut rng: Mut<BattleRng>| {
            let attacker_pos = world.get::<Position>(actor).copied();
            let defender_pos = world.get::<Position>(target).copied();
            let cover = match (attacker_pos, defender_pos) {
                (Some(ap), Some(dp)) => {
                    let dir = Direction::from_attack((ap.x, ap.y), (dp.x, dp.y));
                    world
                        .get_resource::<LevelMap>()
                        .map(|m| m.get_cover(dp.x, dp.y, dir))
                        .unwrap_or(CoverLevel::None)
                }
                _ => CoverLevel::None,
            };
            let base_chance = calc_hit_chance(cover);
            let flux_penalty = world
                .get_resource::<TemporalFlux>()
                .map(|f| f.hit_penalty())
                .unwrap_or(0.0);
            let adjusted_chance = (base_chance - flux_penalty).max(0.05);
            let hit = roll_hit(adjusted_chance, &mut rng.0);
            (hit, cover)
        })
    } else {
        (true, CoverLevel::None)
    }
}
