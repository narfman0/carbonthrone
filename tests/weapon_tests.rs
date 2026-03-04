use carbonthrone::character::{Character, CharacterClass};
use carbonthrone::weapon::{Weapon, WeaponKind};

#[test]
fn melee_weapon_has_correct_kind() {
    let blade = Weapon::melee("Combat Knife", 10, 1);
    assert_eq!(blade.kind, WeaponKind::Melee);
}

#[test]
fn ranged_weapon_has_correct_kind() {
    let rifle = Weapon::ranged("Plasma Rifle", 15, 2);
    assert_eq!(rifle.kind, WeaponKind::Ranged);
}

#[test]
fn spending_ap_reduces_ap() {
    let mut c = Character::new("Aldric", CharacterClass::Warrior);
    let before = c.ap;
    assert!(c.spend_ap(2));
    assert_eq!(c.ap, before - 2);
}

#[test]
fn cannot_spend_more_ap_than_available() {
    let mut c = Character::new("Aldric", CharacterClass::Warrior);
    c.ap = 1;
    assert!(!c.spend_ap(2));
    assert_eq!(c.ap, 1);
}

#[test]
fn refresh_ap_restores_to_max() {
    let mut c = Character::new("Aldric", CharacterClass::Warrior);
    c.ap = 0;
    c.refresh_ap();
    assert_eq!(c.ap, c.max_ap);
}

#[test]
fn switching_weapon_costs_ap() {
    let mut c = Character::new("Aldric", CharacterClass::Warrior);
    let rifle = Weapon::ranged("Plasma Rifle", 15, 2);
    let before = c.ap;
    assert!(c.spend_ap(rifle.switch_cost as i32));
    assert_eq!(c.ap, before - rifle.switch_cost as i32);
}

#[test]
fn cannot_switch_weapon_without_enough_ap() {
    let mut c = Character::new("Aldric", CharacterClass::Warrior);
    c.ap = 0;
    let rifle = Weapon::ranged("Plasma Rifle", 15, 2);
    assert!(!c.spend_ap(rifle.switch_cost as i32));
}
