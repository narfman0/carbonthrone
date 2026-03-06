use carbonthrone::terrain::Tile;
use carbonthrone::zone::{SurpriseState, Zone, ZoneKind};
use rand::SeedableRng;
use rand::rngs::StdRng;

fn rng() -> StdRng {
    StdRng::seed_from_u64(42)
}

#[test]
fn level_has_at_least_one_enemy() {
    let zone = Zone::generate(1, &mut rng());
    assert!(!zone.enemies.is_empty());
}

#[test]
fn level_has_at_most_four_enemies() {
    let zone = Zone::generate(1, &mut rng());
    assert!(zone.enemies.len() <= 4);
}

#[test]
fn enemy_level_matches_depth() {
    let depth = 5;
    let zone = Zone::generate(depth, &mut rng());
    assert!(zone.enemies.iter().all(|(e, _)| e.level == depth));
}

#[test]
fn depth_zero_clamps_enemy_level_to_one() {
    let zone = Zone::generate(0, &mut rng());
    assert!(zone.enemies.iter().all(|(e, _)| e.level == 1));
}

#[test]
fn enemy_positions_are_within_grid() {
    let zone = Zone::generate(1, &mut rng());
    for (_, pos) in &zone.enemies {
        assert!(pos.x >= 0 && pos.x < zone.cols as i32);
        assert!(pos.y >= 0 && pos.y < zone.rows as i32);
    }
}

#[test]
fn enemy_positions_are_unique() {
    let zone = Zone::generate(1, &mut rng());
    let positions: Vec<_> = zone.enemies.iter().map(|(_, p)| (p.x, p.y)).collect();
    let unique: std::collections::HashSet<_> = positions.iter().collect();
    assert_eq!(positions.len(), unique.len());
}

#[test]
fn level_grid_has_minimum_dimensions() {
    let zone = Zone::generate(1, &mut rng());
    assert!(zone.cols >= 8);
    assert!(zone.rows >= 8);
}

#[test]
fn surprise_states_all_occur_across_many_levels() {
    let mut rng = rng();
    let mut saw_normal = false;
    let mut saw_party_ambushed = false;
    let mut saw_enemy_ambushed = false;

    for _ in 0..200 {
        match Zone::generate(1, &mut rng).surprise {
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
    let a = Zone::generate(3, &mut StdRng::seed_from_u64(1));
    let b = Zone::generate(3, &mut StdRng::seed_from_u64(99));
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
    let zone = Zone::generate(1, &mut rng());
    let _ = zone.kind;
}

#[test]
fn level_map_dimensions_match_cols_rows() {
    let zone = Zone::generate(1, &mut rng());
    assert_eq!(zone.map.cols, zone.cols);
    assert_eq!(zone.map.rows, zone.rows);
}

#[test]
fn enemy_spawn_tiles_are_open() {
    let zone = Zone::generate(1, &mut rng());
    for (_, pos) in &zone.enemies {
        assert_eq!(
            zone.map.get(pos.x, pos.y),
            Tile::Open,
            "enemy spawn at ({}, {}) should be Open",
            pos.x,
            pos.y
        );
    }
}

#[test]
fn all_zone_kind_variants_can_be_generated() {
    let mut rng = rng();
    let mut saw = [false; 9];

    for _ in 0..500 {
        let idx = match Zone::generate(1, &mut rng).kind {
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
