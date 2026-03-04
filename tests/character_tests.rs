use carbonthrone::experience::Experience;
use carbonthrone::health::Health;

// --- Health tests ---

#[test]
fn health_starts_at_full() {
    let h = Health::new(100);
    assert_eq!(h.current, h.max);
}

#[test]
fn take_damage_reduces_hp() {
    let mut h = Health::new(100);
    h.take_damage(10);
    assert_eq!(h.current, 90);
}

#[test]
fn hp_cannot_go_below_zero() {
    let mut h = Health::new(100);
    h.take_damage(9999);
    assert_eq!(h.current, 0);
    assert!(!h.is_alive());
}

#[test]
fn heal_does_not_exceed_max_hp() {
    let mut h = Health::new(100);
    h.take_damage(10);
    h.heal(9999);
    assert_eq!(h.current, h.max);
}

// --- Experience tests ---

#[test]
fn gaining_enough_xp_triggers_level_up() {
    let mut xp = Experience::new();
    assert_eq!(xp.level, 1);
    xp.add(100);
    assert_eq!(xp.level, 2);
}

#[test]
fn xp_carries_over_after_level_up() {
    let mut xp = Experience::new();
    xp.add(150); // 100 for lvl2, 50 carried
    assert_eq!(xp.level, 2);
    assert_eq!(xp.current, 50);
}

#[test]
fn level_advances_after_adding_xp() {
    let mut xp = Experience::new();
    xp.add(100);
    assert_eq!(xp.level, 2);
    assert_eq!(xp.current, 0);
}
