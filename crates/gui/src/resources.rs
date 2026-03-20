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

/// State for the developer terminal overlay (toggled with backtick).
#[derive(Resource, Default)]
pub struct TerminalState {
    pub open: bool,
    pub input: String,
    pub last_output: String,
    pub last_command: String,
}

/// Whether the pause menu overlay is visible.
#[derive(Resource, Default)]
pub struct PauseMenuOpen(pub bool);

/// Whether the ECS world inspector panel is visible (dev builds only).
#[cfg(feature = "dev")]
#[derive(Resource, Default)]
pub struct InspectorOpen(pub bool);

/// Tracks which save slot the current session was loaded from / last saved to.
#[derive(Resource, Default)]
pub struct ActiveSaveSlot(pub Option<u8>);

/// Whether the minimap overlay is visible.
#[derive(Resource, Default)]
pub struct MinimapOpen(pub bool);

/// Transient camera-shake effect. Call `trigger()` to start a shake.
#[derive(Resource, Default)]
pub struct ScreenShake {
    /// Remaining shake time in seconds.
    pub timer: f32,
    /// Peak offset magnitude (world units).
    pub magnitude: f32,
    /// Offset applied to the camera last frame (subtracted before re-applying).
    pub current_offset: Vec3,
}

impl ScreenShake {
    pub fn trigger(&mut self, magnitude: f32) {
        self.timer = 0.3;
        self.magnitude = magnitude;
    }
}

/// Maps hotkey slots to ability choice indices for the current player turn.
/// Slot 0 = key "1", slot 1 = key "2", …, slot 8 = key "9".
/// `pass_idx` holds the choice index for the Pass action (triggered with "P").
#[derive(Resource, Default)]
pub struct AbilityHotkeyMap {
    /// Indexed by hotkey slot (0–8). `None` means the slot is unused.
    pub abilities: Vec<Option<usize>>,
    /// Choice index for the Pass action, if available.
    pub pass_idx: Option<usize>,
}

/// Queued movement path used in both exploration and combat.
/// In combat, `actor` identifies who is moving and `total_ap_cost` is charged on completion.
/// In exploration, only `path` is used.
#[derive(Resource, Default)]
pub struct PendingPath {
    pub path: Vec<(i32, i32)>,
    /// The entity being moved. `None` during exploration (player is implicit).
    pub actor: Option<bevy::ecs::entity::Entity>,
    pub total_ap_cost: i32,
}
