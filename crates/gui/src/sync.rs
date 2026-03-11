use bevy::prelude::*;
use carbonthrone::{combat::Turn, game::GamePhase};

use super::resources::{GameSessionRes, LastKnownZone, PendingAbilityTarget, PendingPlayerChoices};
use super::state::AppState;

pub struct SyncPlugin;

impl Plugin for SyncPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (phase_sync_system, zone_change_detect_system).chain(),
        )
        .add_systems(
            Update,
            refresh_player_choices.run_if(in_state(AppState::Battle)),
        );
    }
}

/// Read the game session phase and drive `NextState<AppState>` accordingly.
pub fn phase_sync_system(
    session: Res<GameSessionRes>,
    current_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if !session.is_changed() {
        return;
    }
    let desired = match &session.0.phase {
        GamePhase::Exploration(e) if e.in_dialog => AppState::Dialog,
        GamePhase::Exploration(_) => AppState::Exploration,
        GamePhase::Battle(_) => AppState::Battle,
        GamePhase::Transitioning => return,
    };
    if *current_state.get() != desired {
        next_state.set(desired);
    }
}

/// Detect when the zone kind changes and respawn all visuals.
pub fn zone_change_detect_system(
    session: Res<GameSessionRes>,
    mut last_zone: ResMut<LastKnownZone>,
    current_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    // current_zone_kind() is a method on GameSessionRes
    let new_zone = session.current_zone_kind();
    if last_zone.0 == new_zone {
        return;
    }
    last_zone.0 = new_zone;

    // Force visual respawn for same-state zone transitions (e.g. hallway → hallway).
    let desired_state = match &session.0.phase {
        GamePhase::Exploration(e) if e.in_dialog => AppState::Dialog,
        GamePhase::Exploration(_) => AppState::Exploration,
        GamePhase::Battle(_) => AppState::Battle,
        GamePhase::Transitioning => return,
    };
    if *current_state.get() == desired_state {
        next_state.set(desired_state);
    }
}

/// Refresh PendingPlayerChoices when it's the player's turn in battle.
fn refresh_player_choices(
    mut session: ResMut<GameSessionRes>,
    mut choices: ResMut<PendingPlayerChoices>,
    mut targeting: ResMut<PendingAbilityTarget>,
) {
    let s = &mut session.0;
    let is_player_turn = s
        .battle
        .as_ref()
        .map(|b| b.turn == Turn::Player)
        .unwrap_or(false);
    if !is_player_turn || s.battle_over() {
        choices.choices.clear();
        targeting.0 = None;
        return;
    }
    if choices.needs_refresh || choices.choices.is_empty() {
        choices.choices = s
            .battle
            .as_mut()
            .map(|b| b.player_choices(&mut s.world))
            .unwrap_or_default();
        choices.needs_refresh = false;
        targeting.0 = None; // cancel targeting when choices are refreshed (new actor)
    }
}
