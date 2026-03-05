use crate::character::PlayerCharacter;
use bevy::prelude::Component;

#[derive(Debug, Clone, Component)]
pub struct Stats {
    pub max_hp: i32,
    pub attack: i32,
    pub defense: i32,
    pub speed: i32,
}

impl Stats {
    pub fn for_character(character: &PlayerCharacter) -> Self {
        match character {
            PlayerCharacter::Researcher => Self {
                max_hp: 75,
                attack: 9,
                defense: 4,
                speed: 16,
            },
            PlayerCharacter::Orin => Self {
                max_hp: 90,
                attack: 8,
                defense: 8,
                speed: 9,
            },
            PlayerCharacter::Doss => Self {
                max_hp: 120,
                attack: 15,
                defense: 12,
                speed: 8,
            },
            PlayerCharacter::Kaleo => Self {
                max_hp: 85,
                attack: 12,
                defense: 7,
                speed: 15,
            },
        }
    }

    /// Apply growth on level-up. Values are per-level increases.
    pub fn level_up(&mut self, character: &PlayerCharacter) {
        let (hp, atk, def, spd) = match character {
            PlayerCharacter::Researcher => (9, 2, 1, 2),
            PlayerCharacter::Orin => (12, 1, 2, 1),
            PlayerCharacter::Doss => (18, 3, 3, 1),
            PlayerCharacter::Kaleo => (11, 2, 1, 2),
        };
        self.max_hp += hp;
        self.attack += atk;
        self.defense += def;
        self.speed += spd;
    }
}
