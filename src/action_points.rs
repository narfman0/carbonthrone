use bevy::prelude::*;

/// Action points available to a combatant each turn.
#[derive(Debug, Clone, Component)]
pub struct ActionPoints {
    pub current: i32,
    pub max: i32,
}

impl ActionPoints {
    pub fn new(max: i32) -> Self {
        Self { current: max, max }
    }

    /// Spends `cost` AP. Returns true on success; no change on failure.
    pub fn spend(&mut self, cost: i32) -> bool {
        if self.current >= cost {
            self.current -= cost;
            true
        } else {
            false
        }
    }

    /// Restores AP to max. Schedule at the start of each turn.
    pub fn refresh(&mut self) {
        self.current = self.max;
    }
}

/// Bevy system: restores all ActionPoints to max. Run at the start of each turn.
pub fn refresh_ap_system(mut query: Query<&mut ActionPoints>) {
    for mut ap in &mut query {
        ap.refresh();
    }
}
