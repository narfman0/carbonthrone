use crate::zone::ZoneKind;

/// Tracks an in-progress journey between two named zones.
///
/// While traveling, the player passes through one or more anonymous
/// [`ZoneKind::Hallway`] zones. Each time they exit a hallway, the game
/// rolls against [`arrival_chance`] to decide whether they reach the
/// destination or enter another hallway. Higher loop numbers reduce the
/// arrival chance, reflecting temporal disorientation.
#[derive(Debug, Clone)]
pub struct TravelState {
    /// The named zone the player departed from.
    pub origin: ZoneKind,
    /// The named zone the player is trying to reach.
    pub destination: ZoneKind,
    /// How many hallways have been traversed so far this journey.
    pub hallways_traversed: u32,
}

impl TravelState {
    pub fn new(origin: ZoneKind, destination: ZoneKind) -> Self {
        Self {
            origin,
            destination,
            hallways_traversed: 0,
        }
    }
}

/// Probability of reaching the destination when exiting a hallway.
///
/// | Loop | Chance |
/// |------|--------|
/// |  1   | 80%    |
/// |  2   | 65%    |
/// |  3   | 50%    |
/// |  4   | 35%    |
/// |  5+  | 20%    |
///
/// Minimum clamped to 0.10.
pub fn arrival_chance(loop_number: u32) -> f64 {
    (0.95 - loop_number as f64 * 0.15).max(0.10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrival_chance_by_loop() {
        assert!((arrival_chance(1) - 0.80).abs() < 1e-9);
        assert!((arrival_chance(2) - 0.65).abs() < 1e-9);
        assert!((arrival_chance(3) - 0.50).abs() < 1e-9);
        assert!((arrival_chance(4) - 0.35).abs() < 1e-9);
        assert!((arrival_chance(5) - 0.20).abs() < 1e-9);
        // Clamped floor at 0.10
        assert!((arrival_chance(10) - 0.10).abs() < 1e-9);
    }
}
