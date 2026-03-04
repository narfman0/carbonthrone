use rand::SeedableRng;
use rand::rngs::StdRng;
use carbonthrone::terrain::{Biome, Tile, generate_map};

fn rng() -> StdRng {
    StdRng::seed_from_u64(42)
}

#[test]
fn obstacles_are_impassable() {
    assert!(!Tile::Obstacle.is_passable());
}

#[test]
fn open_tile_is_passable() {
    assert!(Tile::Open.is_passable());
}

#[test]
fn partial_cover_is_passable() {
    assert!(Tile::PartialCover.is_passable());
}

#[test]
fn full_cover_is_passable() {
    assert!(Tile::FullCover.is_passable());
}

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
    // Out-of-bounds coordinates default to Open
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
    // AsteroidColony has 18% obstacle density — with a 10×10 grid we expect several.
    let map = generate_map(20, 20, Biome::AsteroidColony, &[], &mut rng());
    let obstacle_count = (0..20i32)
        .flat_map(|y| (0..20i32).map(move |x| (x, y)))
        .filter(|(x, y)| map.get(*x, *y) == Tile::Obstacle)
        .count();
    assert!(obstacle_count > 0, "expected obstacles in AsteroidColony map");
}

#[test]
fn map_contains_some_full_cover_for_biolab() {
    // BioLab has 14% full cover density.
    let map = generate_map(20, 20, Biome::BioLab, &[], &mut rng());
    let full_cover_count = (0..20i32)
        .flat_map(|y| (0..20i32).map(move |x| (x, y)))
        .filter(|(x, y)| map.get(*x, *y) == Tile::FullCover)
        .count();
    assert!(full_cover_count > 0, "expected FullCover tiles in BioLab map");
}

#[test]
fn different_biomes_produce_different_tile_distributions() {
    let mut rng1 = StdRng::seed_from_u64(99);
    let mut rng2 = StdRng::seed_from_u64(99);
    let asteroid = generate_map(20, 20, Biome::AsteroidColony, &[], &mut rng1);
    let biolab = generate_map(20, 20, Biome::BioLab, &[], &mut rng2);

    let asteroid_obstacles = (0..20i32)
        .flat_map(|y| (0..20i32).map(move |x| (x, y)))
        .filter(|(x, y)| asteroid.get(*x, *y) == Tile::Obstacle)
        .count();
    let biolab_obstacles = (0..20i32)
        .flat_map(|y| (0..20i32).map(move |x| (x, y)))
        .filter(|(x, y)| biolab.get(*x, *y) == Tile::Obstacle)
        .count();

    // AsteroidColony has 3× the obstacle density of BioLab (18% vs 6%)
    assert!(
        asteroid_obstacles > biolab_obstacles,
        "AsteroidColony should have more obstacles than BioLab (got {} vs {})",
        asteroid_obstacles, biolab_obstacles
    );
}
