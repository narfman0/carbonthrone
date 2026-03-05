use carbonthrone::character::{Aggression, Character, CharacterKind};

#[test]
fn enemy_starts_at_full_hp() {
    let e = Character::new_npc(CharacterKind::Scavenger, 1);
    assert_eq!(e.current_hp, e.stats.max_hp);
}

#[test]
fn take_damage_reduces_hp() {
    let mut e = Character::new_npc(CharacterKind::DrifterBoss, 1);
    e.take_damage(10);
    assert_eq!(e.current_hp, e.stats.max_hp - 10);
}

#[test]
fn hp_cannot_go_below_zero() {
    let mut e = Character::new_npc(CharacterKind::Scavenger, 1);
    e.take_damage(9999);
    assert_eq!(e.current_hp, 0);
    assert!(!e.is_alive());
}

#[test]
fn higher_level_enemy_has_more_hp() {
    let low = Character::new_npc(CharacterKind::DrifterBoss, 1);
    let high = Character::new_npc(CharacterKind::DrifterBoss, 5);
    assert!(high.stats.max_hp > low.stats.max_hp);
}

#[test]
fn higher_level_enemy_gives_more_xp() {
    let low = Character::new_npc(CharacterKind::VoidRaider, 1);
    let high = Character::new_npc(CharacterKind::VoidRaider, 3);
    assert!(high.xp_reward > low.xp_reward);
}

#[test]
fn combat_frame_has_more_hp_than_scavenger_at_same_level() {
    let boss = Character::new_npc(CharacterKind::CombatFrame, 1);
    let light = Character::new_npc(CharacterKind::Scavenger, 1);
    assert!(boss.stats.max_hp > light.stats.max_hp);
}

#[test]
fn take_damage_and_check_name() {
    let mut e = Character::new_npc(CharacterKind::DrifterBoss, 1);
    let hp_before = e.current_hp;
    e.take_damage(5);
    assert_eq!(e.current_hp, hp_before - 5);
    assert_eq!(e.name, "Drifter Boss");
}

#[test]
fn default_aggression_is_correct() {
    assert_eq!(
        Character::new_npc(CharacterKind::Scavenger, 1).aggression,
        Aggression::Aggressive
    );
    assert_eq!(
        Character::new_npc(CharacterKind::MaintenanceDrone, 1).aggression,
        Aggression::Neutral
    );
    assert_eq!(
        Character::new_npc(CharacterKind::SalvageOperative, 1).aggression,
        Aggression::Friendly
    );
    assert_eq!(
        Character::new_npc(CharacterKind::StationGuard, 1).aggression,
        Aggression::Friendly
    );
    assert_eq!(
        Character::new_npc(CharacterKind::ShockTrooper, 1).aggression,
        Aggression::Aggressive
    );
}
