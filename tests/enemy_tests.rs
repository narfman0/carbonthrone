use carbonthrone::combatant::Combatant;
use carbonthrone::enemy::{Enemy, EnemyKind};

#[test]
fn enemy_starts_at_full_hp() {
    let e = Enemy::new(EnemyKind::Goblin, 1);
    assert_eq!(e.current_hp, e.stats.max_hp);
}

#[test]
fn take_damage_reduces_hp() {
    let mut e = Enemy::new(EnemyKind::Orc, 1);
    e.take_damage(10);
    assert_eq!(e.current_hp, e.stats.max_hp - 10);
}

#[test]
fn hp_cannot_go_below_zero() {
    let mut e = Enemy::new(EnemyKind::Goblin, 1);
    e.take_damage(9999);
    assert_eq!(e.current_hp, 0);
    assert!(!e.is_alive());
}

#[test]
fn higher_level_enemy_has_more_hp() {
    let low  = Enemy::new(EnemyKind::Orc, 1);
    let high = Enemy::new(EnemyKind::Orc, 5);
    assert!(high.stats.max_hp > low.stats.max_hp);
}

#[test]
fn higher_level_enemy_gives_more_xp() {
    let low  = Enemy::new(EnemyKind::Skeleton, 1);
    let high = Enemy::new(EnemyKind::Skeleton, 3);
    assert!(high.xp_reward > low.xp_reward);
}

#[test]
fn dragon_has_more_hp_than_goblin_at_same_level() {
    let dragon = Enemy::new(EnemyKind::Dragon, 1);
    let goblin = Enemy::new(EnemyKind::Goblin, 1);
    assert!(dragon.stats.max_hp > goblin.stats.max_hp);
}

#[test]
fn dark_mage_has_high_magic() {
    let mage   = Enemy::new(EnemyKind::DarkMage, 1);
    let goblin = Enemy::new(EnemyKind::Goblin, 1);
    assert!(mage.stats.magic > goblin.stats.magic);
}

#[test]
fn combatant_trait_delegates_correctly() {
    let mut e = Enemy::new(EnemyKind::Orc, 1);
    let hp_before = e.current_hp();
    e.take_damage(5);
    assert_eq!(e.current_hp(), hp_before - 5);
    assert_eq!(e.name(), "Orc");
}
