use bevy::prelude::Component;

/// The named player characters, each with distinct stats and abilities.
#[derive(Debug, Clone, PartialEq, Component)]
pub enum PlayerCharacter {
    /// The player character — Temporal Mage; high magic, fragile, fast.
    Researcher,
    /// Dr. Sable Orin — Cleric; support healer, high defense, low offense.
    Orin,
    /// Recruiter Doss — Warrior; heavy armor, high HP, strong melee.
    Doss,
    /// Unit Kaleo — Ranger; precision attacks, system hacking, very fast.
    Kaleo,
}

/// Identity component for player characters.
/// Pair with Health, Stats, ActionPoints, and Experience on the same entity.
#[derive(Debug, Clone, Component)]
pub struct Character {
    pub name: String,
    pub character: PlayerCharacter,
}

impl Character {
    pub fn new(name: impl Into<String>, character: PlayerCharacter) -> Self {
        Self {
            name: name.into(),
            character,
        }
    }
}
