use crate::stats::Stats;
use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Component, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterKind {
    // Player characters
    /// The player character — Temporal Mage; high magic, fragile, fast.
    Researcher,
    /// Dr. Sable Orin — Cleric; support healer, high defense, low offense.
    Orin,
    /// Recruiter Doss — Warrior; heavy armor, high HP, strong melee.
    Doss,
    /// Unit Kaleo — Ranger; precision attacks, system hacking, very fast.
    Kaleo,

    // The Constancy — anti-temporal zealots
    Zealot,   // fast melee; glass cannon; first enemies encountered
    Preacher, // support; aura suppresses Flux generation
    Purifier, // ranged; anti-temporal rounds; bonus damage to high-Flux units
    Archon,   // boss; heavily armored; zone-wide Flux suppression

    // Drifters — scavengers and bandits
    Scavenger,   // fast light melee; glass cannon
    VoidRaider,  // ranged gunslinger; balanced
    DrifterBoss, // heavy bruiser; high attack, pack leader

    // Automata — corrupted station robotics
    MaintenanceDrone, // confused melee; erratic; medium defense
    SecurityUnit,     // ranged patrol; medium-high stats
    CombatFrame,      // military-grade mech; boss tier

    // Abyssal Fauna — native moon creatures
    MoonCrawler,  // fastest enemy; pack melee; dies easily
    VoidSpitter,  // ranged bio attack; magic-adjacent
    AbyssalBrute, // slow tanky apex predator

    // Station Personnel — crew and contractors
    SalvageOperative, // lightly armed mercenary; friendly by default
    GunForHire,       // armed contractor; neutral by default
    StationGuard,     // station security; friendly early, hostile late
    ShockTrooper,     // military enforcer; always aggressive
}

impl CharacterKind {
    pub fn is_player(&self) -> bool {
        matches!(
            self,
            CharacterKind::Researcher
                | CharacterKind::Orin
                | CharacterKind::Doss
                | CharacterKind::Kaleo
        )
    }
}

/// Base stats at level 1.
pub(crate) fn base_stats(kind: &CharacterKind) -> Stats {
    match kind {
        // Player characters
        CharacterKind::Researcher => Stats {
            max_hp: 75,
            attack: 9,
            defense: 4,
            speed: 16,
        },
        CharacterKind::Orin => Stats {
            max_hp: 90,
            attack: 8,
            defense: 8,
            speed: 9,
        },
        CharacterKind::Doss => Stats {
            max_hp: 120,
            attack: 15,
            defense: 12,
            speed: 8,
        },
        CharacterKind::Kaleo => Stats {
            max_hp: 85,
            attack: 12,
            defense: 7,
            speed: 15,
        },
        // The Constancy
        CharacterKind::Zealot => Stats {
            max_hp: 45,
            attack: 13,
            defense: 4,
            speed: 14,
        },
        CharacterKind::Preacher => Stats {
            max_hp: 50,
            attack: 8,
            defense: 6,
            speed: 9,
        },
        CharacterKind::Purifier => Stats {
            max_hp: 55,
            attack: 14,
            defense: 5,
            speed: 10,
        },
        CharacterKind::Archon => Stats {
            max_hp: 220,
            attack: 20,
            defense: 18,
            speed: 8,
        },
        // Drifters
        CharacterKind::Scavenger => Stats {
            max_hp: 40,
            attack: 9,
            defense: 3,
            speed: 15,
        },
        CharacterKind::VoidRaider => Stats {
            max_hp: 55,
            attack: 12,
            defense: 5,
            speed: 11,
        },
        CharacterKind::DrifterBoss => Stats {
            max_hp: 95,
            attack: 17,
            defense: 8,
            speed: 7,
        },
        // Automata
        CharacterKind::MaintenanceDrone => Stats {
            max_hp: 65,
            attack: 8,
            defense: 10,
            speed: 7,
        },
        CharacterKind::SecurityUnit => Stats {
            max_hp: 80,
            attack: 13,
            defense: 11,
            speed: 9,
        },
        CharacterKind::CombatFrame => Stats {
            max_hp: 280,
            attack: 24,
            defense: 20,
            speed: 10,
        },
        // Abyssal Fauna
        CharacterKind::MoonCrawler => Stats {
            max_hp: 35,
            attack: 11,
            defense: 2,
            speed: 18,
        },
        CharacterKind::VoidSpitter => Stats {
            max_hp: 55,
            attack: 14,
            defense: 4,
            speed: 10,
        },
        CharacterKind::AbyssalBrute => Stats {
            max_hp: 160,
            attack: 13,
            defense: 15,
            speed: 4,
        },
        // Station Personnel
        CharacterKind::SalvageOperative => Stats {
            max_hp: 55,
            attack: 9,
            defense: 6,
            speed: 10,
        },
        CharacterKind::GunForHire => Stats {
            max_hp: 75,
            attack: 15,
            defense: 9,
            speed: 10,
        },
        CharacterKind::StationGuard => Stats {
            max_hp: 65,
            attack: 12,
            defense: 10,
            speed: 10,
        },
        CharacterKind::ShockTrooper => Stats {
            max_hp: 100,
            attack: 19,
            defense: 13,
            speed: 8,
        },
    }
}

/// Per-level stat growth: (hp, atk, def, spd).
pub(crate) fn growth(kind: &CharacterKind) -> (i32, i32, i32, i32) {
    match kind {
        // Player characters
        CharacterKind::Researcher => (9, 2, 1, 2),
        CharacterKind::Orin => (12, 1, 2, 1),
        CharacterKind::Doss => (18, 3, 3, 1),
        CharacterKind::Kaleo => (11, 2, 1, 2),
        // The Constancy
        CharacterKind::Zealot => (5, 2, 0, 1),
        CharacterKind::Preacher => (6, 1, 1, 0),
        CharacterKind::Purifier => (7, 2, 1, 0),
        CharacterKind::Archon => (30, 4, 4, 0),
        // Drifters
        CharacterKind::Scavenger => (5, 1, 0, 1),
        CharacterKind::VoidRaider => (7, 2, 1, 0),
        CharacterKind::DrifterBoss => (12, 3, 1, 0),
        // Automata
        CharacterKind::MaintenanceDrone => (9, 1, 2, 0),
        CharacterKind::SecurityUnit => (10, 2, 2, 0),
        CharacterKind::CombatFrame => (38, 5, 4, 1),
        // Abyssal Fauna
        CharacterKind::MoonCrawler => (4, 2, 0, 1),
        CharacterKind::VoidSpitter => (7, 2, 0, 0),
        CharacterKind::AbyssalBrute => (22, 2, 2, 0),
        // Station Personnel
        CharacterKind::SalvageOperative => (6, 1, 1, 0),
        CharacterKind::GunForHire => (10, 2, 1, 0),
        CharacterKind::StationGuard => (8, 2, 1, 0),
        CharacterKind::ShockTrooper => (13, 3, 2, 0),
    }
}

pub(crate) fn scaled_stats(kind: &CharacterKind, level: u32) -> Stats {
    let mut s = base_stats(kind);
    let (hp, atk, def, spd) = growth(kind);
    let extra = (level - 1) as i32;
    s.max_hp += hp * extra;
    s.attack += atk * extra;
    s.defense += def * extra;
    s.speed += spd * extra;
    s
}

pub(crate) fn base_xp(kind: &CharacterKind) -> u32 {
    match kind {
        // Player characters — yield no XP
        CharacterKind::Researcher => 0,
        CharacterKind::Orin => 0,
        CharacterKind::Doss => 0,
        CharacterKind::Kaleo => 0,
        // The Constancy
        CharacterKind::Zealot => 22,
        CharacterKind::Preacher => 35,
        CharacterKind::Purifier => 30,
        CharacterKind::Archon => 200,
        // Drifters
        CharacterKind::Scavenger => 20,
        CharacterKind::VoidRaider => 28,
        CharacterKind::DrifterBoss => 45,
        // Automata
        CharacterKind::MaintenanceDrone => 35,
        CharacterKind::SecurityUnit => 40,
        CharacterKind::CombatFrame => 240,
        // Abyssal Fauna
        CharacterKind::MoonCrawler => 18,
        CharacterKind::VoidSpitter => 30,
        CharacterKind::AbyssalBrute => 70,
        // Station Personnel
        CharacterKind::SalvageOperative => 20,
        CharacterKind::GunForHire => 50,
        CharacterKind::StationGuard => 30,
        CharacterKind::ShockTrooper => 60,
    }
}

pub(crate) fn default_name(kind: &CharacterKind) -> &'static str {
    match kind {
        // Player characters
        CharacterKind::Researcher => "Researcher",
        CharacterKind::Orin => "Dr. Orin",
        CharacterKind::Doss => "Doss",
        CharacterKind::Kaleo => "Kaleo",
        CharacterKind::Zealot => "Zealot",
        CharacterKind::Preacher => "Preacher",
        CharacterKind::Purifier => "Purifier",
        CharacterKind::Archon => "Archon",
        CharacterKind::Scavenger => "Scavenger",
        CharacterKind::VoidRaider => "Void Raider",
        CharacterKind::DrifterBoss => "Drifter Boss",
        CharacterKind::MaintenanceDrone => "Maintenance Drone",
        CharacterKind::SecurityUnit => "Security Unit",
        CharacterKind::CombatFrame => "Combat Frame",
        CharacterKind::MoonCrawler => "Moon Crawler",
        CharacterKind::VoidSpitter => "Void Spitter",
        CharacterKind::AbyssalBrute => "Abyssal Brute",
        CharacterKind::SalvageOperative => "Salvage Operative",
        CharacterKind::GunForHire => "Gun-for-Hire",
        CharacterKind::StationGuard => "Station Guard",
        CharacterKind::ShockTrooper => "Shock Trooper",
    }
}

pub(crate) fn default_aggression(kind: &CharacterKind) -> Aggression {
    match kind {
        // Player characters
        CharacterKind::Researcher => Aggression::Friendly,
        CharacterKind::Orin => Aggression::Friendly,
        CharacterKind::Doss => Aggression::Friendly,
        CharacterKind::Kaleo => Aggression::Friendly,
        // The Constancy — always aggressive, no exceptions
        CharacterKind::Zealot => Aggression::Aggressive,
        CharacterKind::Preacher => Aggression::Aggressive,
        CharacterKind::Purifier => Aggression::Aggressive,
        CharacterKind::Archon => Aggression::Aggressive,
        // Drifters — always hostile
        CharacterKind::Scavenger => Aggression::Aggressive,
        CharacterKind::VoidRaider => Aggression::Aggressive,
        CharacterKind::DrifterBoss => Aggression::Aggressive,
        // Automata — Security hostile; Maintenance drones wander until provoked
        CharacterKind::MaintenanceDrone => Aggression::Neutral,
        CharacterKind::SecurityUnit => Aggression::Aggressive,
        CharacterKind::CombatFrame => Aggression::Aggressive,
        // Abyssal Fauna — aggressive by nature
        CharacterKind::MoonCrawler => Aggression::Aggressive,
        CharacterKind::VoidSpitter => Aggression::Aggressive,
        CharacterKind::AbyssalBrute => Aggression::Aggressive,
        // Station Personnel — personnel start friendly or neutral; hostile later
        CharacterKind::SalvageOperative => Aggression::Friendly,
        CharacterKind::GunForHire => Aggression::Neutral,
        CharacterKind::StationGuard => Aggression::Friendly,
        CharacterKind::ShockTrooper => Aggression::Aggressive,
    }
}

/// Returns the aggression state for a character kind at the given loop number.
///
/// Aggression evolves over loops as the station degrades and temporal instability peaks:
/// - **Maintenance Drones**: Neutral loops 1–2; Aggressive loop 3+
/// - **Station Guards**: Friendly loops 1–2; Neutral loop 3; Aggressive loops 4–5
/// - **Salvage Operatives**: Friendly loops 1–2; Neutral loop 3; Aggressive loops 4–5
/// - **Gun-for-Hire**: Neutral loops 1–4; Aggressive loop 5 (Doss turns them)
/// - **Abyssal Fauna** (Moon Crawler, Void Spitter, Abyssal Brute): Aggressive loops 1–3;
///   Lethargic loops 4–5 (temporal flux collapses their instincts)
/// - All other kinds return their default aggression unchanged.
pub fn loop_aggression(kind: &CharacterKind, loop_number: u32) -> Aggression {
    match kind {
        // Automata — Maintenance Drones go hostile as station degrades
        CharacterKind::MaintenanceDrone => {
            if loop_number >= 3 {
                Aggression::Aggressive
            } else {
                Aggression::Neutral
            }
        }
        // Station Personnel — flip from friendly to hostile over loops
        CharacterKind::StationGuard => {
            if loop_number <= 2 {
                Aggression::Friendly
            } else if loop_number == 3 {
                Aggression::Neutral
            } else {
                Aggression::Aggressive
            }
        }
        CharacterKind::SalvageOperative => {
            if loop_number <= 2 {
                Aggression::Friendly
            } else if loop_number == 3 {
                Aggression::Neutral
            } else {
                Aggression::Aggressive
            }
        }
        CharacterKind::GunForHire => {
            if loop_number >= 5 {
                Aggression::Aggressive
            } else {
                Aggression::Neutral
            }
        }
        // Abyssal Fauna — go Lethargic in loops 4–5 as temporal flux peaks
        CharacterKind::MoonCrawler | CharacterKind::VoidSpitter | CharacterKind::AbyssalBrute => {
            if loop_number >= 4 {
                Aggression::Lethargic
            } else {
                Aggression::Aggressive
            }
        }
        _ => default_aggression(kind),
    }
}

/// Current disposition of this character toward the player party.
#[derive(Debug, Clone, PartialEq, Component)]
pub enum Aggression {
    /// Attacks the party on sight.
    Aggressive,
    /// Ignores the party unless provoked or bribed.
    Neutral,
    /// Will not initiate combat; may trade or give information.
    Friendly,
    /// Passive, slow, and non-initiating — temporal instability has dulled the creature.
    Lethargic,
}

/// Universal entity component for both player characters and NPCs.
/// Use `aggression` to determine combat role; use `kind` to access class-specific data.
/// Use `kind.is_player()` to distinguish player characters from NPCs.
#[derive(Debug, Clone, Component)]
pub struct Character {
    pub name: String,
    pub kind: CharacterKind,
    pub level: u32,
    pub stats: Stats,
    pub current_hp: i32,
    pub xp_reward: u32,
    pub aggression: Aggression,
}

impl Character {
    /// Create a character at the given level with scaled stats, default name, and default aggression.
    pub fn new_character(kind: CharacterKind, level: u32) -> Self {
        let stats = scaled_stats(&kind, level);
        let current_hp = stats.max_hp;
        let xp_reward = base_xp(&kind) * level;
        let name = default_name(&kind).to_string();
        let aggression = default_aggression(&kind);
        Self {
            name,
            kind,
            level,
            stats,
            current_hp,
            xp_reward,
            aggression,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.current_hp > 0
    }

    pub fn take_damage(&mut self, amount: i32) {
        self.current_hp = (self.current_hp - amount).max(0);
    }
}
