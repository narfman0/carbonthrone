use crate::stats::Stats;

#[derive(Debug, Clone, PartialEq, bevy::prelude::Component)]
pub enum NPCKind {
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

impl NPCKind {
    pub fn is_player(&self) -> bool {
        matches!(
            self,
            NPCKind::Researcher | NPCKind::Orin | NPCKind::Doss | NPCKind::Kaleo
        )
    }
}

/// Base stats at level 1. Only valid for NPC kinds.
pub(crate) fn base_stats(kind: &NPCKind) -> Stats {
    match kind {
        // The Constancy
        NPCKind::Zealot => Stats {
            max_hp: 45,
            attack: 13,
            defense: 4,
            speed: 14,
        },
        NPCKind::Preacher => Stats {
            max_hp: 50,
            attack: 8,
            defense: 6,
            speed: 9,
        },
        NPCKind::Purifier => Stats {
            max_hp: 55,
            attack: 14,
            defense: 5,
            speed: 10,
        },
        NPCKind::Archon => Stats {
            max_hp: 220,
            attack: 20,
            defense: 18,
            speed: 8,
        },
        // Drifters
        NPCKind::Scavenger => Stats {
            max_hp: 40,
            attack: 9,
            defense: 3,
            speed: 15,
        },
        NPCKind::VoidRaider => Stats {
            max_hp: 55,
            attack: 12,
            defense: 5,
            speed: 11,
        },
        NPCKind::DrifterBoss => Stats {
            max_hp: 95,
            attack: 17,
            defense: 8,
            speed: 7,
        },
        // Automata
        NPCKind::MaintenanceDrone => Stats {
            max_hp: 65,
            attack: 8,
            defense: 10,
            speed: 7,
        },
        NPCKind::SecurityUnit => Stats {
            max_hp: 80,
            attack: 13,
            defense: 11,
            speed: 9,
        },
        NPCKind::CombatFrame => Stats {
            max_hp: 280,
            attack: 24,
            defense: 20,
            speed: 10,
        },
        // Abyssal Fauna
        NPCKind::MoonCrawler => Stats {
            max_hp: 35,
            attack: 11,
            defense: 2,
            speed: 18,
        },
        NPCKind::VoidSpitter => Stats {
            max_hp: 55,
            attack: 14,
            defense: 4,
            speed: 10,
        },
        NPCKind::AbyssalBrute => Stats {
            max_hp: 160,
            attack: 13,
            defense: 15,
            speed: 4,
        },
        // Station Personnel
        NPCKind::SalvageOperative => Stats {
            max_hp: 55,
            attack: 9,
            defense: 6,
            speed: 10,
        },
        NPCKind::GunForHire => Stats {
            max_hp: 75,
            attack: 15,
            defense: 9,
            speed: 10,
        },
        NPCKind::StationGuard => Stats {
            max_hp: 65,
            attack: 12,
            defense: 10,
            speed: 10,
        },
        NPCKind::ShockTrooper => Stats {
            max_hp: 100,
            attack: 19,
            defense: 13,
            speed: 8,
        },
        _ => panic!("base_stats called on player character kind {:?}", kind),
    }
}

/// Per-level stat growth: (hp, atk, def, spd). Only valid for NPC kinds.
pub(crate) fn growth(kind: &NPCKind) -> (i32, i32, i32, i32) {
    match kind {
        // The Constancy
        NPCKind::Zealot => (5, 2, 0, 1),
        NPCKind::Preacher => (6, 1, 1, 0),
        NPCKind::Purifier => (7, 2, 1, 0),
        NPCKind::Archon => (30, 4, 4, 0),
        // Drifters
        NPCKind::Scavenger => (5, 1, 0, 1),
        NPCKind::VoidRaider => (7, 2, 1, 0),
        NPCKind::DrifterBoss => (12, 3, 1, 0),
        // Automata
        NPCKind::MaintenanceDrone => (9, 1, 2, 0),
        NPCKind::SecurityUnit => (10, 2, 2, 0),
        NPCKind::CombatFrame => (38, 5, 4, 1),
        // Abyssal Fauna
        NPCKind::MoonCrawler => (4, 2, 0, 1),
        NPCKind::VoidSpitter => (7, 2, 0, 0),
        NPCKind::AbyssalBrute => (22, 2, 2, 0),
        // Station Personnel
        NPCKind::SalvageOperative => (6, 1, 1, 0),
        NPCKind::GunForHire => (10, 2, 1, 0),
        NPCKind::StationGuard => (8, 2, 1, 0),
        NPCKind::ShockTrooper => (13, 3, 2, 0),
        _ => panic!("growth called on player character kind {:?}", kind),
    }
}

pub(crate) fn scaled_stats(kind: &NPCKind, level: u32) -> Stats {
    let mut s = base_stats(kind);
    let (hp, atk, def, spd) = growth(kind);
    let extra = (level - 1) as i32;
    s.max_hp += hp * extra;
    s.attack += atk * extra;
    s.defense += def * extra;
    s.speed += spd * extra;
    s
}

pub(crate) fn base_xp(kind: &NPCKind) -> u32 {
    match kind {
        // The Constancy
        NPCKind::Zealot => 22,
        NPCKind::Preacher => 35,
        NPCKind::Purifier => 30,
        NPCKind::Archon => 200,
        // Drifters
        NPCKind::Scavenger => 20,
        NPCKind::VoidRaider => 28,
        NPCKind::DrifterBoss => 45,
        // Automata
        NPCKind::MaintenanceDrone => 35,
        NPCKind::SecurityUnit => 40,
        NPCKind::CombatFrame => 240,
        // Abyssal Fauna
        NPCKind::MoonCrawler => 18,
        NPCKind::VoidSpitter => 30,
        NPCKind::AbyssalBrute => 70,
        // Station Personnel
        NPCKind::SalvageOperative => 20,
        NPCKind::GunForHire => 50,
        NPCKind::StationGuard => 30,
        NPCKind::ShockTrooper => 60,
        _ => panic!("base_xp called on player character kind {:?}", kind),
    }
}

pub(crate) fn default_name(kind: &NPCKind) -> &'static str {
    match kind {
        NPCKind::Zealot => "Zealot",
        NPCKind::Preacher => "Preacher",
        NPCKind::Purifier => "Purifier",
        NPCKind::Archon => "Archon",
        NPCKind::Scavenger => "Scavenger",
        NPCKind::VoidRaider => "Void Raider",
        NPCKind::DrifterBoss => "Drifter Boss",
        NPCKind::MaintenanceDrone => "Maintenance Drone",
        NPCKind::SecurityUnit => "Security Unit",
        NPCKind::CombatFrame => "Combat Frame",
        NPCKind::MoonCrawler => "Moon Crawler",
        NPCKind::VoidSpitter => "Void Spitter",
        NPCKind::AbyssalBrute => "Abyssal Brute",
        NPCKind::SalvageOperative => "Salvage Operative",
        NPCKind::GunForHire => "Gun-for-Hire",
        NPCKind::StationGuard => "Station Guard",
        NPCKind::ShockTrooper => "Shock Trooper",
        _ => panic!("default_name called on player character kind {:?}", kind),
    }
}

pub(crate) fn default_aggression(kind: &NPCKind) -> crate::character::Aggression {
    use crate::character::Aggression;
    match kind {
        // The Constancy — always aggressive, no exceptions
        NPCKind::Zealot => Aggression::Aggressive,
        NPCKind::Preacher => Aggression::Aggressive,
        NPCKind::Purifier => Aggression::Aggressive,
        NPCKind::Archon => Aggression::Aggressive,
        // Drifters — always hostile
        NPCKind::Scavenger => Aggression::Aggressive,
        NPCKind::VoidRaider => Aggression::Aggressive,
        NPCKind::DrifterBoss => Aggression::Aggressive,
        // Automata — Security hostile; Maintenance drones wander until provoked
        NPCKind::MaintenanceDrone => Aggression::Neutral,
        NPCKind::SecurityUnit => Aggression::Aggressive,
        NPCKind::CombatFrame => Aggression::Aggressive,
        // Abyssal Fauna — aggressive by nature
        NPCKind::MoonCrawler => Aggression::Aggressive,
        NPCKind::VoidSpitter => Aggression::Aggressive,
        NPCKind::AbyssalBrute => Aggression::Aggressive,
        // Station Personnel — personnel start friendly or neutral; hostile later
        NPCKind::SalvageOperative => Aggression::Friendly,
        NPCKind::GunForHire => Aggression::Neutral,
        NPCKind::StationGuard => Aggression::Friendly,
        NPCKind::ShockTrooper => Aggression::Aggressive,
        _ => panic!(
            "default_aggression called on player character kind {:?}",
            kind
        ),
    }
}
