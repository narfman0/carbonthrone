use crate::character::{CharacterKind, growth};
use bevy::prelude::Component;

#[derive(Debug, Clone, Component)]
pub struct Stats {
    pub max_hp: i32,
    pub attack: i32,
    pub defense: i32,
    pub speed: i32,
}

impl Stats {
    pub fn for_character(kind: &CharacterKind) -> Self {
        crate::character::base_stats(kind)
    }

    /// Apply growth on level-up. Values are per-level increases.
    pub fn level_up(&mut self, kind: &CharacterKind) {
        let (hp, atk, def, spd) = growth(kind);
        self.max_hp += hp;
        self.attack += atk;
        self.defense += def;
        self.speed += spd;
    }
}
