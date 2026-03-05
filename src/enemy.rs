use crate::stats::Stats;
use bevy::prelude::Component;

#[derive(Debug, Clone, PartialEq, Component)]
pub enum EnemyKind {
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

/// Current disposition of this enemy toward the player party.
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

#[derive(Debug, Clone, Component)]
pub struct Enemy {
    pub name: String,
    pub kind: EnemyKind,
    pub level: u32,
    pub stats: Stats,
    pub current_hp: i32,
    pub xp_reward: u32,
    pub aggression: Aggression,
}

impl Enemy {
    pub fn new(kind: EnemyKind, level: u32) -> Self {
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

/// Base stats at level 1.
fn base_stats(kind: &EnemyKind) -> Stats {
    match kind {
        // Drifters
        EnemyKind::Scavenger => Stats {
            max_hp: 40,
            attack: 9,
            defense: 3,
            speed: 15,
        },
        EnemyKind::VoidRaider => Stats {
            max_hp: 55,
            attack: 12,
            defense: 5,
            speed: 11,
        },
        EnemyKind::DrifterBoss => Stats {
            max_hp: 95,
            attack: 17,
            defense: 8,
            speed: 7,
        },
        // Automata
        EnemyKind::MaintenanceDrone => Stats {
            max_hp: 65,
            attack: 8,
            defense: 10,
            speed: 7,
        },
        EnemyKind::SecurityUnit => Stats {
            max_hp: 80,
            attack: 13,
            defense: 11,
            speed: 9,
        },
        EnemyKind::CombatFrame => Stats {
            max_hp: 280,
            attack: 24,
            defense: 20,
            speed: 10,
        },
        // Abyssal Fauna
        EnemyKind::MoonCrawler => Stats {
            max_hp: 35,
            attack: 11,
            defense: 2,
            speed: 18,
        },
        EnemyKind::VoidSpitter => Stats {
            max_hp: 55,
            attack: 14,
            defense: 4,
            speed: 10,
        },
        EnemyKind::AbyssalBrute => Stats {
            max_hp: 160,
            attack: 13,
            defense: 15,
            speed: 4,
        },
        // Station Personnel
        EnemyKind::SalvageOperative => Stats {
            max_hp: 55,
            attack: 9,
            defense: 6,
            speed: 10,
        },
        EnemyKind::GunForHire => Stats {
            max_hp: 75,
            attack: 15,
            defense: 9,
            speed: 10,
        },
        EnemyKind::StationGuard => Stats {
            max_hp: 65,
            attack: 12,
            defense: 10,
            speed: 10,
        },
        EnemyKind::ShockTrooper => Stats {
            max_hp: 100,
            attack: 19,
            defense: 13,
            speed: 8,
        },
    }
}

/// Per-level stat growth: (hp, atk, def, spd).
fn growth(kind: &EnemyKind) -> (i32, i32, i32, i32) {
    match kind {
        // Drifters
        EnemyKind::Scavenger => (5, 1, 0, 1),
        EnemyKind::VoidRaider => (7, 2, 1, 0),
        EnemyKind::DrifterBoss => (12, 3, 1, 0),
        // Automata
        EnemyKind::MaintenanceDrone => (9, 1, 2, 0),
        EnemyKind::SecurityUnit => (10, 2, 2, 0),
        EnemyKind::CombatFrame => (38, 5, 4, 1),
        // Abyssal Fauna
        EnemyKind::MoonCrawler => (4, 2, 0, 1),
        EnemyKind::VoidSpitter => (7, 2, 0, 0),
        EnemyKind::AbyssalBrute => (22, 2, 2, 0),
        // Station Personnel
        EnemyKind::SalvageOperative => (6, 1, 1, 0),
        EnemyKind::GunForHire => (10, 2, 1, 0),
        EnemyKind::StationGuard => (8, 2, 1, 0),
        EnemyKind::ShockTrooper => (13, 3, 2, 0),
    }
}

fn scaled_stats(kind: &EnemyKind, level: u32) -> Stats {
    let mut s = base_stats(kind);
    let (hp, atk, def, spd) = growth(kind);
    let extra = (level - 1) as i32;
    s.max_hp += hp * extra;
    s.attack += atk * extra;
    s.defense += def * extra;
    s.speed += spd * extra;
    s
}

fn base_xp(kind: &EnemyKind) -> u32 {
    match kind {
        // Drifters
        EnemyKind::Scavenger => 20,
        EnemyKind::VoidRaider => 28,
        EnemyKind::DrifterBoss => 45,
        // Automata
        EnemyKind::MaintenanceDrone => 35,
        EnemyKind::SecurityUnit => 40,
        EnemyKind::CombatFrame => 240,
        // Abyssal Fauna
        EnemyKind::MoonCrawler => 18,
        EnemyKind::VoidSpitter => 30,
        EnemyKind::AbyssalBrute => 70,
        // Station Personnel
        EnemyKind::SalvageOperative => 20,
        EnemyKind::GunForHire => 50,
        EnemyKind::StationGuard => 30,
        EnemyKind::ShockTrooper => 60,
    }
}

fn default_name(kind: &EnemyKind) -> &'static str {
    match kind {
        EnemyKind::Scavenger => "Scavenger",
        EnemyKind::VoidRaider => "Void Raider",
        EnemyKind::DrifterBoss => "Drifter Boss",
        EnemyKind::MaintenanceDrone => "Maintenance Drone",
        EnemyKind::SecurityUnit => "Security Unit",
        EnemyKind::CombatFrame => "Combat Frame",
        EnemyKind::MoonCrawler => "Moon Crawler",
        EnemyKind::VoidSpitter => "Void Spitter",
        EnemyKind::AbyssalBrute => "Abyssal Brute",
        EnemyKind::SalvageOperative => "Salvage Operative",
        EnemyKind::GunForHire => "Gun-for-Hire",
        EnemyKind::StationGuard => "Station Guard",
        EnemyKind::ShockTrooper => "Shock Trooper",
    }
}

fn default_aggression(kind: &EnemyKind) -> Aggression {
    match kind {
        // Drifters — always hostile
        EnemyKind::Scavenger => Aggression::Aggressive,
        EnemyKind::VoidRaider => Aggression::Aggressive,
        EnemyKind::DrifterBoss => Aggression::Aggressive,
        // Automata — Security hostile; Maintenance drones wander until provoked
        EnemyKind::MaintenanceDrone => Aggression::Neutral,
        EnemyKind::SecurityUnit => Aggression::Aggressive,
        EnemyKind::CombatFrame => Aggression::Aggressive,
        // Abyssal Fauna — aggressive by nature
        EnemyKind::MoonCrawler => Aggression::Aggressive,
        EnemyKind::VoidSpitter => Aggression::Aggressive,
        EnemyKind::AbyssalBrute => Aggression::Aggressive,
        // Station Personnel — personnel start friendly or neutral; hostile later
        EnemyKind::SalvageOperative => Aggression::Friendly,
        EnemyKind::GunForHire => Aggression::Neutral,
        EnemyKind::StationGuard => Aggression::Friendly,
        EnemyKind::ShockTrooper => Aggression::Aggressive,
    }
}
