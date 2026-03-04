use std::collections::{HashMap, HashSet};

use bevy::prelude::Resource;
use rand::Rng;
use rand::rngs::StdRng;

/// The visual and mechanical theme of a generated level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Biome {
    /// Derelict space station: corridors, wall panels, consoles.
    VoidStation,
    /// Neon-lit cyberpunk streets: hologram pillars, vendor barriers.
    NeonDistrict,
    /// Abandoned research facility: containment tanks, lab benches.
    BioLab,
    /// Off-world mining colony: rock formations, ore deposits, machinery.
    AsteroidColony,
}

impl Biome {
    pub fn display_name(&self) -> &'static str {
        match self {
            Biome::VoidStation => "Void Station",
            Biome::NeonDistrict => "Neon District",
            Biome::BioLab => "Bio Lab",
            Biome::AsteroidColony => "Asteroid Colony",
        }
    }
}

/// The terrain type of a single map cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    /// Open ground — passable, no cover benefit.
    Open,
    /// Impassable solid structure — blocks movement.
    Obstacle,
    /// Low cover (crates, debris) — passable; hit chance reduced to ~65%.
    PartialCover,
    /// Heavy cover (walls, bulkheads) — passable; hit chance reduced to ~35%.
    FullCover,
}

impl Tile {
    /// Returns `false` only for `Obstacle`; everything else can be stood on.
    pub fn is_passable(self) -> bool {
        !matches!(self, Tile::Obstacle)
    }

    /// Terminal character used in the map display.
    pub fn glyph(self) -> char {
        match self {
            Tile::Open => '.',
            Tile::Obstacle => '#',
            Tile::PartialCover => 'c',
            Tile::FullCover => 'C',
        }
    }
}

/// The 2-D terrain grid for one level.
#[derive(Debug, Resource)]
pub struct LevelMap {
    pub cols: u32,
    pub rows: u32,
    pub biome: Biome,
    tiles: HashMap<(i32, i32), Tile>,
}

impl LevelMap {
    pub fn new(cols: u32, rows: u32, biome: Biome) -> Self {
        Self { cols, rows, biome, tiles: HashMap::new() }
    }

    /// Returns the tile at `(x, y)`, defaulting to `Open` for unknown cells.
    pub fn get(&self, x: i32, y: i32) -> Tile {
        self.tiles.get(&(x, y)).copied().unwrap_or(Tile::Open)
    }

    pub fn set(&mut self, x: i32, y: i32, tile: Tile) {
        self.tiles.insert((x, y), tile);
    }

    pub fn is_passable(&self, x: i32, y: i32) -> bool {
        self.get(x, y).is_passable()
    }
}

/// Battle RNG resource — wraps a seeded RNG so it can be stored on the `World`.
#[derive(Resource)]
pub struct BattleRng(pub StdRng);

// ── Biome densities ──────────────────────────────────────────────────────────

struct BiomeDensity {
    obstacle: f32,
    partial_cover: f32,
    full_cover: f32,
}

fn biome_density(biome: Biome) -> BiomeDensity {
    match biome {
        Biome::VoidStation => BiomeDensity {
            obstacle: 0.12,
            partial_cover: 0.08,
            full_cover: 0.04,
        },
        Biome::NeonDistrict => BiomeDensity {
            obstacle: 0.08,
            partial_cover: 0.15,
            full_cover: 0.05,
        },
        Biome::BioLab => BiomeDensity {
            obstacle: 0.06,
            partial_cover: 0.06,
            full_cover: 0.14,
        },
        Biome::AsteroidColony => BiomeDensity {
            obstacle: 0.18,
            partial_cover: 0.10,
            full_cover: 0.03,
        },
    }
}

// ── Map generation ───────────────────────────────────────────────────────────

/// Procedurally generate a terrain map for the given biome.
///
/// `reserved_open` lists grid positions that must remain `Open` (e.g. spawn tiles).
pub fn generate_map(
    cols: u32,
    rows: u32,
    biome: Biome,
    reserved_open: &[(i32, i32)],
    rng: &mut impl Rng,
) -> LevelMap {
    let mut map = LevelMap::new(cols, rows, biome);
    let density = biome_density(biome);
    let reserved: HashSet<(i32, i32)> = reserved_open.iter().copied().collect();

    let obstacle_threshold = density.obstacle;
    let partial_threshold = obstacle_threshold + density.partial_cover;
    let full_threshold = partial_threshold + density.full_cover;

    for y in 0..rows as i32 {
        for x in 0..cols as i32 {
            if reserved.contains(&(x, y)) {
                continue; // keep as Open
            }
            let roll: f32 = rng.r#gen();
            if roll < obstacle_threshold {
                map.set(x, y, Tile::Obstacle);
            } else if roll < partial_threshold {
                map.set(x, y, Tile::PartialCover);
            } else if roll < full_threshold {
                map.set(x, y, Tile::FullCover);
            }
            // else: Open (default, no entry needed)
        }
    }

    map
}
