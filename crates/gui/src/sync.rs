use bevy::prelude::*;
use carbonthrone::{combat::Turn, game::GamePhase, zone::ZoneKind};

use super::camera::SunLight;
use super::resources::{GameSessionRes, LastKnownZone, PendingAbilityTarget, PendingPlayerChoices};
use super::state::AppState;

pub struct SyncPlugin;

impl Plugin for SyncPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (phase_sync_system, zone_change_detect_system)
                .chain()
                .run_if(not(in_state(AppState::MainMenu)).and(not(in_state(AppState::Intro)))),
        )
        .add_systems(
            Update,
            (
                refresh_player_choices.run_if(in_state(AppState::Battle)),
                zone_lighting_system,
            ),
        );
    }
}

/// Read the game session phase and drive `NextState<AppState>` accordingly.
pub fn phase_sync_system(
    session: Res<GameSessionRes>,
    current_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let desired = match &session.0.phase {
        GamePhase::Exploration(e) if e.in_dialog => AppState::Dialog,
        GamePhase::Exploration(_) => AppState::Exploration,
        GamePhase::Battle(_) => AppState::Battle,
        GamePhase::Transitioning | GamePhase::Ended(_) => return,
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
        GamePhase::Transitioning | GamePhase::Ended(_) => return,
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
        targeting.0 = None;
    }
}

/// Tint the directional sun light based on the current zone's atmosphere.
fn zone_lighting_system(
    session: Res<GameSessionRes>,
    mut sun_q: Query<&mut DirectionalLight, With<SunLight>>,
) {
    if !session.is_changed() {
        return;
    }
    let Some(zone_kind) = session.current_zone_kind() else {
        return;
    };
    let Ok(mut sun) = sun_q.single_mut() else {
        return;
    };

    let (color, illuminance) = zone_tint(zone_kind);
    sun.color = color;
    sun.illuminance = illuminance;
}

fn zone_tint(zone: ZoneKind) -> (Color, f32) {
    match zone {
        // Exterior zones — cold blue moonlight.
        ZoneKind::StationExterior | ZoneKind::RelayArray | ZoneKind::ExcavationSite => {
            (Color::srgb(0.65, 0.75, 1.00), 8000.0)
        }
        // Engineering/systems — warm orange heat glow.
        ZoneKind::SystemsCore => (Color::srgb(1.00, 0.75, 0.45), 11000.0),
        // Medical — sterile white-blue.
        ZoneKind::MedicalBay => (Color::srgb(0.85, 0.95, 1.00), 12000.0),
        // Military — harsh white.
        ZoneKind::MilitaryAnnex => (Color::srgb(1.00, 0.95, 0.85), 13000.0),
        // Docking — yellowish industrial.
        ZoneKind::DockingBay => (Color::srgb(1.00, 0.90, 0.65), 10000.0),
        // Hallway — dim neutral.
        ZoneKind::Hallway => (Color::srgb(0.80, 0.80, 0.85), 7000.0),
        // Default interior — neutral daylight.
        _ => (Color::WHITE, 10000.0),
    }
}
