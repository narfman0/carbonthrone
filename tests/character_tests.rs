use carbonthrone::character::{Character, CharacterClass};

#[test]
fn new_character_starts_at_full_hp() {
    let c = Character::new("Aldric", CharacterClass::Warrior);
    assert_eq!(c.current_hp, c.stats.max_hp);
}

#[test]
fn take_damage_reduces_hp() {
    let mut c = Character::new("Aldric", CharacterClass::Warrior);
    c.take_damage(10);
    assert_eq!(c.current_hp, c.stats.max_hp - 10);
}

#[test]
fn hp_cannot_go_below_zero() {
    let mut c = Character::new("Aldric", CharacterClass::Warrior);
    c.take_damage(9999);
    assert_eq!(c.current_hp, 0);
    assert!(!c.is_alive());
}

#[test]
fn heal_does_not_exceed_max_hp() {
    let mut c = Character::new("Aldric", CharacterClass::Warrior);
    c.take_damage(10);
    c.heal(9999);
    assert_eq!(c.current_hp, c.stats.max_hp);
}

#[test]
fn gaining_enough_xp_triggers_level_up() {
    let mut c = Character::new("Aldric", CharacterClass::Warrior);
    assert_eq!(c.level, 1);
    c.gain_experience(100);
    assert_eq!(c.level, 2);
}

#[test]
fn xp_carries_over_after_level_up() {
    let mut c = Character::new("Aldric", CharacterClass::Warrior);
    c.gain_experience(150); // 100 for lvl2, 50 carried
    assert_eq!(c.level, 2);
    assert_eq!(c.experience, 50);
}
