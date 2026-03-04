use std::collections::HashSet;
use bevy::prelude::Resource;
use rand::Rng;
use crate::enemy::{Enemy, EnemyKind};
use crate::position::Position;
use crate::terrain::{Biome, LevelMap, generate_map};

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
    /// Grid width (number of columns along the X axis).
    pub cols: u32,
    /// Grid height (number of rows along the Y axis).
    pub rows: u32,
    /// The scifi biome theme for this level.
    pub biome: Biome,
    /// Each enemy paired with its starting grid position (z = 0).
    pub enemies: Vec<(Enemy, Position)>,
    /// The terrain map for this level.
    pub map: LevelMap,
    pub surprise: SurpriseState,
}

impl Level {
    /// Randomly generate a level at the given depth using the provided RNG.
    /// Grid: 8–16 columns × 8–16 rows. Enemy count: 1–4 at unique positions.
    /// Enemy level equals `depth` (minimum 1).
    /// Surprise: 25% party ambushed, 25% enemy ambushed, 50% normal.
    pub fn generate(depth: u32, rng: &mut impl Rng) -> Self {
        let cols: u32 = rng.gen_range(8..=16);
        let rows: u32 = rng.gen_range(8..=16);
        let biome = random_biome(rng);
        let enemy_level = depth.max(1);
        let enemy_count: usize = rng.gen_range(1..=4);

        let mut used: HashSet<(i32, i32)> = HashSet::new();
        let mut enemies = Vec::with_capacity(enemy_count);
        while enemies.len() < enemy_count {
            let x = rng.gen_range(0..cols as i32);
            let y = rng.gen_range(0..rows as i32);
            if used.insert((x, y)) {
                let kind = random_enemy_kind(rng);
                enemies.push((Enemy::new(kind, enemy_level), Position::new(x, y, 0)));
            }
        }

        let reserved: Vec<(i32, i32)> = enemies.iter().map(|(_, p)| (p.x, p.y)).collect();
        let map = generate_map(cols, rows, biome, &reserved, rng);

        let surprise = match rng.gen_range(0..4u32) {
            0 => SurpriseState::PartyAmbushed,
            1 => SurpriseState::EnemyAmbushed,
            _ => SurpriseState::Normal,
        };

        Self { depth, cols, rows, biome, enemies, map, surprise }
    }
}

fn random_biome(rng: &mut impl Rng) -> Biome {
    match rng.gen_range(0..4u32) {
        0 => Biome::VoidStation,
        1 => Biome::NeonDistrict,
        2 => Biome::BioLab,
        _ => Biome::AsteroidColony,
    }
}

fn random_enemy_kind(rng: &mut impl Rng) -> EnemyKind {
    match rng.gen_range(0..13u32) {
        0  => EnemyKind::Scavenger,
        1  => EnemyKind::VoidRaider,
        2  => EnemyKind::DrifterBoss,
        3  => EnemyKind::MaintenanceDrone,
        4  => EnemyKind::SecurityUnit,
        5  => EnemyKind::CombatFrame,
        6  => EnemyKind::MoonCrawler,
        7  => EnemyKind::VoidSpitter,
        8  => EnemyKind::AbyssalBrute,
        9  => EnemyKind::SalvageOperative,
        10 => EnemyKind::GunForHire,
        11 => EnemyKind::StationGuard,
        _  => EnemyKind::ShockTrooper,
    }
}
