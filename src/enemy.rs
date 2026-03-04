use bevy::prelude::Component;
use crate::combatant::Combatant;
use crate::stats::Stats;

#[derive(Debug, Clone, PartialEq, Component)]
pub enum EnemyKind {
    Goblin,   // fast, low hp, low damage
    Skeleton, // balanced undead
    Orc,      // high attack, medium hp
    Troll,    // tanky, slow, high hp
    DarkMage, // high magic, low physical defense
    Dragon,   // boss-tier: all stats elevated
}

#[derive(Debug, Clone, Component)]
pub struct Enemy {
    pub name: String,
    pub kind: EnemyKind,
    pub level: u32,
    pub stats: Stats,
    pub current_hp: i32,
    pub xp_reward: u32,
}

impl Enemy {
    pub fn new(kind: EnemyKind, level: u32) -> Self {
        let stats = scaled_stats(&kind, level);
        let current_hp = stats.max_hp;
        let xp_reward = base_xp(&kind) * level;
        let name = default_name(&kind).to_string();
        Self { name, kind, level, stats, current_hp, xp_reward }
    }

    pub fn is_alive(&self) -> bool {
        self.current_hp > 0
    }

    pub fn take_damage(&mut self, amount: i32) {
        self.current_hp = (self.current_hp - amount).max(0);
    }
}

impl Combatant for Enemy {
    fn name(&self) -> &str         { &self.name }
    fn current_hp(&self) -> i32    { self.current_hp }
    fn max_hp(&self) -> i32        { self.stats.max_hp }
    fn attack(&self) -> i32        { self.stats.attack }
    fn defense(&self) -> i32       { self.stats.defense }
    fn speed(&self) -> i32         { self.stats.speed }
    fn magic(&self) -> i32         { self.stats.magic }
    fn take_damage(&mut self, amount: i32) { Enemy::take_damage(self, amount); }
    fn is_alive(&self) -> bool     { Enemy::is_alive(self) }
}

/// Base stats at level 1.
fn base_stats(kind: &EnemyKind) -> Stats {
    match kind {
        EnemyKind::Goblin   => Stats { max_hp: 40,  attack: 8,  defense: 3,  speed: 16, magic: 0  },
        EnemyKind::Skeleton => Stats { max_hp: 60,  attack: 10, defense: 6,  speed: 10, magic: 0  },
        EnemyKind::Orc      => Stats { max_hp: 90,  attack: 16, defense: 8,  speed: 7,  magic: 0  },
        EnemyKind::Troll    => Stats { max_hp: 150, attack: 12, defense: 14, speed: 4,  magic: 0  },
        EnemyKind::DarkMage => Stats { max_hp: 50,  attack: 4,  defense: 3,  speed: 11, magic: 18 },
        EnemyKind::Dragon   => Stats { max_hp: 300, attack: 25, defense: 20, speed: 12, magic: 15 },
    }
}

/// Per-level stat growth.
fn growth(kind: &EnemyKind) -> (i32, i32, i32, i32, i32) {
    // (hp, atk, def, spd, mag)
    match kind {
        EnemyKind::Goblin   => (5,  1, 0, 1, 0),
        EnemyKind::Skeleton => (8,  2, 1, 0, 0),
        EnemyKind::Orc      => (12, 3, 1, 0, 0),
        EnemyKind::Troll    => (20, 2, 2, 0, 0),
        EnemyKind::DarkMage => (6,  0, 0, 1, 3),
        EnemyKind::Dragon   => (40, 5, 4, 1, 3),
    }
}

fn scaled_stats(kind: &EnemyKind, level: u32) -> Stats {
    let mut s = base_stats(kind);
    let (hp, atk, def, spd, mag) = growth(kind);
    let extra = (level - 1) as i32;
    s.max_hp  += hp  * extra;
    s.attack  += atk * extra;
    s.defense += def * extra;
    s.speed   += spd * extra;
    s.magic   += mag * extra;
    s
}

fn base_xp(kind: &EnemyKind) -> u32 {
    match kind {
        EnemyKind::Goblin   => 20,
        EnemyKind::Skeleton => 30,
        EnemyKind::Orc      => 45,
        EnemyKind::Troll    => 70,
        EnemyKind::DarkMage => 55,
        EnemyKind::Dragon   => 250,
    }
}

fn default_name(kind: &EnemyKind) -> &'static str {
    match kind {
        EnemyKind::Goblin   => "Goblin",
        EnemyKind::Skeleton => "Skeleton",
        EnemyKind::Orc      => "Orc",
        EnemyKind::Troll    => "Troll",
        EnemyKind::DarkMage => "Dark Mage",
        EnemyKind::Dragon   => "Dragon",
    }
}
