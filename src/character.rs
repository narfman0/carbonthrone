use bevy::prelude::Component;

/// The class of a player character, driving base stats and level-up growth.
#[derive(Debug, Clone, PartialEq, Component)]
pub enum CharacterClass {
    Warrior,
    Rogue,
    Cleric,
    Ranger,
}

/// Identity component for player characters.
/// Pair with Health, Stats, ActionPoints, and Experience on the same entity.
#[derive(Debug, Clone, Component)]
pub struct Character {
    pub name: String,
    pub class: CharacterClass,
}

impl Character {
    pub fn new(name: impl Into<String>, class: CharacterClass) -> Self {
        Self { name: name.into(), class }
    }
}
