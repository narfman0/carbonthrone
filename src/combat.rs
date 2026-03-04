use rand::Rng;

use crate::terrain::Tile;

/// Calculates physical damage dealt, factoring in attacker's attack and defender's defense.
/// Minimum of 1 damage always applies.
pub fn calc_damage(attack: i32, defense: i32) -> i32 {
    (attack - defense / 2).max(1)
}

/// Base probability that an attack connects (no cover).
pub const BASE_HIT_CHANCE: f32 = 0.90;

/// Returns the hit probability for an attack against a defender on `tile`.
/// Partial cover reduces hit chance to 65%; full cover to 35%.
pub fn calc_hit_chance(tile: Tile) -> f32 {
    match tile {
        Tile::Open | Tile::Obstacle => BASE_HIT_CHANCE,
        Tile::PartialCover => 0.65,
        Tile::FullCover => 0.35,
    }
}

/// Rolls to determine whether an attack hits. Returns `true` on a hit.
pub fn roll_hit(hit_chance: f32, rng: &mut impl Rng) -> bool {
    rng.r#gen::<f32>() < hit_chance
}

/// Determines turn order by speed (highest speed acts first).
/// Returns indices into the provided speed slice, sorted descending.
pub fn turn_order(speeds: &[i32]) -> Vec<usize> {
    let mut indexed: Vec<(usize, i32)> = speeds.iter().copied().enumerate().collect();
    indexed.sort_by(|a, b| b.1.cmp(&a.1));
    indexed.into_iter().map(|(i, _)| i).collect()
}
