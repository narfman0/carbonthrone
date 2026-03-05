use crate::character::CharacterClass;
use bevy::prelude::Component;

#[derive(Debug, Clone, Component)]
pub struct Stats {
    pub max_hp: i32,
    pub attack: i32,
    pub defense: i32,
    pub speed: i32,
}

impl Stats {
    pub fn for_class(class: &CharacterClass) -> Self {
        match class {
            CharacterClass::Warrior => Self {
                max_hp: 120,
                attack: 15,
                defense: 12,
                speed: 8,
            },
            CharacterClass::Rogue => Self {
                max_hp: 80,
                attack: 14,
                defense: 6,
                speed: 18,
            },
            CharacterClass::Cleric => Self {
                max_hp: 90,
                attack: 8,
                defense: 8,
                speed: 9,
            },
            CharacterClass::Ranger => Self {
                max_hp: 85,
                attack: 12,
                defense: 7,
                speed: 15,
            },
        }
    }

    /// Apply growth on level-up. Values are per-level increases.
    pub fn level_up(&mut self, class: &CharacterClass) {
        let (hp, atk, def, spd) = match class {
            CharacterClass::Warrior => (18, 3, 3, 1),
            CharacterClass::Rogue => (10, 3, 1, 3),
            CharacterClass::Cleric => (12, 1, 2, 1),
            CharacterClass::Ranger => (11, 2, 1, 2),
        };
        self.max_hp += hp;
        self.attack += atk;
        self.defense += def;
        self.speed += spd;
    }
}
