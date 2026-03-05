use bevy::prelude::Component;

use crate::character::CharacterClass;

/// The mechanical effect of using an ability in combat.
#[derive(Debug, Clone, PartialEq)]
pub enum AbilityEffect {
    /// Deal bonus damage on an attack (normal hit check applies).
    BonusDamage { bonus: i32 },
    /// Attack that ignores `pierce_fraction` of the defender's defense (0.0–1.0).
    ArmorPiercing { pierce_fraction: f32 },
    /// Armor-piercing strike with additional bonus damage.
    ArmorPiercingStrike { pierce_fraction: f32, bonus: i32 },
    /// Restore HP to the target.
    Heal { amount: i32 },
    /// Reduce the target's current action points.
    DrainAP { amount: i32 },
    /// Grant the caster additional action points this turn.
    GrantAP { amount: i32 },
}

/// A class ability that can be used in combat.
#[derive(Debug, Clone)]
pub struct Ability {
    pub name: &'static str,
    pub description: &'static str,
    /// Minimum character level required to use this ability.
    pub level_required: u32,
    /// Action-point cost to activate.
    pub ap_cost: i32,
    pub effect: AbilityEffect,
}

/// Bevy component that records a character's class for ability queries.
/// Pair with `Experience` to call `available()` at runtime.
#[derive(Debug, Clone, Component)]
pub struct ClassAbilities {
    pub class: CharacterClass,
}

impl ClassAbilities {
    pub fn new(class: CharacterClass) -> Self {
        Self { class }
    }

    /// Returns abilities unlocked at or below `level`.
    pub fn available(&self, level: u32) -> Vec<Ability> {
        available_abilities(&self.class, level)
    }
}

/// Returns all abilities defined for `class`.
pub fn class_abilities(class: &CharacterClass) -> Vec<Ability> {
    match class {
        CharacterClass::Warrior => warrior_abilities(),
        CharacterClass::Rogue => rogue_abilities(),
        CharacterClass::Cleric => cleric_abilities(),
        CharacterClass::Ranger => ranger_abilities(),
    }
}

/// Returns abilities for `class` unlocked at or below `level`.
pub fn available_abilities(class: &CharacterClass, level: u32) -> Vec<Ability> {
    class_abilities(class)
        .into_iter()
        .filter(|a| a.level_required <= level)
        .collect()
}

// ── Per-class ability tables ──────────────────────────────────────────────────

fn warrior_abilities() -> Vec<Ability> {
    vec![
        Ability {
            name: "Power Strike",
            description: "A brutal melee blow that deals significant bonus damage.",
            level_required: 1,
            ap_cost: 3,
            effect: AbilityEffect::BonusDamage { bonus: 8 },
        },
        Ability {
            name: "Shield Bash",
            description: "Strike the enemy with your shield, disrupting their next action.",
            level_required: 5,
            ap_cost: 2,
            effect: AbilityEffect::DrainAP { amount: 1 },
        },
        Ability {
            name: "Adrenaline Rush",
            description: "Push through pain and exhaustion, gaining extra actions this turn.",
            level_required: 12,
            ap_cost: 0,
            effect: AbilityEffect::GrantAP { amount: 2 },
        },
    ]
}

fn rogue_abilities() -> Vec<Ability> {
    vec![
        Ability {
            name: "Sneak Attack",
            description: "Strike a vulnerable point, bypassing most of the target's armor.",
            level_required: 1,
            ap_cost: 2,
            effect: AbilityEffect::ArmorPiercing { pierce_fraction: 0.75 },
        },
        Ability {
            name: "Shadow Step",
            description: "Blur through shadows, gaining momentum for rapid repositioning.",
            level_required: 6,
            ap_cost: 1,
            effect: AbilityEffect::GrantAP { amount: 3 },
        },
        Ability {
            name: "Assassination",
            description: "A precisely timed killing blow that deals massive bonus damage.",
            level_required: 12,
            ap_cost: 3,
            effect: AbilityEffect::BonusDamage { bonus: 25 },
        },
    ]
}

fn cleric_abilities() -> Vec<Ability> {
    vec![
        Ability {
            name: "Heal",
            description: "Channel restorative energy to mend an ally's wounds.",
            level_required: 1,
            ap_cost: 2,
            effect: AbilityEffect::Heal { amount: 20 },
        },
        Ability {
            name: "Greater Heal",
            description: "A powerful surge of healing that restores significant HP.",
            level_required: 7,
            ap_cost: 3,
            effect: AbilityEffect::Heal { amount: 45 },
        },
        Ability {
            name: "Divine Restoration",
            description: "A transcendent healing wave that nearly fully restores an ally.",
            level_required: 14,
            ap_cost: 4,
            effect: AbilityEffect::Heal { amount: 80 },
        },
    ]
}

fn ranger_abilities() -> Vec<Ability> {
    vec![
        Ability {
            name: "Aimed Shot",
            description: "A carefully lined-up shot that deals bonus damage.",
            level_required: 1,
            ap_cost: 2,
            effect: AbilityEffect::BonusDamage { bonus: 5 },
        },
        Ability {
            name: "System Hack",
            description: "Interface with enemy systems to disrupt their action economy.",
            level_required: 5,
            ap_cost: 2,
            effect: AbilityEffect::DrainAP { amount: 2 },
        },
        Ability {
            name: "Precision Barrage",
            description: "A sustained volley of precise fire that shreds armor and deals heavy damage.",
            level_required: 10,
            ap_cost: 4,
            effect: AbilityEffect::ArmorPiercingStrike { pierce_fraction: 0.5, bonus: 10 },
        },
    ]
}
