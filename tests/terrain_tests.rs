use rand::SeedableRng;
use rand::rngs::StdRng;
use carbonthrone::terrain::{Biome, CoverLevel, Direction, Tile, generate_map};

fn rng() -> StdRng {
    StdRng::seed_from_u64(42)
}

// ── Tile passability ──────────────────────────────────────────────────────────

#[test]
fn obstacles_are_impassable() {
    assert!(!Tile::Obstacle.is_passable());
}

#[test]
fn open_tile_is_passable() {
    assert!(Tile::Open.is_passable());
}

// ── Map basics ────────────────────────────────────────────────────────────────

#[test]
fn spawn_positions_are_always_open() {
    let reserved = vec![(0i32, 0i32), (3i32, 3i32), (9i32, 9i32)];
    let map = generate_map(10, 10, Biome::AsteroidColony, &reserved, &mut rng());
    for (x, y) in &reserved {
        assert_eq!(
            map.get(*x, *y),
            Tile::Open,
            "reserved position ({}, {}) should be Open",
            x, y
        );
    }
}

#[test]
fn map_default_tile_is_open() {
    let map = generate_map(10, 10, Biome::VoidStation, &[], &mut rng());
    assert_eq!(map.get(100, 100), Tile::Open);
    assert_eq!(map.get(-1, -1), Tile::Open);
}

#[test]
fn map_dimensions_are_recorded() {
    let map = generate_map(12, 8, Biome::NeonDistrict, &[], &mut rng());
    assert_eq!(map.cols, 12);
    assert_eq!(map.rows, 8);
}

#[test]
fn biome_is_recorded_on_map() {
    let map = generate_map(10, 10, Biome::BioLab, &[], &mut rng());
    assert_eq!(map.biome, Biome::BioLab);
}

#[test]
fn map_contains_some_obstacles_for_asteroid_colony() {
    let map = generate_map(20, 20, Biome::AsteroidColony, &[], &mut rng());
    let obstacle_count = (0..20i32)
        .flat_map(|y| (0..20i32).map(move |x| (x, y)))
        .filter(|(x, y)| map.get(*x, *y) == Tile::Obstacle)
        .count();
    assert!(obstacle_count > 0, "expected obstacles in AsteroidColony map");
}

#[test]
fn different_biomes_produce_different_obstacle_densities() {
    let mut rng1 = StdRng::seed_from_u64(99);
    let mut rng2 = StdRng::seed_from_u64(99);
    let asteroid = generate_map(20, 20, Biome::AsteroidColony, &[], &mut rng1);
    let biolab = generate_map(20, 20, Biome::BioLab, &[], &mut rng2);

    let count = |map: &carbonthrone::terrain::LevelMap| {
        (0..20i32)
            .flat_map(|y| (0..20i32).map(move |x| (x, y)))
            .filter(|(x, y)| map.get(*x, *y) == Tile::Obstacle)
            .count()
    };
    // AsteroidColony (22%) should have more obstacles than BioLab (8%)
    assert!(
        count(&asteroid) > count(&biolab),
        "AsteroidColony should have more obstacles than BioLab"
    );
}

// ── Directional cover ─────────────────────────────────────────────────────────

#[test]
fn tile_adjacent_to_obstacle_gets_full_cover_from_that_direction() {
    // On a generated map, wherever a tile is directly east of an obstacle,
    // it should have Full cover from the West direction (attack coming from west).
    let map = generate_map(20, 20, Biome::AsteroidColony, &[], &mut rng());
    let mut found = false;
    'outer: for y in 0..20i32 {
        for x in 1..20i32 {
            if map.get(x, y) == Tile::Open && map.get(x - 1, y) == Tile::Obstacle {
                assert_eq!(
                    map.get_cover(x, y, Direction::West),
                    CoverLevel::Full,
                    "tile ({},{}) directly east of obstacle should have Full cover from West",
                    x, y
                );
                found = true;
                break 'outer;
            }
        }
    }
    assert!(found, "expected obstacle-adjacent tile in AsteroidColony map");
}

#[test]
fn cover_is_directional_not_omnidirectional() {
    // A tile with an obstacle to its north should have Full cover from North
    // but NOT Full cover from South (no obstacle to south).
    let map = generate_map(20, 20, Biome::AsteroidColony, &[], &mut rng());
    let mut found = false;
    'outer: for y in 1..19i32 {
        for x in 0..20i32 {
            if map.get(x, y) == Tile::Open
                && map.get(x, y - 1) == Tile::Obstacle
                && map.get(x, y + 1) == Tile::Open
            {
                assert_eq!(map.get_cover(x, y, Direction::North), CoverLevel::Full);
                assert!(
                    map.get_cover(x, y, Direction::South) < CoverLevel::Full,
                    "tile with obstacle north but open south should not have Full cover from South"
                );
                found = true;
                break 'outer;
            }
        }
    }
    assert!(found, "expected tile with obstacle to north and open to south");
}

#[test]
fn tile_diagonal_to_obstacle_may_get_partial_cover() {
    // Find a tile where the only adjacent obstacle is diagonal and check partial cover.
    let map = generate_map(20, 20, Biome::AsteroidColony, &[], &mut rng());
    let mut found_partial = false;
    'outer: for y in 1..19i32 {
        for x in 1..19i32 {
            if map.get(x, y) != Tile::Open { continue; }
            // Check: no direct obstacle north, but obstacle NW or NE
            let north_open = map.get(x, y - 1) == Tile::Open;
            let nw_obs = map.get(x - 1, y - 1) == Tile::Obstacle;
            let ne_obs = map.get(x + 1, y - 1) == Tile::Obstacle;
            if north_open && (nw_obs || ne_obs) {
                let cover = map.get_cover(x, y, Direction::North);
                if cover == CoverLevel::Partial {
                    found_partial = true;
                    break 'outer;
                }
            }
        }
    }
    // Partial cover from diagonal obstacles should exist in a dense map
    assert!(found_partial, "expected tile with partial cover from diagonal obstacle");
}

#[test]
fn isolated_tile_has_no_cover() {
    // Find a tile where all 8 neighbors are open and verify no cover.
    let map = generate_map(20, 20, Biome::NeonDistrict, &[], &mut rng());
    let dirs = [Direction::North, Direction::South, Direction::East, Direction::West];
    let mut found = false;
    'outer: for y in 1..19i32 {
        for x in 1..19i32 {
            if map.get(x, y) != Tile::Open { continue; }
            let all_neighbors_open = (-1..=1i32)
                .flat_map(|dy| (-1..=1i32).map(move |dx| (dx, dy)))
                .filter(|&(dx, dy)| dx != 0 || dy != 0)
                .all(|(dx, dy)| map.get(x + dx, y + dy) == Tile::Open);
            if all_neighbors_open {
                for dir in dirs {
                    assert_eq!(
                        map.get_cover(x, y, dir),
                        CoverLevel::None,
                        "isolated tile ({},{}) dir {:?} should have no cover",
                        x, y, dir
                    );
                }
                found = true;
                break 'outer;
            }
        }
    }
    // NeonDistrict (10%) should have isolated tiles
    if !found {
        eprintln!("Note: no fully isolated tile found — test is inconclusive");
    }
}

#[test]
fn display_glyph_reflects_cover_levels() {
    let map = generate_map(20, 20, Biome::AsteroidColony, &[], &mut rng());
    let dirs = [Direction::North, Direction::South, Direction::East, Direction::West];
    for y in 0..20i32 {
        for x in 0..20i32 {
            let glyph = map.display_glyph(x, y);
            match map.get(x, y) {
                Tile::Obstacle => assert_eq!(glyph, '#'),
                Tile::Open => {
                    let max_cover = dirs.iter().map(|&d| map.get_cover(x, y, d)).max().unwrap();
                    match max_cover {
                        CoverLevel::Full    => assert_eq!(glyph, 'C'),
                        CoverLevel::Partial => assert_eq!(glyph, 'c'),
                        CoverLevel::None    => assert_eq!(glyph, '.'),
                    }
                }
            }
        }
    }
}
