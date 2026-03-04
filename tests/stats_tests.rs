use carbonthrone::character::CharacterClass;
use carbonthrone::stats::Stats;

#[test]
fn warrior_has_highest_hp() {
    let warrior = Stats::for_class(&CharacterClass::Warrior);
    let mage = Stats::for_class(&CharacterClass::Mage);
    assert!(warrior.max_hp > mage.max_hp);
}

#[test]
fn mage_has_highest_magic() {
    let mage = Stats::for_class(&CharacterClass::Mage);
    let warrior = Stats::for_class(&CharacterClass::Warrior);
    assert!(mage.magic > warrior.magic);
}

#[test]
fn level_up_increases_max_hp() {
    let mut stats = Stats::for_class(&CharacterClass::Warrior);
    let hp_before = stats.max_hp;
    stats.level_up(&CharacterClass::Warrior);
    assert!(stats.max_hp > hp_before);
}
