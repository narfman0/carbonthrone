use bevy::prelude::*;

/// Identifies which team an entity belongs to in combat.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub enum Side {
    Player,
    Enemy,
}
