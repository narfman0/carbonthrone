use crate::npc::NPCKind;
use bevy::prelude::Component;

#[derive(Debug, Clone, Component)]
pub struct Stats {
    pub max_hp: i32,
    pub attack: i32,
    pub defense: i32,
    pub speed: i32,
}

impl Stats {
    pub fn for_character(kind: &NPCKind) -> Self {
        match kind {
            NPCKind::Researcher => Self {
                max_hp: 75,
                attack: 9,
                defense: 4,
                speed: 16,
            },
            NPCKind::Orin => Self {
                max_hp: 90,
                attack: 8,
                defense: 8,
                speed: 9,
            },
            NPCKind::Doss => Self {
                max_hp: 120,
                attack: 15,
                defense: 12,
                speed: 8,
            },
            NPCKind::Kaleo => Self {
                max_hp: 85,
                attack: 12,
                defense: 7,
                speed: 15,
            },
            _ => panic!("Stats::for_character called on NPC kind {:?}", kind),
        }
    }

    /// Apply growth on level-up. Values are per-level increases.
    pub fn level_up(&mut self, kind: &NPCKind) {
        let (hp, atk, def, spd) = match kind {
            NPCKind::Researcher => (9, 2, 1, 2),
            NPCKind::Orin => (12, 1, 2, 1),
            NPCKind::Doss => (18, 3, 3, 1),
            NPCKind::Kaleo => (11, 2, 1, 2),
            _ => panic!("Stats::level_up called on NPC kind {:?}", kind),
        };
        self.max_hp += hp;
        self.attack += atk;
        self.defense += def;
        self.speed += spd;
    }
}
