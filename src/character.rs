use crate::enemy::EnemyKind;
use crate::stats::Stats;
use bevy::prelude::Component;

/// The named player characters, each with distinct stats and abilities.
#[derive(Debug, Clone, PartialEq, Component)]
pub enum PlayerCharacter {
    /// The player character — Temporal Mage; high magic, fragile, fast.
    Researcher,
    /// Dr. Sable Orin — Cleric; support healer, high defense, low offense.
    Orin,
    /// Recruiter Doss — Warrior; heavy armor, high HP, strong melee.
    Doss,
    /// Unit Kaleo — Ranger; precision attacks, system hacking, very fast.
    Kaleo,
}

/// Discriminator for any entity in the world — player or NPC.
#[derive(Debug, Clone, PartialEq)]
pub enum CharacterKind {
    Player(PlayerCharacter),
    NPC(EnemyKind),
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
    /// Create a player character at level 1 with class-based base stats.
    pub fn new_player(name: impl Into<String>, pc: PlayerCharacter) -> Self {
        let stats = Stats::for_character(&pc);
        let current_hp = stats.max_hp;
        Self {
            name: name.into(),
            kind: CharacterKind::Player(pc),
            level: 1,
            stats,
            current_hp,
            xp_reward: 0,
            aggression: Aggression::Friendly,
        }
    }

    /// Create an NPC (enemy or neutral) at the given level with scaled stats.
    pub fn new_npc(kind: EnemyKind, level: u32) -> Self {
        use crate::enemy::{base_xp, default_aggression, default_name, scaled_stats};
        let stats = scaled_stats(&kind, level);
        let current_hp = stats.max_hp;
        let xp_reward = base_xp(&kind) * level;
        let name = default_name(&kind).to_string();
        let aggression = default_aggression(&kind);
        Self {
            name,
            kind: CharacterKind::NPC(kind),
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
