use rand::SeedableRng;
use rand::rngs::StdRng;
use carbonthrone::level::{Level, SurpriseState};

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
            SurpriseState::Normal        => saw_normal = true,
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
        && a.enemies.iter().zip(b.enemies.iter()).all(|((x, _), (y, _))| x.kind == y.kind);
    assert!(!same);
}
