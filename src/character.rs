use crate::npc::NPCKind;
use crate::stats::Stats;
use bevy::prelude::Component;

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
    pub kind: NPCKind,
    pub level: u32,
    pub stats: Stats,
    pub current_hp: i32,
    pub xp_reward: u32,
    pub aggression: Aggression,
}

impl Character {
    /// Create a player character at level 1 with class-based base stats.
    pub fn new_player(name: impl Into<String>, kind: NPCKind) -> Self {
        let stats = Stats::for_character(&kind);
        let current_hp = stats.max_hp;
        Self {
            name: name.into(),
            kind,
            level: 1,
            stats,
            current_hp,
            xp_reward: 0,
            aggression: Aggression::Friendly,
        }
    }

    /// Create an NPC (enemy or neutral) at the given level with scaled stats.
    pub fn new_npc(kind: NPCKind, level: u32) -> Self {
        use crate::npc::{base_xp, default_aggression, default_name, scaled_stats};
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
