use carbonthrone::character::CharacterClass;
use carbonthrone::stats::Stats;

#[test]
fn warrior_has_highest_hp() {
    let warrior = Stats::for_class(&CharacterClass::Warrior);
    let rogue = Stats::for_class(&CharacterClass::Rogue);
    assert!(warrior.max_hp > rogue.max_hp);
}

#[test]
fn level_up_increases_max_hp() {
    let mut stats = Stats::for_class(&CharacterClass::Warrior);
    let hp_before = stats.max_hp;
    stats.level_up(&CharacterClass::Warrior);
    assert!(stats.max_hp > hp_before);
}
