/// Calculates physical damage dealt, factoring in attacker's attack and defender's defense.
/// Minimum of 1 damage always applies.
pub fn calc_damage(attack: i32, defense: i32) -> i32 {
    (attack - defense / 2).max(1)
}

/// Determines turn order by speed (highest speed acts first).
/// Returns indices into the provided speed slice, sorted descending.
pub fn turn_order(speeds: &[i32]) -> Vec<usize> {
    let mut indexed: Vec<(usize, i32)> = speeds.iter().copied().enumerate().collect();
    indexed.sort_by(|a, b| b.1.cmp(&a.1));
    indexed.into_iter().map(|(i, _)| i).collect()
}
