use crate::character::Character;
use crate::character::CharacterKind;
use crate::position::Position;
use crate::terrain::{LevelMap, generate_map};
use crate::zone::ZoneKind;
use bevy::prelude::Resource;
use rand::Rng;
use std::collections::HashSet;

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
    /// The zone this level was generated for.
    pub zone_kind: ZoneKind,
    /// Each NPC paired with its starting grid position (z = 0).
    pub enemies: Vec<(Character, Position)>,
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
        let zone_kind = random_zone_kind(rng);
        Self::generate_inner(depth, zone_kind, None, rng)
    }

    /// Generate a level for a specific zone with its enemy pool.
    pub fn generate_for_zone(
        depth: u32,
        zone_kind: ZoneKind,
        enemy_pool: &[CharacterKind],
        rng: &mut impl Rng,
    ) -> Self {
        Self::generate_inner(depth, zone_kind, Some(enemy_pool), rng)
    }

    fn generate_inner(
        depth: u32,
        zone_kind: ZoneKind,
        enemy_pool: Option<&[CharacterKind]>,
        rng: &mut impl Rng,
    ) -> Self {
        let cols: u32 = rng.gen_range(8..=16);
        let rows: u32 = rng.gen_range(8..=16);
        let enemy_level = depth.max(1);
        let enemy_count: usize = rng.gen_range(1..=4);

        let mut used: HashSet<(i32, i32)> = HashSet::new();
        let mut enemies = Vec::with_capacity(enemy_count);
        while enemies.len() < enemy_count {
            let x = rng.gen_range(0..cols as i32);
            let y = rng.gen_range(0..rows as i32);
            if used.insert((x, y)) {
                let kind = match enemy_pool {
                    Some(pool) => pool[rng.gen_range(0..pool.len())].clone(),
                    None => random_enemy_kind(rng),
                };
                enemies.push((
                    Character::new_character(kind, enemy_level),
                    Position::new(x, y, 0),
                ));
            }
        }

        let reserved: Vec<(i32, i32)> = enemies.iter().map(|(_, p)| (p.x, p.y)).collect();
        let map = generate_map(cols, rows, zone_kind, &reserved, rng);

        let surprise = match rng.gen_range(0..4u32) {
            0 => SurpriseState::PartyAmbushed,
            1 => SurpriseState::EnemyAmbushed,
            _ => SurpriseState::Normal,
        };

        Self {
            depth,
            cols,
            rows,
            zone_kind,
            enemies,
            map,
            surprise,
        }
    }
}

fn random_zone_kind(rng: &mut impl Rng) -> ZoneKind {
    match rng.gen_range(0..9u32) {
        0 => ZoneKind::ResearchWing,
        1 => ZoneKind::CommandDeck,
        2 => ZoneKind::MilitaryAnnex,
        3 => ZoneKind::SystemsCore,
        4 => ZoneKind::MedicalBay,
        5 => ZoneKind::DockingBay,
        6 => ZoneKind::StationExterior,
        7 => ZoneKind::RelayArray,
        _ => ZoneKind::ExcavationSite,
    }
}

fn random_enemy_kind(rng: &mut impl Rng) -> CharacterKind {
    match rng.gen_range(0..13u32) {
        0 => CharacterKind::Scavenger,
        1 => CharacterKind::VoidRaider,
        2 => CharacterKind::DrifterBoss,
        3 => CharacterKind::MaintenanceDrone,
        4 => CharacterKind::SecurityUnit,
        5 => CharacterKind::CombatFrame,
        6 => CharacterKind::MoonCrawler,
        7 => CharacterKind::VoidSpitter,
        8 => CharacterKind::AbyssalBrute,
        9 => CharacterKind::SalvageOperative,
        10 => CharacterKind::GunForHire,
        11 => CharacterKind::StationGuard,
        _ => CharacterKind::ShockTrooper,
    }
}
