use bevy::prelude::*;
use carbonthrone::{
    ability::{
        AbilityEffect, ClassAbilities, available_abilities, class_abilities,
    },
    action_points::ActionPoints,
    character::CharacterClass,
    health::Health,
    stats::Stats,
    turn::{Action, TurnAction, apply_action},
};

// ── Ability table tests ───────────────────────────────────────────────────────

#[test]
fn each_class_has_three_abilities() {
    for class in [
        CharacterClass::Warrior,
        CharacterClass::Rogue,
        CharacterClass::Cleric,
        CharacterClass::Ranger,
    ] {
        assert_eq!(class_abilities(&class).len(), 3, "{class:?} should have 3 abilities");
    }
}

#[test]
fn each_class_has_level_1_ability() {
    for class in [
        CharacterClass::Warrior,
        CharacterClass::Rogue,
        CharacterClass::Cleric,
        CharacterClass::Ranger,
    ] {
        let lvl1: Vec<_> = class_abilities(&class)
            .into_iter()
            .filter(|a| a.level_required == 1)
            .collect();
        assert_eq!(lvl1.len(), 1, "{class:?} should have exactly one level-1 ability");
    }
}

#[test]
fn available_abilities_filters_by_level() {
    // At level 1 warrior only has Power Strike
    let lvl1 = available_abilities(&CharacterClass::Warrior, 1);
    assert_eq!(lvl1.len(), 1);
    assert_eq!(lvl1[0].name, "Power Strike");

    // At level 5 warrior gains Shield Bash
    let lvl5 = available_abilities(&CharacterClass::Warrior, 5);
    assert_eq!(lvl5.len(), 2);

    // At level 12 warrior has all three
    let lvl12 = available_abilities(&CharacterClass::Warrior, 12);
    assert_eq!(lvl12.len(), 3);
}

#[test]
fn class_abilities_component_available_matches_free_fn() {
    let comp = ClassAbilities::new(CharacterClass::Rogue);
    assert_eq!(comp.available(6).len(), available_abilities(&CharacterClass::Rogue, 6).len());
}

#[test]
fn all_abilities_have_positive_or_zero_ap_cost() {
    for class in [
        CharacterClass::Warrior,
        CharacterClass::Rogue,
        CharacterClass::Cleric,
        CharacterClass::Ranger,
    ] {
        for ability in class_abilities(&class) {
            assert!(ability.ap_cost >= 0, "{} has negative AP cost", ability.name);
        }
    }
}

#[test]
fn all_abilities_have_non_empty_names_and_descriptions() {
    for class in [
        CharacterClass::Warrior,
        CharacterClass::Rogue,
        CharacterClass::Cleric,
        CharacterClass::Ranger,
    ] {
        for ability in class_abilities(&class) {
            assert!(!ability.name.is_empty(), "ability has empty name");
            assert!(!ability.description.is_empty(), "ability has empty description");
        }
    }
}

// ── UseAbility action tests ───────────────────────────────────────────────────

fn stats(attack: i32, defense: i32) -> Stats {
    Stats { max_hp: 100, attack, defense, speed: 10 }
}

#[test]
fn power_strike_deals_bonus_damage() {
    let mut world = World::new();
    let ability = available_abilities(&CharacterClass::Warrior, 1).remove(0);
    assert_eq!(ability.name, "Power Strike");

    let attacker = world.spawn((stats(10, 5), ActionPoints::new(4))).id();
    let target = world.spawn((stats(5, 0), Health::new(100))).id();

    // No BattleRng → always hits. Normal calc_damage(10, 0) = 10, + bonus 8 = 18.
    let result = apply_action(&mut world, attacker, &Action::UseAbility { ability, target: Some(target) });

    assert!(result.is_some());
    match result.unwrap() {
        TurnAction::UseAbility { hit, value, .. } => {
            assert!(hit);
            assert!(value > 0);
        }
        _ => panic!("expected UseAbility result"),
    }
    assert!(world.get::<Health>(target).unwrap().current < 100);
}

#[test]
fn power_strike_fails_without_enough_ap() {
    let mut world = World::new();
    let ability = available_abilities(&CharacterClass::Warrior, 1).remove(0);
    assert_eq!(ability.ap_cost, 3);

    let attacker = world.spawn((stats(10, 5), ActionPoints::new(2))).id(); // only 2 AP
    let target = world.spawn((stats(5, 0), Health::new(100))).id();

    let result = apply_action(&mut world, attacker, &Action::UseAbility { ability, target: Some(target) });

    assert!(result.is_none());
    assert_eq!(world.get::<Health>(target).unwrap().current, 100); // undamaged
    assert_eq!(world.get::<ActionPoints>(attacker).unwrap().current, 2); // unchanged
}

#[test]
fn sneak_attack_pierces_armor() {
    let mut world = World::new();
    let sneak_attack = available_abilities(&CharacterClass::Rogue, 1).remove(0);
    assert_eq!(sneak_attack.name, "Sneak Attack");
    assert!(matches!(sneak_attack.effect, AbilityEffect::ArmorPiercing { .. }));

    let attacker = world.spawn((stats(15, 5), ActionPoints::new(4))).id();
    // High defense target
    let target = world.spawn((stats(5, 20), Health::new(100))).id();

    // Normal attack: calc_damage(15, 20) = (15 - 10).max(1) = 5
    // Sneak attack with 75% pierce: effective_defense = 20 * 0.25 = 5 → calc_damage(15, 5) = 13
    apply_action(&mut world, attacker, &Action::UseAbility { ability: sneak_attack, target: Some(target) });
    let hp_after = world.get::<Health>(target).unwrap().current;
    assert!(hp_after < 100, "sneak attack should deal damage");
    // Damage should be higher than a normal attack would give against same defense
    assert!(100 - hp_after > 5, "sneak attack should deal more than normal attack against armored target");
}

#[test]
fn shield_bash_drains_target_ap() {
    let mut world = World::new();
    let mut abilities = available_abilities(&CharacterClass::Warrior, 5);
    let shield_bash = abilities.remove(1); // Shield Bash is index 1 at level 5
    assert_eq!(shield_bash.name, "Shield Bash");

    let attacker = world.spawn((stats(10, 5), ActionPoints::new(4))).id();
    let target = world.spawn((stats(5, 0), Health::new(100), ActionPoints::new(4))).id();

    let result = apply_action(&mut world, attacker, &Action::UseAbility { ability: shield_bash, target: Some(target) });

    assert!(result.is_some());
    assert_eq!(world.get::<ActionPoints>(target).unwrap().current, 3); // drained 1 AP
}

#[test]
fn adrenaline_rush_grants_extra_ap() {
    let mut world = World::new();
    let abilities = available_abilities(&CharacterClass::Warrior, 12);
    let adrenaline = abilities.into_iter().find(|a| a.name == "Adrenaline Rush").unwrap();
    assert_eq!(adrenaline.ap_cost, 0);

    let actor = world.spawn(ActionPoints::new(4)).id();
    let result = apply_action(&mut world, actor, &Action::UseAbility { ability: adrenaline, target: None });

    assert!(result.is_some());
    // Grants 2 AP with 0 cost → now has 6 AP
    assert_eq!(world.get::<ActionPoints>(actor).unwrap().current, 6);
}

#[test]
fn heal_restores_hp() {
    let mut world = World::new();
    let heal = available_abilities(&CharacterClass::Cleric, 1).remove(0);
    assert_eq!(heal.name, "Heal");

    let healer = world.spawn(ActionPoints::new(4)).id();
    let mut target_hp = Health::new(100);
    target_hp.take_damage(50);
    let target = world.spawn(target_hp).id();

    apply_action(&mut world, healer, &Action::UseAbility { ability: heal, target: Some(target) });
    assert_eq!(world.get::<Health>(target).unwrap().current, 70); // 50 + 20 healed
}

#[test]
fn heal_cannot_exceed_max_hp() {
    let mut world = World::new();
    let heal = available_abilities(&CharacterClass::Cleric, 1).remove(0);

    let healer = world.spawn(ActionPoints::new(4)).id();
    let target = world.spawn(Health::new(100)).id(); // full HP

    apply_action(&mut world, healer, &Action::UseAbility { ability: heal, target: Some(target) });
    assert_eq!(world.get::<Health>(target).unwrap().current, 100); // capped at max
}

#[test]
fn system_hack_drains_target_ap() {
    let mut world = World::new();
    let hack = available_abilities(&CharacterClass::Ranger, 5)
        .into_iter()
        .find(|a| a.name == "System Hack")
        .unwrap();
    assert!(matches!(hack.effect, AbilityEffect::DrainAP { amount: 2 }));

    let attacker = world.spawn(ActionPoints::new(4)).id();
    let target = world.spawn(ActionPoints::new(4)).id();

    apply_action(&mut world, attacker, &Action::UseAbility { ability: hack, target: Some(target) });
    assert_eq!(world.get::<ActionPoints>(target).unwrap().current, 2); // 4 - 2
}

#[test]
fn precision_barrage_pierces_and_deals_bonus_damage() {
    let mut world = World::new();
    let barrage = available_abilities(&CharacterClass::Ranger, 10)
        .into_iter()
        .find(|a| a.name == "Precision Barrage")
        .unwrap();
    assert!(matches!(
        barrage.effect,
        AbilityEffect::ArmorPiercingStrike { pierce_fraction: _, bonus: _ }
    ));

    let attacker = world.spawn((stats(12, 5), ActionPoints::new(5))).id();
    let target = world.spawn((stats(5, 10), Health::new(100))).id();

    apply_action(&mut world, attacker, &Action::UseAbility { ability: barrage, target: Some(target) });
    assert!(world.get::<Health>(target).unwrap().current < 100);
}

#[test]
fn ability_on_dead_target_fails() {
    let mut world = World::new();
    let ability = available_abilities(&CharacterClass::Warrior, 1).remove(0);

    let attacker = world.spawn((stats(10, 5), ActionPoints::new(4))).id();
    let mut dead_hp = Health::new(50);
    dead_hp.take_damage(50);
    let dead = world.spawn((stats(5, 0), dead_hp)).id();

    let result = apply_action(&mut world, attacker, &Action::UseAbility { ability, target: Some(dead) });
    assert!(result.is_none());
}

#[test]
fn greater_heal_restores_more_than_basic_heal() {
    let basic = available_abilities(&CharacterClass::Cleric, 1)
        .into_iter().find(|a| a.name == "Heal").unwrap();
    let greater = available_abilities(&CharacterClass::Cleric, 7)
        .into_iter().find(|a| a.name == "Greater Heal").unwrap();

    let basic_amount = match basic.effect { AbilityEffect::Heal { amount } => amount, _ => 0 };
    let greater_amount = match greater.effect { AbilityEffect::Heal { amount } => amount, _ => 0 };
    assert!(greater_amount > basic_amount);
}
