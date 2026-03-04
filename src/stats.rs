use bevy::prelude::Component;
use crate::character::CharacterClass;

#[derive(Debug, Clone, Component)]
pub struct Stats {
    pub max_hp: i32,
    pub attack: i32,
    pub defense: i32,
    pub speed: i32,
    /// Used by Mage/Cleric for spell damage/healing
    pub magic: i32,
}

impl Stats {
    pub fn for_class(class: &CharacterClass) -> Self {
        match class {
            CharacterClass::Warrior => Self { max_hp: 120, attack: 15, defense: 12, speed: 8,  magic: 2  },
            CharacterClass::Mage    => Self { max_hp: 60,  attack: 5,  defense: 4,  speed: 10, magic: 20 },
            CharacterClass::Rogue   => Self { max_hp: 80,  attack: 14, defense: 6,  speed: 18, magic: 4  },
            CharacterClass::Cleric  => Self { max_hp: 90,  attack: 8,  defense: 8,  speed: 9,  magic: 14 },
            CharacterClass::Ranger  => Self { max_hp: 85,  attack: 12, defense: 7,  speed: 15, magic: 5  },
        }
    }

    /// Apply growth on level-up. Values are per-level increases.
    pub fn level_up(&mut self, class: &CharacterClass) {
        let (hp, atk, def, spd, mag) = match class {
            CharacterClass::Warrior => (18, 3, 3, 1, 0),
            CharacterClass::Mage    => (8,  1, 1, 2, 4),
            CharacterClass::Rogue   => (10, 3, 1, 3, 0),
            CharacterClass::Cleric  => (12, 1, 2, 1, 3),
            CharacterClass::Ranger  => (11, 2, 1, 2, 1),
        };
        self.max_hp  += hp;
        self.attack  += atk;
        self.defense += def;
        self.speed   += spd;
        self.magic   += mag;
    }
}
