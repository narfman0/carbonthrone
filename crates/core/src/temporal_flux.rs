use bevy::prelude::Resource;

pub const HIGH_FLUX_THRESHOLD: u32 = 75;
pub const MAX_FLUX: u32 = 100;

/// Zone-level temporal instability counter.
///
/// Increases as temporal abilities are used; at maximum triggers a Temporal Collapse
/// that damages and disrupts all combatants. Affects hit accuracy above the threshold.
#[derive(Resource, Default)]
pub struct TemporalFlux {
    pub flux: u32,
}

impl TemporalFlux {
    /// Add flux, clamped to [`MAX_FLUX`].
    pub fn add(&mut self, amount: u32) {
        self.flux = (self.flux + amount).min(MAX_FLUX);
    }

    /// Reset flux to zero (called after a Temporal Collapse).
    pub fn reset(&mut self) {
        self.flux = 0;
    }

    /// Returns `true` when flux has reached [`MAX_FLUX`] — triggers a Temporal Collapse.
    pub fn is_overloaded(&self) -> bool {
        self.flux >= MAX_FLUX
    }

    /// Hit penalty applied when flux exceeds [`HIGH_FLUX_THRESHOLD`].
    /// Returns 0.0 at or below threshold; linearly ramps to 0.20 at [`MAX_FLUX`].
    pub fn hit_penalty(&self) -> f32 {
        if self.flux <= HIGH_FLUX_THRESHOLD {
            0.0
        } else {
            (self.flux - HIGH_FLUX_THRESHOLD) as f32 / 25.0 * 0.20
        }
    }
}
