use bevy::prelude::*;
use crate::character::CharacterClass;
use crate::health::Health;
use crate::stats::Stats;

/// Tracks XP and level for player characters.
#[derive(Debug, Clone, Component)]
pub struct Experience {
    pub level: u32,
    pub current: u32,
    /// Level-ups gained since the last `level_up_system` run.
    pub(crate) pending_levels: u32,
}

impl Experience {
    pub fn new() -> Self {
        Self { level: 1, current: 0, pending_levels: 0 }
    }

    pub fn xp_to_next(&self) -> u32 {
        100 * self.level
    }

    /// Add XP. Pending level-ups are applied by `level_up_system`.
    pub fn add(&mut self, amount: u32) {
        self.current += amount;
        while self.current >= self.xp_to_next() {
            self.current -= self.xp_to_next();
            self.level += 1;
            self.pending_levels += 1;
        }
    }
}

impl Default for Experience {
    fn default() -> Self {
        Self::new()
    }
}

/// Bevy system: applies stat growth for pending level-ups and syncs Health.
/// Entities must have (Experience, Stats, CharacterClass, Health).
pub fn level_up_system(
    mut query: Query<(&mut Experience, &mut Stats, &CharacterClass, &mut Health)>,
) {
    for (mut xp, mut stats, class, mut health) in &mut query {
        while xp.pending_levels > 0 {
            stats.level_up(class);
            health.max = stats.max_hp;
            health.current = stats.max_hp; // full restore on level-up
            xp.pending_levels -= 1;
        }
    }
}
