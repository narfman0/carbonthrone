use bevy::prelude::*;

use crate::{
    ability::{Ability, AbilityEffect, AbilityKind},
    action_points::ActionPoints,
    combat::{calc_damage, calc_hit_chance, roll_hit},
    health::Health,
    position::Position,
    stats::Stats,
    terrain::{BattleRng, CoverLevel, Direction, LevelMap},
};

pub const MOVE_AP_COST: i32 = 1;

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
}

/// Execute an action for `actor`.
/// Returns `Some(TurnAction)` if the action was carried out, `None` if it
/// was invalid (insufficient AP, dead target, blocked tile, out of melee range, etc.) or was Pass.
pub fn apply_action(world: &mut World, actor: Entity, action: &Action) -> Option<TurnAction> {
    match action {
        Action::Move { destination } => {
            let current = match world.get::<Position>(actor) {
                Some(p) => *p,
                None => return None,
            };
            let distance = (destination.x - current.x).abs() + (destination.y - current.y).abs();
            if distance == 0 {
                return None;
            }
            let cost = MOVE_AP_COST * distance;
            let ap = world
                .get::<ActionPoints>(actor)
                .map(|ap| ap.current)
                .unwrap_or(0);
            if ap < cost {
                return None;
            }

            // Block movement onto obstacle tiles.
            if let Some(map) = world.get_resource::<LevelMap>()
                && !map.is_passable(destination.x, destination.y)
            {
                return None;
            }

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
    if ability.kind == AbilityKind::Melee {
        if let Some(target_entity) = target {
            if let (Some(actor_pos), Some(target_pos)) = (
                world.get::<Position>(actor).copied(),
                world.get::<Position>(target_entity).copied(),
            ) {
                let chebyshev = (actor_pos.x - target_pos.x)
                    .abs()
                    .max((actor_pos.y - target_pos.y).abs());
                if chebyshev > 1 {
                    return None;
                }
            }
        }
    }

    match &ability.effect {
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
    }
}

/// Rolls a hit check for an ability (uses BattleRng if present; otherwise always hits).
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
            let hit = roll_hit(calc_hit_chance(cover), &mut rng.0);
            (hit, cover)
        })
    } else {
        (true, CoverLevel::None)
    }
}
