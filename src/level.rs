use bevy::prelude::Resource;
use rand::Rng;
use crate::enemy::{Enemy, EnemyKind};

/// Who, if anyone, has the initiative advantage at the start of an encounter.
#[derive(Debug, Clone, PartialEq)]
pub enum SurpriseState {
    /// Turn order is determined normally by speed stats.
    Normal,
    /// Enemies detected the party first — enemies act before the party this encounter.
    PartyAmbushed,
    /// The party detected the enemies first — party acts before enemies this encounter.
    EnemyAmbushed,
}

/// A single procedurally generated encounter level.
#[derive(Debug, Resource)]
pub struct Level {
    /// Depth in the dungeon; drives enemy levels and difficulty.
    pub depth: u32,
    pub enemies: Vec<Enemy>,
    pub surprise: SurpriseState,
}

impl Level {
    /// Randomly generate a level at the given depth using the provided RNG.
    /// Enemy count: 1–4. Enemy level equals `depth` (minimum 1).
    /// Surprise: 25% party ambushed, 25% enemy ambushed, 50% normal.
    pub fn generate(depth: u32, rng: &mut impl Rng) -> Self {
        let enemy_count: usize = rng.gen_range(1..=4);
        let enemy_level = depth.max(1);
        let enemies = (0..enemy_count)
            .map(|_| Enemy::new(random_enemy_kind(rng), enemy_level))
            .collect();

        let surprise = match rng.gen_range(0..4u32) {
            0 => SurpriseState::PartyAmbushed,
            1 => SurpriseState::EnemyAmbushed,
            _ => SurpriseState::Normal,
        };

        Self { depth, enemies, surprise }
    }
}

fn random_enemy_kind(rng: &mut impl Rng) -> EnemyKind {
    match rng.gen_range(0..5u32) {
        0 => EnemyKind::Goblin,
        1 => EnemyKind::Skeleton,
        2 => EnemyKind::Orc,
        3 => EnemyKind::Troll,
        _ => EnemyKind::Dragon,
    }
}
