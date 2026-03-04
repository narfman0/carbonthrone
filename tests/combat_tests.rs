use carbonthrone::combat::{calc_damage, turn_order};

#[test]
fn damage_reduced_by_defense() {
    assert_eq!(calc_damage(20, 10), 15); // 20 - 10/2 = 15
}

#[test]
fn damage_minimum_is_one() {
    assert_eq!(calc_damage(1, 100), 1);
}

#[test]
fn turn_order_highest_speed_first() {
    let speeds = vec![5, 18, 10];
    let order = turn_order(&speeds);
    assert_eq!(order, vec![1, 2, 0]); // 18, 10, 5
}

#[test]
fn turn_order_single_combatant() {
    assert_eq!(turn_order(&[7]), vec![0]);
}
