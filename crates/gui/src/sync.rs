use bevy::prelude::*;
use carbonthrone::{combat::Turn, game::GamePhase, player_input::PlayerActionChoice};

use super::camera::IsometricCamera;
use super::grid::map_center;
use super::resources::{GameSessionRes, LastKnownZone, PendingPlayerChoices, SelectedChoiceIndex};
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
            (refresh_player_choices, auto_pass_zero_ap)
                .chain()
                .run_if(in_state(AppState::Battle)),
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
    mut camera_q: Query<&mut Transform, With<IsometricCamera>>,
    current_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    // current_zone_kind() is a method on GameSessionRes
    let new_zone = session.current_zone_kind();
    if last_zone.0 == new_zone {
        return;
    }
    last_zone.0 = new_zone;

    // Reposition camera to new zone's center.
    if let Some((cols, rows)) = session.exploration_zone_size() {
        let center = map_center(cols, rows);
        let offset = Vec3::new(18.0, 18.0, 18.0);
        let cam_pos = center + offset;
        if let Ok(mut transform) = camera_q.single_mut() {
            *transform = Transform::from_translation(cam_pos).looking_at(center, Vec3::Y);
        }
    }

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

/// Auto-pass when the active player actor has no actions left (choices = [Pass] only).
fn auto_pass_zero_ap(
    choices: Res<PendingPlayerChoices>,
    mut selected: ResMut<SelectedChoiceIndex>,
) {
    if selected.0.is_some() {
        return;
    }
    let only_pass = choices.choices.len() == 1
        && matches!(choices.choices[0], PlayerActionChoice::Pass);
    if only_pass {
        selected.0 = Some(0);
    }
}

/// Refresh PendingPlayerChoices when it's the player's turn in battle.
fn refresh_player_choices(
    mut session: ResMut<GameSessionRes>,
    mut choices: ResMut<PendingPlayerChoices>,
) {
    let s = &mut session.0;
    let is_player_turn = s
        .battle
        .as_ref()
        .map(|b| b.turn == Turn::Player)
        .unwrap_or(false);
    if !is_player_turn || s.battle_over() {
        choices.choices.clear();
        return;
    }
    if choices.needs_refresh || choices.choices.is_empty() {
        choices.choices = s
            .battle
            .as_mut()
            .map(|b| b.player_choices(&mut s.world))
            .unwrap_or_default();
        choices.needs_refresh = false;
    }
}
