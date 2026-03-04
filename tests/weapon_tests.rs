use carbonthrone::action_points::ActionPoints;
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
    let mut ap = ActionPoints::new(4);
    let before = ap.current;
    assert!(ap.spend(2));
    assert_eq!(ap.current, before - 2);
}

#[test]
fn cannot_spend_more_ap_than_available() {
    let mut ap = ActionPoints::new(4);
    ap.current = 1;
    assert!(!ap.spend(2));
    assert_eq!(ap.current, 1);
}

#[test]
fn refresh_ap_restores_to_max() {
    let mut ap = ActionPoints::new(4);
    ap.current = 0;
    ap.refresh();
    assert_eq!(ap.current, ap.max);
}

#[test]
fn switching_weapon_costs_ap() {
    let mut ap = ActionPoints::new(4);
    let rifle = Weapon::ranged("Plasma Rifle", 15, 2);
    let before = ap.current;
    assert!(ap.spend(rifle.switch_cost as i32));
    assert_eq!(ap.current, before - rifle.switch_cost as i32);
}

#[test]
fn cannot_switch_weapon_without_enough_ap() {
    let mut ap = ActionPoints::new(4);
    ap.current = 0;
    let rifle = Weapon::ranged("Plasma Rifle", 15, 2);
    assert!(!ap.spend(rifle.switch_cost as i32));
}
