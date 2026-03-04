use bevy::prelude::*;

use crate::{
    action_points::ActionPoints,
    combat::{calc_damage, calc_hit_chance, roll_hit},
    health::Health,
    position::Position,
    stats::Stats,
    terrain::{BattleRng, LevelMap, Tile},
};

pub const ATTACK_AP_COST: i32 = 2;
pub const MOVE_AP_COST: i32 = 1;

/// An action a combatant can take on their turn, each costing AP.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Attack a target entity. Costs `ATTACK_AP_COST` AP.
    Attack { target: Entity },
    /// Move to a destination. Costs `MOVE_AP_COST` × Manhattan distance AP.
    Move { destination: Position },
    /// End the turn without spending AP.
    Pass,
}

/// One action that occurred during a combatant's turn (for the event log).
#[derive(Debug, Clone)]
pub enum TurnAction {
    /// An attack was attempted. `hit` indicates whether it connected.
    Attack { target: Entity, damage: i32, hit: bool },
    Move { to: Position },
}

/// Execute an action for `actor`.
/// Returns `Some(TurnAction)` if the action was carried out, `None` if it
/// was invalid (insufficient AP, dead target, blocked tile, etc.) or was Pass.
pub fn apply_action(world: &mut World, actor: Entity, action: &Action) -> Option<TurnAction> {
    match action {
        Action::Attack { target } => {
            let ap = world.get::<ActionPoints>(actor).map(|ap| ap.current).unwrap_or(0);
            if ap < ATTACK_AP_COST {
                return None;
            }
            if !world.get::<Health>(*target).map(|h| h.is_alive()).unwrap_or(false) {
                return None;
            }

            // Determine hit chance from defender's cover tile.
            // resource_scope temporarily removes BattleRng so we can access &mut World + rng.
            let hit = if world.get_resource::<BattleRng>().is_some() {
                world.resource_scope(|world, mut rng: Mut<BattleRng>| {
                    let tile = world
                        .get::<Position>(*target)
                        .copied()
                        .and_then(|p| world.get_resource::<LevelMap>().map(|m| m.get(p.x, p.y)))
                        .unwrap_or(Tile::Open);
                    roll_hit(calc_hit_chance(tile), &mut rng.0)
                })
            } else {
                true // no RNG resource → always hit (e.g. in simple unit tests)
            };

            let attack = world.get::<Stats>(actor).map(|s| s.attack).unwrap_or(0);
            let defense = world.get::<Stats>(*target).map(|s| s.defense).unwrap_or(0);
            let damage = if hit { calc_damage(attack, defense) } else { 0 };

            world.get_mut::<ActionPoints>(actor).unwrap().spend(ATTACK_AP_COST);
            if hit {
                world.get_mut::<Health>(*target).unwrap().take_damage(damage);
            }

            Some(TurnAction::Attack { target: *target, damage, hit })
        }
        Action::Move { destination } => {
            let current = match world.get::<Position>(actor) {
                Some(p) => *p,
                None => return None,
            };
            let distance =
                (destination.x - current.x).abs() + (destination.y - current.y).abs();
            if distance == 0 {
                return None;
            }
            let cost = MOVE_AP_COST * distance;
            let ap = world.get::<ActionPoints>(actor).map(|ap| ap.current).unwrap_or(0);
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
        Action::Pass => None,
    }
}
