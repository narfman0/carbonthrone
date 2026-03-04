use bevy::prelude::*;

use crate::{
    action_points::ActionPoints,
    combat::calc_damage,
    health::Health,
    position::Position,
    stats::Stats,
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

/// Execute an action for `actor`. Returns `true` if the action was applied.
pub fn apply_action(world: &mut World, actor: Entity, action: &Action) -> bool {
    match action {
        Action::Attack { target } => {
            let ap = world.get::<ActionPoints>(actor).map(|ap| ap.current).unwrap_or(0);
            if ap < ATTACK_AP_COST {
                return false;
            }
            if !world.get::<Health>(*target).map(|h| h.is_alive()).unwrap_or(false) {
                return false;
            }
            let attack = world.get::<Stats>(actor).map(|s| s.attack).unwrap_or(0);
            let defense = world.get::<Stats>(*target).map(|s| s.defense).unwrap_or(0);
            let damage = calc_damage(attack, defense);
            world.get_mut::<ActionPoints>(actor).unwrap().spend(ATTACK_AP_COST);
            world.get_mut::<Health>(*target).unwrap().take_damage(damage);
            true
        }
        Action::Move { destination } => {
            let current = match world.get::<Position>(actor) {
                Some(p) => *p,
                None => return false,
            };
            let distance =
                (destination.x - current.x).abs() + (destination.y - current.y).abs();
            if distance == 0 {
                return false;
            }
            let cost = MOVE_AP_COST * distance;
            let ap = world.get::<ActionPoints>(actor).map(|ap| ap.current).unwrap_or(0);
            if ap < cost {
                return false;
            }
            world.get_mut::<ActionPoints>(actor).unwrap().spend(cost);
            if let Some(mut p) = world.get_mut::<Position>(actor) {
                *p = *destination;
            }
            true
        }
        Action::Pass => true,
    }
}
