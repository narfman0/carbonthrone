use bevy::prelude::Component;

use crate::character::CharacterKind;

/// Classifies how an ability is used in relation to the target's position.
#[derive(Debug, Clone, PartialEq)]
pub enum AbilityKind {
    /// Only usable when the target is within Chebyshev distance 1 (any adjacent tile,
    /// including diagonals). Each character has at most one melee attack.
    Melee,
    /// Usable at any range. Each character has at most one ranged weapon
    /// (abilities that deal damage). Disruption/utility ranged abilities are uncapped.
    Ranged,
    /// Self-targeted or battlefield-wide; no positional restriction.
    Utility,
}

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

/// A character ability that can be used in combat.
#[derive(Debug, Clone)]
pub struct Ability {
    pub name: &'static str,
    pub description: &'static str,
    /// Minimum character level required to use this ability.
    pub level_required: u32,
    /// Action-point cost to activate.
    pub ap_cost: i32,
    pub effect: AbilityEffect,
    /// Positional constraint: Melee requires adjacency; Ranged works at any distance.
    pub kind: AbilityKind,
}

/// Bevy component that records which character this entity is for ability queries.
/// Pair with `Experience` to call `available()` at runtime.
#[derive(Debug, Clone, Component)]
pub struct CharacterAbilities {
    pub character: CharacterKind,
}

impl CharacterAbilities {
    pub fn new(character: CharacterKind) -> Self {
        Self { character }
    }

    /// Returns abilities unlocked at or below `level`.
    pub fn available(&self, level: u32) -> Vec<Ability> {
        available_abilities(&self.character, level)
    }
}

/// Returns all abilities defined for `character`.
pub fn character_abilities(character: &CharacterKind) -> Vec<Ability> {
    match character {
        // Player characters
        CharacterKind::Researcher => researcher_abilities(),
        CharacterKind::Orin => orin_abilities(),
        CharacterKind::Doss => doss_abilities(),
        CharacterKind::Kaleo => kaleo_abilities(),
        // The Constancy
        CharacterKind::Zealot => zealot_abilities(),
        CharacterKind::Preacher => preacher_abilities(),
        CharacterKind::Purifier => purifier_abilities(),
        CharacterKind::Archon => archon_abilities(),
        // Drifters
        CharacterKind::Scavenger => scavenger_abilities(),
        CharacterKind::VoidRaider => void_raider_abilities(),
        CharacterKind::DrifterBoss => drifter_boss_abilities(),
        // Automata
        CharacterKind::MaintenanceDrone => maintenance_drone_abilities(),
        CharacterKind::SecurityUnit => security_unit_abilities(),
        CharacterKind::CombatFrame => combat_frame_abilities(),
        // Abyssal Fauna
        CharacterKind::MoonCrawler => moon_crawler_abilities(),
        CharacterKind::VoidSpitter => void_spitter_abilities(),
        CharacterKind::AbyssalBrute => abyssal_brute_abilities(),
        // Station Personnel
        CharacterKind::SalvageOperative => salvage_operative_abilities(),
        CharacterKind::GunForHire => gun_for_hire_abilities(),
        CharacterKind::StationGuard => station_guard_abilities(),
        CharacterKind::ShockTrooper => shock_trooper_abilities(),
    }
}

/// Returns abilities for `character` unlocked at or below `level`.
pub fn available_abilities(character: &CharacterKind, level: u32) -> Vec<Ability> {
    character_abilities(character)
        .into_iter()
        .filter(|a| a.level_required <= level)
        .collect()
}

// ── Player character ability tables ───────────────────────────────────────────

fn researcher_abilities() -> Vec<Ability> {
    vec![
        Ability {
            name: "Temporal Bolt",
            description: "A focused burst of temporal energy fired at range, dealing significant bonus damage.",
            level_required: 1,
            ap_cost: 3,
            effect: AbilityEffect::BonusDamage { bonus: 10 },
            kind: AbilityKind::Ranged,
        },
        Ability {
            name: "Stasis",
            description: "Lock an enemy in a temporal freeze at range, draining their action economy.",
            level_required: 6,
            ap_cost: 2,
            effect: AbilityEffect::DrainAP { amount: 3 },
            kind: AbilityKind::Ranged,
        },
        Ability {
            name: "Rewind",
            description: "Reverse an ally's recent injuries, restoring a substantial amount of HP.",
            level_required: 12,
            ap_cost: 3,
            effect: AbilityEffect::Heal { amount: 35 },
            kind: AbilityKind::Utility,
        },
    ]
}

fn doss_abilities() -> Vec<Ability> {
    vec![
        Ability {
            name: "Power Strike",
            description: "A brutal melee blow that deals significant bonus damage.",
            level_required: 1,
            ap_cost: 3,
            effect: AbilityEffect::BonusDamage { bonus: 8 },
            kind: AbilityKind::Melee,
        },
        Ability {
            name: "Shield Bash",
            description: "Strike the enemy with your shield in close quarters, disrupting their next action.",
            level_required: 5,
            ap_cost: 2,
            effect: AbilityEffect::DrainAP { amount: 1 },
            kind: AbilityKind::Melee,
        },
        Ability {
            name: "Adrenaline Rush",
            description: "Push through pain and exhaustion, gaining extra actions this turn.",
            level_required: 12,
            ap_cost: 0,
            effect: AbilityEffect::GrantAP { amount: 2 },
            kind: AbilityKind::Utility,
        },
    ]
}

fn orin_abilities() -> Vec<Ability> {
    vec![
        Ability {
            name: "Heal",
            description: "Channel restorative energy to mend an ally's wounds.",
            level_required: 1,
            ap_cost: 2,
            effect: AbilityEffect::Heal { amount: 20 },
            kind: AbilityKind::Utility,
        },
        Ability {
            name: "Greater Heal",
            description: "A powerful surge of healing that restores significant HP.",
            level_required: 7,
            ap_cost: 3,
            effect: AbilityEffect::Heal { amount: 45 },
            kind: AbilityKind::Utility,
        },
        Ability {
            name: "Divine Restoration",
            description: "A transcendent healing wave that nearly fully restores an ally.",
            level_required: 14,
            ap_cost: 4,
            effect: AbilityEffect::Heal { amount: 80 },
            kind: AbilityKind::Utility,
        },
    ]
}

fn kaleo_abilities() -> Vec<Ability> {
    vec![
        Ability {
            name: "Aimed Shot",
            description: "A carefully lined-up ranged shot that deals bonus damage.",
            level_required: 1,
            ap_cost: 2,
            effect: AbilityEffect::BonusDamage { bonus: 5 },
            kind: AbilityKind::Ranged,
        },
        Ability {
            name: "System Hack",
            description: "Interface with enemy systems at range to disrupt their action economy.",
            level_required: 5,
            ap_cost: 2,
            effect: AbilityEffect::DrainAP { amount: 2 },
            kind: AbilityKind::Ranged,
        },
        Ability {
            name: "Precision Barrage",
            description: "A sustained close-range volley of precise fire that shreds armor and deals heavy damage.",
            level_required: 10,
            ap_cost: 4,
            effect: AbilityEffect::ArmorPiercingStrike {
                pierce_fraction: 0.5,
                bonus: 10,
            },
            kind: AbilityKind::Melee,
        },
    ]
}

// ── NPC ability tables ─────────────────────────────────────────────────────────

// The Constancy

fn zealot_abilities() -> Vec<Ability> {
    vec![Ability {
        name: "Zealot Strike",
        description: "A fanatical melee charge driven by absolute conviction.",
        level_required: 1,
        ap_cost: 2,
        effect: AbilityEffect::BonusDamage { bonus: 5 },
        kind: AbilityKind::Melee,
    }]
}

fn preacher_abilities() -> Vec<Ability> {
    vec![Ability {
        name: "Dampen Temporal Field",
        description: "Emit a suppression pulse that drains a target's temporal action economy.",
        level_required: 1,
        ap_cost: 2,
        effect: AbilityEffect::DrainAP { amount: 1 },
        kind: AbilityKind::Ranged,
    }]
}

fn purifier_abilities() -> Vec<Ability> {
    vec![Ability {
        name: "Purifying Round",
        description: "An anti-temporal round fired from range that deals bonus damage.",
        level_required: 1,
        ap_cost: 2,
        effect: AbilityEffect::BonusDamage { bonus: 8 },
        kind: AbilityKind::Ranged,
    }]
}

fn archon_abilities() -> Vec<Ability> {
    vec![
        Ability {
            name: "Armored Crush",
            description: "A devastating melee blow from a heavily armored frame that pierces defenses.",
            level_required: 1,
            ap_cost: 3,
            effect: AbilityEffect::ArmorPiercing {
                pierce_fraction: 0.3,
            },
            kind: AbilityKind::Melee,
        },
        Ability {
            name: "Temporal Suppression",
            description: "Activate the zone-wide Flux dampener, draining a target's action points.",
            level_required: 1,
            ap_cost: 2,
            effect: AbilityEffect::DrainAP { amount: 2 },
            kind: AbilityKind::Ranged,
        },
    ]
}

// Drifters

fn scavenger_abilities() -> Vec<Ability> {
    vec![Ability {
        name: "Quick Slash",
        description: "A fast, opportunistic melee strike from a glass-cannon scavenger.",
        level_required: 1,
        ap_cost: 2,
        effect: AbilityEffect::BonusDamage { bonus: 3 },
        kind: AbilityKind::Melee,
    }]
}

fn void_raider_abilities() -> Vec<Ability> {
    vec![Ability {
        name: "Suppressive Fire",
        description: "A burst of gunfire from range that keeps targets pinned.",
        level_required: 1,
        ap_cost: 2,
        effect: AbilityEffect::BonusDamage { bonus: 5 },
        kind: AbilityKind::Ranged,
    }]
}

fn drifter_boss_abilities() -> Vec<Ability> {
    vec![
        Ability {
            name: "Heavy Blow",
            description: "A crushing melee strike from the pack leader.",
            level_required: 1,
            ap_cost: 3,
            effect: AbilityEffect::BonusDamage { bonus: 10 },
            kind: AbilityKind::Melee,
        },
        Ability {
            name: "Rally",
            description: "Bark orders to push through the fight, gaining a burst of extra action.",
            level_required: 1,
            ap_cost: 0,
            effect: AbilityEffect::GrantAP { amount: 1 },
            kind: AbilityKind::Utility,
        },
    ]
}

// Automata

fn maintenance_drone_abilities() -> Vec<Ability> {
    vec![Ability {
        name: "Maintenance Strike",
        description: "An erratic melee blow from a corrupted maintenance unit.",
        level_required: 1,
        ap_cost: 2,
        effect: AbilityEffect::BonusDamage { bonus: 2 },
        kind: AbilityKind::Melee,
    }]
}

fn security_unit_abilities() -> Vec<Ability> {
    vec![Ability {
        name: "Security Fire",
        description: "A controlled ranged burst from a corrupted security patrol unit.",
        level_required: 1,
        ap_cost: 2,
        effect: AbilityEffect::BonusDamage { bonus: 6 },
        kind: AbilityKind::Ranged,
    }]
}

fn combat_frame_abilities() -> Vec<Ability> {
    vec![
        Ability {
            name: "Mech Strike",
            description: "A punishing melee blow from a military-grade combat frame that shreds armor.",
            level_required: 1,
            ap_cost: 3,
            effect: AbilityEffect::ArmorPiercingStrike {
                pierce_fraction: 0.4,
                bonus: 8,
            },
            kind: AbilityKind::Melee,
        },
        Ability {
            name: "Heavy Cannon",
            description: "A devastating ranged salvo from the combat frame's main armament.",
            level_required: 1,
            ap_cost: 3,
            effect: AbilityEffect::BonusDamage { bonus: 15 },
            kind: AbilityKind::Ranged,
        },
    ]
}

// Abyssal Fauna

fn moon_crawler_abilities() -> Vec<Ability> {
    vec![Ability {
        name: "Feral Pounce",
        description: "A lightning-fast melee lunge from the fastest creature on the moon.",
        level_required: 1,
        ap_cost: 2,
        effect: AbilityEffect::BonusDamage { bonus: 4 },
        kind: AbilityKind::Melee,
    }]
}

fn void_spitter_abilities() -> Vec<Ability> {
    vec![Ability {
        name: "Bio Spit",
        description: "A ranged biological projectile with magic-adjacent corrosive damage.",
        level_required: 1,
        ap_cost: 2,
        effect: AbilityEffect::BonusDamage { bonus: 7 },
        kind: AbilityKind::Ranged,
    }]
}

fn abyssal_brute_abilities() -> Vec<Ability> {
    vec![Ability {
        name: "Crushing Blow",
        description: "A slow but devastating melee strike from the apex predator that pierces armor.",
        level_required: 1,
        ap_cost: 3,
        effect: AbilityEffect::ArmorPiercing {
            pierce_fraction: 0.5,
        },
        kind: AbilityKind::Melee,
    }]
}

// Station Personnel

fn salvage_operative_abilities() -> Vec<Ability> {
    vec![Ability {
        name: "Quick Shot",
        description: "A hasty ranged shot from a lightly armed mercenary.",
        level_required: 1,
        ap_cost: 2,
        effect: AbilityEffect::BonusDamage { bonus: 3 },
        kind: AbilityKind::Ranged,
    }]
}

fn gun_for_hire_abilities() -> Vec<Ability> {
    vec![Ability {
        name: "Contract Shot",
        description: "A professional ranged shot from an armed contractor.",
        level_required: 1,
        ap_cost: 2,
        effect: AbilityEffect::BonusDamage { bonus: 7 },
        kind: AbilityKind::Ranged,
    }]
}

fn station_guard_abilities() -> Vec<Ability> {
    vec![
        Ability {
            name: "Security Baton",
            description: "A close-quarters melee strike with a security enforcement baton.",
            level_required: 1,
            ap_cost: 2,
            effect: AbilityEffect::BonusDamage { bonus: 4 },
            kind: AbilityKind::Melee,
        },
        Ability {
            name: "Warning Shot",
            description: "A ranged shot from station security — effective at any range.",
            level_required: 1,
            ap_cost: 2,
            effect: AbilityEffect::BonusDamage { bonus: 5 },
            kind: AbilityKind::Ranged,
        },
    ]
}

fn shock_trooper_abilities() -> Vec<Ability> {
    vec![
        Ability {
            name: "Shock Assault",
            description: "A brutal close-quarters assault that pierces armor and deals heavy damage.",
            level_required: 1,
            ap_cost: 3,
            effect: AbilityEffect::ArmorPiercingStrike {
                pierce_fraction: 0.2,
                bonus: 5,
            },
            kind: AbilityKind::Melee,
        },
        Ability {
            name: "Assault Fire",
            description: "A disciplined burst of automatic fire from a military enforcer.",
            level_required: 1,
            ap_cost: 2,
            effect: AbilityEffect::BonusDamage { bonus: 10 },
            kind: AbilityKind::Ranged,
        },
    ]
}
