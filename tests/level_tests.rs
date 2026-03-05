use carbonthrone::level::{Level, SurpriseState};
use carbonthrone::terrain::Tile;
use rand::SeedableRng;
use rand::rngs::StdRng;

fn rng() -> StdRng {
    StdRng::seed_from_u64(42)
}

#[test]
fn level_has_at_least_one_enemy() {
    let level = Level::generate(1, &mut rng());
    assert!(!level.enemies.is_empty());
}

#[test]
fn level_has_at_most_four_enemies() {
    let level = Level::generate(1, &mut rng());
    assert!(level.enemies.len() <= 4);
}

#[test]
fn enemy_level_matches_depth() {
    let depth = 5;
    let level = Level::generate(depth, &mut rng());
    assert!(level.enemies.iter().all(|(e, _)| e.level == depth));
}

#[test]
fn depth_zero_clamps_enemy_level_to_one() {
    let level = Level::generate(0, &mut rng());
    assert!(level.enemies.iter().all(|(e, _)| e.level == 1));
}

#[test]
fn enemy_positions_are_within_grid() {
    let level = Level::generate(1, &mut rng());
    for (_, pos) in &level.enemies {
        assert!(pos.x >= 0 && pos.x < level.cols as i32);
        assert!(pos.y >= 0 && pos.y < level.rows as i32);
        assert_eq!(pos.z, 0);
    }
}

#[test]
fn enemy_positions_are_unique() {
    let level = Level::generate(1, &mut rng());
    let positions: Vec<_> = level.enemies.iter().map(|(_, p)| (p.x, p.y)).collect();
    let unique: std::collections::HashSet<_> = positions.iter().collect();
    assert_eq!(positions.len(), unique.len());
}

#[test]
fn level_grid_has_minimum_dimensions() {
    let level = Level::generate(1, &mut rng());
    assert!(level.cols >= 8);
    assert!(level.rows >= 8);
}

#[test]
fn surprise_states_all_occur_across_many_levels() {
    let mut rng = rng();
    let mut saw_normal = false;
    let mut saw_party_ambushed = false;
    let mut saw_enemy_ambushed = false;

    for _ in 0..200 {
        match Level::generate(1, &mut rng).surprise {
            SurpriseState::Normal => saw_normal = true,
            SurpriseState::PartyAmbushed => saw_party_ambushed = true,
            SurpriseState::EnemyAmbushed => saw_enemy_ambushed = true,
        }
    }

    assert!(saw_normal);
    assert!(saw_party_ambushed);
    assert!(saw_enemy_ambushed);
}

#[test]
fn different_seeds_produce_different_levels() {
    let a = Level::generate(3, &mut StdRng::seed_from_u64(1));
    let b = Level::generate(3, &mut StdRng::seed_from_u64(99));
    let same = a.enemies.len() == b.enemies.len()
        && a.enemies
            .iter()
            .zip(b.enemies.iter())
            .all(|((x, _), (y, _))| x.kind == y.kind);
    assert!(!same);
}

// ── Zone and terrain tests ────────────────────────────────────────────────

#[test]
fn level_has_a_zone_kind() {
    let level = Level::generate(1, &mut rng());
    // Just verify the zone_kind field exists and is valid (all variants are valid)
    let _ = level.zone_kind;
}

#[test]
fn level_map_dimensions_match_cols_rows() {
    let level = Level::generate(1, &mut rng());
    assert_eq!(level.map.cols, level.cols);
    assert_eq!(level.map.rows, level.rows);
}

#[test]
fn enemy_spawn_tiles_are_open() {
    let level = Level::generate(1, &mut rng());
    for (_, pos) in &level.enemies {
        assert_eq!(
            level.map.get(pos.x, pos.y),
            Tile::Open,
            "enemy spawn at ({}, {}) should be Open",
            pos.x,
            pos.y
        );
    }
}

#[test]
fn all_zone_kind_variants_can_be_generated() {
    use carbonthrone::zone::ZoneKind;
    let mut rng = rng();
    let mut saw = [false; 9];

    for _ in 0..500 {
        let idx = match Level::generate(1, &mut rng).zone_kind {
            ZoneKind::ResearchWing => 0,
            ZoneKind::CommandDeck => 1,
            ZoneKind::MilitaryAnnex => 2,
            ZoneKind::SystemsCore => 3,
            ZoneKind::MedicalBay => 4,
            ZoneKind::DockingBay => 5,
            ZoneKind::StationExterior => 6,
            ZoneKind::RelayArray => 7,
            ZoneKind::ExcavationSite => 8,
        };
        saw[idx] = true;
    }

    assert!(saw.iter().all(|&s| s), "all zone kinds should appear");
}
