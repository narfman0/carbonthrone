use carbonthrone::character::CharacterKind;
use carbonthrone::stats::Stats;

#[test]
fn doss_has_highest_hp() {
    let doss = Stats::for_character(&CharacterKind::Doss);
    let researcher = Stats::for_character(&CharacterKind::Researcher);
    assert!(doss.max_hp > researcher.max_hp);
}

#[test]
fn level_up_increases_max_hp() {
    let mut stats = Stats::for_character(&CharacterKind::Doss);
    let hp_before = stats.max_hp;
    stats.level_up(&CharacterKind::Doss);
    assert!(stats.max_hp > hp_before);
}
