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

/// The terrain type of a single map cell. Tiles are either passable or not;
/// cover is derived from adjacency to obstacles, not stored on the tile itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    /// Open ground — passable, no inherent cover.
    Open,
    /// Impassable solid structure — blocks movement and provides adjacent cover.
    Obstacle,
}

impl Tile {
    pub fn is_passable(self) -> bool {
        !matches!(self, Tile::Obstacle)
    }
}

/// How much protection a tile's position provides from a given attack direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoverLevel {
    /// No adjacent obstacle — attacker has normal hit chance.
    None,
    /// Diagonal obstacle — hit chance reduced to ~65%.
    Partial,
    /// Direct adjacent obstacle — hit chance reduced to ~35%.
    Full,
}

/// A cardinal direction on the grid (y increases southward).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    North = 0,
    South = 1,
    East = 2,
    West = 3,
}

impl Direction {
    /// Returns the direction the attacker is approaching FROM (from the defender's
    /// perspective). Uses the dominant axis — |dx| ≥ |dy| picks East/West.
    pub fn from_attack(attacker: (i32, i32), defender: (i32, i32)) -> Self {
        let dx = attacker.0 - defender.0;
        let dy = attacker.1 - defender.1;
        if dx.abs() >= dy.abs() {
            if dx >= 0 { Direction::East } else { Direction::West }
        } else if dy >= 0 { Direction::South } else { Direction::North }
    }
}

/// Cover levels for each of the four cardinal directions for a single tile.
#[derive(Debug, Clone, Copy)]
pub struct DirectionalCover([CoverLevel; 4]);

impl DirectionalCover {
    pub const NONE: Self = DirectionalCover([CoverLevel::None; 4]);

    pub fn get(self, dir: Direction) -> CoverLevel {
        self.0[dir as usize]
    }

    fn any_nonzero(self) -> bool {
        self.0.iter().any(|&c| c != CoverLevel::None)
    }
}

/// The 2-D terrain grid for one level, including precomputed directional cover.
#[derive(Debug, Resource)]
pub struct LevelMap {
    pub cols: u32,
    pub rows: u32,
    pub biome: Biome,
    tiles: HashMap<(i32, i32), Tile>,
    cover: HashMap<(i32, i32), DirectionalCover>,
}

impl LevelMap {
    pub fn new(cols: u32, rows: u32, biome: Biome) -> Self {
        Self { cols, rows, biome, tiles: HashMap::new(), cover: HashMap::new() }
    }

    /// Returns the tile at `(x, y)`, defaulting to `Open`.
    pub fn get(&self, x: i32, y: i32) -> Tile {
        self.tiles.get(&(x, y)).copied().unwrap_or(Tile::Open)
    }

    pub fn set(&mut self, x: i32, y: i32, tile: Tile) {
        self.tiles.insert((x, y), tile);
    }

    pub fn is_passable(&self, x: i32, y: i32) -> bool {
        self.get(x, y).is_passable()
    }

    /// Returns the cover level at `(x, y)` from the given `from` direction.
    pub fn get_cover(&self, x: i32, y: i32, from: Direction) -> CoverLevel {
        self.cover.get(&(x, y)).map(|dc| dc.get(from)).unwrap_or(CoverLevel::None)
    }

    /// Recomputes directional cover for all passable tiles from the current obstacle layout.
    /// Call this after manually setting tiles (e.g. in tests) to keep cover data consistent.
    pub fn recompute_cover(&mut self) {
        self.cover.clear();
        let passable: Vec<(i32, i32)> = (0..self.rows as i32)
            .flat_map(|y| (0..self.cols as i32).map(move |x| (x, y)))
            .filter(|&(x, y)| self.get(x, y) == Tile::Open)
            .collect();
        for (x, y) in passable {
            let dc = compute_directional_cover(&self.tiles, x, y);
            if dc.any_nonzero() {
                self.cover.insert((x, y), dc);
            }
        }
    }

    /// Terminal character for rendering, including cover hints:
    /// `'#'` obstacle, `'C'` full cover any direction, `'c'` partial, `'.'` open.
    pub fn display_glyph(&self, x: i32, y: i32) -> char {
        if self.get(x, y) == Tile::Obstacle {
            return '#';
        }
        let dirs = [Direction::North, Direction::South, Direction::East, Direction::West];
        if dirs.iter().any(|&d| self.get_cover(x, y, d) == CoverLevel::Full) {
            'C'
        } else if dirs.iter().any(|&d| self.get_cover(x, y, d) == CoverLevel::Partial) {
            'c'
        } else {
            '.'
        }
    }
}

/// Battle RNG resource — wraps a seeded RNG so it can be stored on the `World`.
#[derive(Resource)]
pub struct BattleRng(pub StdRng);

// ── Biome densities ──────────────────────────────────────────────────────────

struct BiomeDensity {
    obstacle: f32,
}

fn biome_density(biome: Biome) -> BiomeDensity {
    match biome {
        Biome::VoidStation    => BiomeDensity { obstacle: 0.15 },
        Biome::NeonDistrict   => BiomeDensity { obstacle: 0.10 },
        Biome::BioLab         => BiomeDensity { obstacle: 0.08 },
        Biome::AsteroidColony => BiomeDensity { obstacle: 0.22 },
    }
}

// ── Map generation ───────────────────────────────────────────────────────────

/// Procedurally generate a terrain map for the given biome.
///
/// Obstacles are placed randomly; directional cover is then computed for every
/// passable tile based on its obstacle neighbors.
/// `reserved_open` lists positions that must remain passable (e.g. spawn tiles).
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

    // Place obstacles.
    for y in 0..rows as i32 {
        for x in 0..cols as i32 {
            if reserved.contains(&(x, y)) {
                continue;
            }
            if rng.r#gen::<f32>() < density.obstacle {
                map.tiles.insert((x, y), Tile::Obstacle);
            }
        }
    }

    // Compute directional cover for all passable tiles from obstacle adjacency.
    let passable: Vec<(i32, i32)> = (0..rows as i32)
        .flat_map(|y| (0..cols as i32).map(move |x| (x, y)))
        .filter(|&(x, y)| map.get(x, y) == Tile::Open)
        .collect();

    for (x, y) in passable {
        let dc = compute_directional_cover(&map.tiles, x, y);
        if dc.any_nonzero() {
            map.cover.insert((x, y), dc);
        }
    }

    map
}

// ── Cover computation ─────────────────────────────────────────────────────────

fn is_obstacle(tiles: &HashMap<(i32, i32), Tile>, x: i32, y: i32) -> bool {
    tiles.get(&(x, y)) == Some(&Tile::Obstacle)
}

/// Cover for a single cardinal direction `(dx, dy)`.
/// Direct obstacle neighbor → Full; diagonal obstacle neighbors → Partial; else None.
fn cover_in_direction(
    tiles: &HashMap<(i32, i32), Tile>,
    x: i32, y: i32,
    dx: i32, dy: i32,
) -> CoverLevel {
    if is_obstacle(tiles, x + dx, y + dy) {
        return CoverLevel::Full;
    }
    // Perpendicular diagonals for this direction:
    // N/S (dx==0): check (x±1, y+dy)   E/W (dy==0): check (x+dx, y±1)
    let (d1, d2) = if dy == 0 { ((dx, -1), (dx, 1)) } else { ((-1, dy), (1, dy)) };
    if is_obstacle(tiles, x + d1.0, y + d1.1) || is_obstacle(tiles, x + d2.0, y + d2.1) {
        CoverLevel::Partial
    } else {
        CoverLevel::None
    }
}

fn compute_directional_cover(tiles: &HashMap<(i32, i32), Tile>, x: i32, y: i32) -> DirectionalCover {
    DirectionalCover([
        cover_in_direction(tiles, x, y, 0, -1),  // North
        cover_in_direction(tiles, x, y, 0, 1),   // South
        cover_in_direction(tiles, x, y, 1, 0),   // East
        cover_in_direction(tiles, x, y, -1, 0),  // West
    ])
}
