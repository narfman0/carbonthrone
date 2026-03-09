use bevy::prelude::*;
use carbonthrone::{
    game::{GamePhase, GameSession},
    terrain::LevelMap,
    zone::ZoneKind,
};
use rand::SeedableRng;
use rand::rngs::StdRng;

/// Wraps `GameSession` so it can be stored as a Bevy `Resource`.
#[derive(Resource)]
pub struct GameSessionRes(pub GameSession);

impl GameSessionRes {
    /// Convenience: get the zone map reference if in Exploration phase.
    pub fn exploration_map(&self) -> Option<&LevelMap> {
        match &self.0.phase {
            GamePhase::Exploration(e) => Some(&e.zone.map),
            _ => None,
        }
    }

    /// Convenience: get zone dimensions if in Exploration phase.
    pub fn exploration_zone_size(&self) -> Option<(u32, u32)> {
        match &self.0.phase {
            GamePhase::Exploration(e) => Some((e.zone.cols, e.zone.rows)),
            _ => None,
        }
    }

    /// Returns the zone kind in either Exploration or Battle phase.
    pub fn current_zone_kind(&self) -> Option<ZoneKind> {
        match &self.0.phase {
            GamePhase::Exploration(e) => Some(e.zone.kind),
            GamePhase::Battle(e) => Some(e.zone.kind),
            _ => None,
        }
    }
}

/// Wraps a `StdRng` for use as a Bevy `Resource` during exploration input.
#[derive(Resource)]
pub struct ExplorationRng(pub StdRng);

impl Default for ExplorationRng {
    fn default() -> Self {
        Self(StdRng::from_entropy())
    }
}

/// Tracks the last zone kind seen, so we can detect zone changes and respawn visuals.
#[derive(Resource, Default)]
pub struct LastKnownZone(pub Option<ZoneKind>);

/// Stores the current pending player combat choices so UI can display them.
#[derive(Resource, Default)]
pub struct PendingPlayerChoices {
    pub choices: Vec<carbonthrone::player_input::PlayerActionChoice>,
    pub needs_refresh: bool,
}

/// Index of the player choice that was selected this frame (if any).
#[derive(Resource, Default)]
pub struct SelectedChoiceIndex(pub Option<usize>);

/// When `Some`, the player has clicked an ability and is now selecting a target on the map.
/// Stores the name of the ability awaiting a target.
#[derive(Resource, Default)]
pub struct PendingAbilityTarget(pub Option<&'static str>);
