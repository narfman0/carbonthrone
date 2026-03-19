use std::collections::{HashMap, HashSet, VecDeque};

use bevy::prelude::Resource;
use rand::Rng;
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};

use crate::zone::ZoneKind;

/// The terrain type of a single map cell. Tiles are either passable or not;
/// cover is derived from adjacency to obstacles, not stored on the tile itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tile {
    /// Open ground — passable, no inherent cover.
    Open,
    /// Impassable solid structure — blocks movement and provides adjacent cover.
    Obstacle,
    /// Door — passable threshold leading to the next zone.
    Door,
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
            if dx >= 0 {
                Direction::East
            } else {
                Direction::West
            }
        } else if dy >= 0 {
            Direction::South
        } else {
            Direction::North
        }
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
#[derive(Debug, Clone, Resource)]
pub struct LevelMap {
    pub cols: u32,
    pub rows: u32,
    pub zone_kind: ZoneKind,
    tiles: HashMap<(i32, i32), Tile>,
    cover: HashMap<(i32, i32), DirectionalCover>,
}

impl LevelMap {
    pub fn new(cols: u32, rows: u32, zone_kind: ZoneKind) -> Self {
        Self {
            cols,
            rows,
            zone_kind,
            tiles: HashMap::new(),
            cover: HashMap::new(),
        }
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
        self.cover
            .get(&(x, y))
            .map(|dc| dc.get(from))
            .unwrap_or(CoverLevel::None)
    }

    /// Recomputes directional cover for all passable tiles from the current obstacle layout.
    /// Call this after manually setting tiles (e.g. in tests) to keep cover data consistent.
    pub fn recompute_cover(&mut self) {
        self.cover.clear();
        let passable: Vec<(i32, i32)> = (0..self.rows as i32)
            .flat_map(|y| (0..self.cols as i32).map(move |x| (x, y)))
            .filter(|&(x, y)| self.get(x, y).is_passable())
            .collect();
        for (x, y) in passable {
            let dc = compute_directional_cover(&self.tiles, x, y);
            if dc.any_nonzero() {
                self.cover.insert((x, y), dc);
            }
        }
    }

    /// BFS pathfinding from `from` to `to`, avoiding non-passable tiles and `occupied` positions.
    /// Returns the path (excluding `from`, including `to`) or an empty vec if unreachable.
    pub fn bfs_path(
        &self,
        from: (i32, i32),
        to: (i32, i32),
        occupied: &HashSet<(i32, i32)>,
    ) -> Vec<(i32, i32)> {
        if from == to {
            return vec![];
        }
        let mut visited: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
        let mut queue: VecDeque<(i32, i32)> = VecDeque::new();
        visited.insert(from, from);
        queue.push_back(from);
        let cols = self.cols as i32;
        let rows = self.rows as i32;
        while let Some(cur) = queue.pop_front() {
            if cur == to {
                // Reconstruct path.
                let mut path = vec![cur];
                let mut prev = cur;
                loop {
                    let p = visited[&prev];
                    if p == from {
                        break;
                    }
                    path.push(p);
                    prev = p;
                }
                path.reverse();
                return path;
            }
            for (dx, dy) in [(0, 1), (0, -1), (1, 0), (-1, 0)] {
                let nx = cur.0 + dx;
                let ny = cur.1 + dy;
                if nx < 0 || ny < 0 || nx >= cols || ny >= rows {
                    continue;
                }
                let next = (nx, ny);
                if visited.contains_key(&next) {
                    continue;
                }
                if !self.is_passable(nx, ny) {
                    continue;
                }
                // Allow the destination even if occupied (player moves there).
                if next != to && occupied.contains(&next) {
                    continue;
                }
                visited.insert(next, cur);
                queue.push_back(next);
            }
        }
        vec![]
    }

    /// Returns the nearest passable tile to `(x, y)` using BFS.
    ///
    /// If `(x, y)` is already passable it is returned as-is.  The search
    /// expands outward until an open or door tile is found.  Falls back to
    /// `(0, 0)` only if the entire map contains no passable tile.
    pub fn nearest_open_tile(&self, x: i32, y: i32) -> (i32, i32) {
        let cols = self.cols as i32;
        let rows = self.rows as i32;
        let start = (x.clamp(0, cols - 1), y.clamp(0, rows - 1));
        if self.is_passable(start.0, start.1) {
            return start;
        }
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(start);
        visited.insert(start);
        while let Some((cx, cy)) = queue.pop_front() {
            if self.is_passable(cx, cy) {
                return (cx, cy);
            }
            for (dx, dy) in [(0, 1), (0, -1), (1, 0), (-1, 0)] {
                let nx = cx + dx;
                let ny = cy + dy;
                if nx < 0 || ny < 0 || nx >= cols || ny >= rows {
                    continue;
                }
                if visited.insert((nx, ny)) {
                    queue.push_back((nx, ny));
                }
            }
        }
        (0, 0)
    }

    /// Returns all non-Open tiles as a vec of `(position, tile)` pairs.
    /// Used for serializing the map; only sparse (obstacle/door) tiles are stored.
    pub fn non_default_tiles(&self) -> Vec<((i32, i32), Tile)> {
        self.tiles.iter().map(|(&pos, &tile)| (pos, tile)).collect()
    }

    /// Construct a `LevelMap` from an explicit tile map, recomputing cover.
    /// `tiles` should contain only non-Open entries (Open is the default).
    pub fn from_tile_map(
        cols: u32,
        rows: u32,
        zone_kind: ZoneKind,
        tiles: HashMap<(i32, i32), Tile>,
    ) -> Self {
        let mut map = Self {
            cols,
            rows,
            zone_kind,
            tiles,
            cover: HashMap::new(),
        };
        map.recompute_cover();
        map
    }

    /// Terminal character for rendering, including cover hints:
    /// `'#'` obstacle, `'+'` door, `'C'` full cover any direction, `'c'` partial, `'.'` open.
    pub fn display_glyph(&self, x: i32, y: i32) -> char {
        match self.get(x, y) {
            Tile::Obstacle => return '#',
            Tile::Door => return '+',
            Tile::Open => {}
        }
        let dirs = [
            Direction::North,
            Direction::South,
            Direction::East,
            Direction::West,
        ];
        if dirs
            .iter()
            .any(|&d| self.get_cover(x, y, d) == CoverLevel::Full)
        {
            'C'
        } else if dirs
            .iter()
            .any(|&d| self.get_cover(x, y, d) == CoverLevel::Partial)
        {
            'c'
        } else {
            '.'
        }
    }
}

/// Battle RNG resource — wraps a seeded RNG so it can be stored on the `World`.
#[derive(Resource)]
pub struct BattleRng(pub StdRng);

// ── Zone state modifiers ──────────────────────────────────────────────────────

enum ZoneStateModifier {
    ClearRegion {
        x_start: f32,
        y_start: f32,
        x_end: f32,
        y_end: f32,
    },
    BlockRegion {
        x_start: f32,
        y_start: f32,
        x_end: f32,
        y_end: f32,
    },
}

fn zone_state_modifiers(zone_kind: ZoneKind, loop_number: u32) -> Vec<ZoneStateModifier> {
    match zone_kind {
        ZoneKind::ExcavationSite => {
            if loop_number >= 3 {
                vec![ZoneStateModifier::ClearRegion {
                    x_start: 0.0,
                    y_start: 0.6,
                    x_end: 0.35,
                    y_end: 1.0,
                }]
            } else {
                vec![ZoneStateModifier::BlockRegion {
                    x_start: 0.0,
                    y_start: 0.6,
                    x_end: 0.35,
                    y_end: 1.0,
                }]
            }
        }
        _ => vec![],
    }
}

// ── Zone densities ────────────────────────────────────────────────────────────

fn zone_density(zone_kind: ZoneKind) -> f32 {
    match zone_kind {
        ZoneKind::ResearchWing => 0.08,
        ZoneKind::CommandDeck => 0.15,
        ZoneKind::MilitaryAnnex => 0.15,
        ZoneKind::SystemsCore => 0.15,
        ZoneKind::MedicalBay => 0.08,
        ZoneKind::DockingBay => 0.10,
        ZoneKind::StationExterior => 0.22,
        ZoneKind::RelayArray => 0.22,
        ZoneKind::ExcavationSite => 0.22,
        ZoneKind::Hallway => 0.12,
    }
}

// ── Map generation ───────────────────────────────────────────────────────────

/// Procedurally generate a terrain map for the given zone.
///
/// Obstacles are placed randomly; directional cover is then computed for every
/// passable tile based on its obstacle neighbors.
/// `reserved_open` lists positions that must remain passable (e.g. spawn tiles).
pub fn generate_map(
    cols: u32,
    rows: u32,
    zone_kind: ZoneKind,
    loop_number: u32,
    reserved_open: &[(i32, i32)],
    door_tiles: &[(i32, i32)],
    rng: &mut impl Rng,
) -> LevelMap {
    let mut map = LevelMap::new(cols, rows, zone_kind);
    let density = zone_density(zone_kind);
    let reserved: HashSet<(i32, i32)> = reserved_open
        .iter()
        .chain(door_tiles.iter())
        .copied()
        .collect();

    // Place obstacles.
    for y in 0..rows as i32 {
        for x in 0..cols as i32 {
            if reserved.contains(&(x, y)) {
                continue;
            }
            if rng.r#gen::<f32>() < density {
                map.tiles.insert((x, y), Tile::Obstacle);
            }
        }
    }

    // Mark door tiles.
    for &(x, y) in door_tiles {
        map.tiles.insert((x, y), Tile::Door);
    }

    // Apply loop-based zone state modifiers (e.g. collapsed sections).
    for modifier in zone_state_modifiers(zone_kind, loop_number) {
        match modifier {
            ZoneStateModifier::ClearRegion {
                x_start,
                y_start,
                x_end,
                y_end,
            } => {
                let (x0, y0) = (
                    (x_start * cols as f32) as i32,
                    (y_start * rows as f32) as i32,
                );
                let (x1, y1) = ((x_end * cols as f32) as i32, (y_end * rows as f32) as i32);
                for y in y0..y1 {
                    for x in x0..x1 {
                        if map.get(x, y) == Tile::Obstacle {
                            map.tiles.remove(&(x, y));
                        }
                    }
                }
            }
            ZoneStateModifier::BlockRegion {
                x_start,
                y_start,
                x_end,
                y_end,
            } => {
                let (x0, y0) = (
                    (x_start * cols as f32) as i32,
                    (y_start * rows as f32) as i32,
                );
                let (x1, y1) = ((x_end * cols as f32) as i32, (y_end * rows as f32) as i32);
                for y in y0..y1 {
                    for x in x0..x1 {
                        if !reserved.contains(&(x, y)) {
                            map.tiles.insert((x, y), Tile::Obstacle);
                        }
                    }
                }
            }
        }
    }

    // Compute directional cover for all passable tiles from obstacle adjacency.
    let passable: Vec<(i32, i32)> = (0..rows as i32)
        .flat_map(|y| (0..cols as i32).map(move |x| (x, y)))
        .filter(|&(x, y)| map.get(x, y).is_passable())
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
    x: i32,
    y: i32,
    dx: i32,
    dy: i32,
) -> CoverLevel {
    if is_obstacle(tiles, x + dx, y + dy) {
        return CoverLevel::Full;
    }
    // Perpendicular diagonals for this direction:
    // N/S (dx==0): check (x±1, y+dy)   E/W (dy==0): check (x+dx, y±1)
    let (d1, d2) = if dy == 0 {
        ((dx, -1), (dx, 1))
    } else {
        ((-1, dy), (1, dy))
    };
    if is_obstacle(tiles, x + d1.0, y + d1.1) || is_obstacle(tiles, x + d2.0, y + d2.1) {
        CoverLevel::Partial
    } else {
        CoverLevel::None
    }
}

fn compute_directional_cover(
    tiles: &HashMap<(i32, i32), Tile>,
    x: i32,
    y: i32,
) -> DirectionalCover {
    DirectionalCover([
        cover_in_direction(tiles, x, y, 0, -1), // North
        cover_in_direction(tiles, x, y, 0, 1),  // South
        cover_in_direction(tiles, x, y, 1, 0),  // East
        cover_in_direction(tiles, x, y, -1, 0), // West
    ])
}
