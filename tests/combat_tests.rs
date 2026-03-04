use rand::SeedableRng;
use rand::rngs::StdRng;
use carbonthrone::combat::{calc_damage, calc_hit_chance, roll_hit, turn_order, BASE_HIT_CHANCE};
use carbonthrone::terrain::Tile;

#[test]
fn damage_reduced_by_defense() {
    assert_eq!(calc_damage(20, 10), 15); // 20 - 10/2 = 15
}

#[test]
fn damage_minimum_is_one() {
    assert_eq!(calc_damage(1, 100), 1);
}

#[test]
fn turn_order_highest_speed_first() {
    let speeds = vec![5, 18, 10];
    let order = turn_order(&speeds);
    assert_eq!(order, vec![1, 2, 0]); // 18, 10, 5
}

#[test]
fn turn_order_single_combatant() {
    assert_eq!(turn_order(&[7]), vec![0]);
}

// ── Hit chance ────────────────────────────────────────────────────────────

#[test]
fn open_tile_has_base_hit_chance() {
    assert!((calc_hit_chance(Tile::Open) - BASE_HIT_CHANCE).abs() < f32::EPSILON);
}

#[test]
fn partial_cover_reduces_hit_chance() {
    let chance = calc_hit_chance(Tile::PartialCover);
    assert!((chance - 0.65).abs() < f32::EPSILON);
    assert!(chance < BASE_HIT_CHANCE);
}

#[test]
fn full_cover_reduces_hit_chance_significantly() {
    let chance = calc_hit_chance(Tile::FullCover);
    assert!((chance - 0.35).abs() < f32::EPSILON);
    assert!(chance < calc_hit_chance(Tile::PartialCover));
}

#[test]
fn always_miss_at_zero_chance() {
    let mut rng = StdRng::seed_from_u64(0);
    for _ in 0..100 {
        assert!(!roll_hit(0.0, &mut rng));
    }
}

#[test]
fn always_hit_at_full_chance() {
    let mut rng = StdRng::seed_from_u64(0);
    for _ in 0..100 {
        assert!(roll_hit(1.0, &mut rng));
    }
}
