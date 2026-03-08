use bevy::{prelude::*, window::PrimaryWindow};
use carbonthrone::game::GamePhase;

use super::{
    camera::IsometricCamera,
    grid::world_to_grid,
    resources::{ExplorationRng, GameSessionRes, PendingPlayerChoices, SelectedChoiceIndex},
    state::AppState,
};

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                right_click_navigate.run_if(in_state(AppState::Exploration)),
                left_click_npc.run_if(in_state(AppState::Exploration)),
                advance_dialog_click.run_if(in_state(AppState::Dialog)),
                apply_player_choice.run_if(in_state(AppState::Battle)),
                auto_advance_enemy_turn.run_if(in_state(AppState::Battle)),
            ),
        );
    }
}

// ── Raycasting ────────────────────────────────────────────────────────────────

/// Cast a ray from the camera through the cursor and intersect with the Y=0 plane.
/// Returns the grid cell hit, or `None` if the cursor is off-screen or ray is parallel.
fn cursor_to_grid(
    camera: &Camera,
    cam_transform: &GlobalTransform,
    cursor_pos: Vec2,
) -> Option<(i32, i32)> {
    let ray = camera.viewport_to_world(cam_transform, cursor_pos).ok()?;
    // Intersect with Y = 0 plane: t = -origin.y / direction.y
    if ray.direction.y.abs() < 1e-6 {
        return None; // Ray is parallel to the ground plane.
    }
    let t = -ray.origin.y / ray.direction.y;
    if t < 0.0 {
        return None; // Ground plane is behind the camera.
    }
    let hit = ray.origin + ray.direction * t;
    Some(world_to_grid(hit))
}

fn get_cursor_grid(
    windows: &Query<&Window, With<PrimaryWindow>>,
    camera_q: &Query<(&Camera, &GlobalTransform), With<IsometricCamera>>,
) -> Option<(i32, i32)> {
    let window = windows.single().ok()?;
    let cursor = window.cursor_position()?;
    let (cam, cam_transform) = camera_q.single().ok()?;
    cursor_to_grid(cam, cam_transform, cursor)
}

// ── Exploration: right-click navigate ────────────────────────────────────────

fn right_click_navigate(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<IsometricCamera>>,
    mut session: ResMut<GameSessionRes>,
    mut rng: ResMut<ExplorationRng>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Some((gx, gy)) = get_cursor_grid(&windows, &camera_q) else {
        return;
    };

    // Read player position.
    let GamePhase::Exploration(state) = &session.0.phase else {
        return;
    };
    let world = &session.0.world;
    let player_pos = world
        .get::<carbonthrone::position::Position>(state.player_entity)
        .copied()
        .unwrap_or(carbonthrone::position::Position::new(0, 0));

    let dx = (gx - player_pos.x).signum();
    let dy = (gy - player_pos.y).signum();

    if dx == 0 && dy == 0 {
        return;
    }

    session.0.move_player(dx, dy, &mut rng.0);
}

// ── Exploration: left-click NPC to start dialog ───────────────────────────────

fn left_click_npc(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<IsometricCamera>>,
    mut session: ResMut<GameSessionRes>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some((gx, gy)) = get_cursor_grid(&windows, &camera_q) else {
        return;
    };

    let GamePhase::Exploration(state) = &session.0.phase else {
        return;
    };
    let world = &session.0.world;

    // Check if player is adjacent to an NPC at the clicked position.
    let player_pos = world
        .get::<carbonthrone::position::Position>(state.player_entity)
        .copied()
        .unwrap_or(carbonthrone::position::Position::new(0, 0));

    let should_interact = state.npcs.iter().any(|n| n.pos == (gx, gy))
        && (gx - player_pos.x).abs() + (gy - player_pos.y).abs() <= 1;

    if should_interact {
        if let GamePhase::Exploration(e) = &mut session.0.phase {
            e.fire_trigger(carbonthrone::dialog::Trigger::OnInteract);
        }
    }
}

// ── Dialog: advance / select choice ──────────────────────────────────────────

/// Left-click in dialog state advances to the next line.
/// Choice selection is handled by UI buttons in `ui/dialog.rs`.
fn advance_dialog_click(
    mouse: Res<ButtonInput<MouseButton>>,
    mut session: ResMut<GameSessionRes>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        let at_choice = match &session.0.phase {
            GamePhase::Exploration(e) => e.at_choice_screen(),
            _ => false,
        };
        if !at_choice {
            session.0.advance_dialog();
        }
    }
}

// ── Combat: execute player choice ────────────────────────────────────────────

fn apply_player_choice(
    mut session: ResMut<GameSessionRes>,
    mut selected: ResMut<SelectedChoiceIndex>,
    mut choices_res: ResMut<PendingPlayerChoices>,
) {
    let Some(idx) = selected.0.take() else {
        return;
    };
    let session = &mut session.0;
    let choice = choices_res.choices.get(idx).cloned();
    let Some(choice) = choice else { return };

    if let Some(battle) = session.battle.as_mut() {
        let result = battle.step_player_action(&mut session.world, &choice);
        if result.outcome.is_some() {
            // Record outcome in last_event-like manner.
        }
        // Update last_event to reflect this step.
        session.last_event = Some(carbonthrone::combat::TurnEvent {
            actor: Some(result.actor),
            turn: carbonthrone::combat::Turn::Player,
            actions: result.action.map(|a| vec![a]).unwrap_or_default(),
            outcome: result.outcome,
        });
        choices_res.needs_refresh = true;
    }
}

// ── Combat: auto-advance enemy turn ──────────────────────────────────────────

fn auto_advance_enemy_turn(
    mut session: ResMut<GameSessionRes>,
    mut choices_res: ResMut<PendingPlayerChoices>,
    time: Res<Time>,
    mut enemy_turn_timer: Local<f32>,
) {
    let session = &mut session.0;
    let is_enemy_turn = session
        .battle
        .as_ref()
        .map(|b| b.turn == carbonthrone::combat::Turn::Enemy)
        .unwrap_or(false);
    if !is_enemy_turn || session.battle_over() {
        *enemy_turn_timer = 0.0;
        return;
    }

    // Small delay between enemy actions for readability.
    *enemy_turn_timer += time.delta_secs();
    if *enemy_turn_timer < 0.3 {
        return;
    }
    *enemy_turn_timer = 0.0;

    session.step_battle();
    choices_res.needs_refresh = true;
}
