use std::collections::HashSet;

use bevy::{prelude::*, window::PrimaryWindow};
use carbonthrone::{
    action_points::ActionPoints,
    combat::Turn,
    game::GamePhase,
    player_input::PlayerActionChoice,
    position::Position,
    save::save_game,
    stats::Stats,
    turn::{Action, bfs_move_path, move_ap_cost},
};

use super::{
    camera::IsometricCamera,
    character_visuals::CharacterMoveAnim,
    grid::world_to_grid,
    resources::{
        ExplorationRng, GameSessionRes, PendingAbilityTarget, PendingPath, PendingPlayerChoices,
        SelectedChoiceIndex,
    },
    state::AppState,
    ui::terminal::terminal_closed,
};

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                escape_to_main_menu
                    .run_if(
                        in_state(AppState::Exploration)
                            .or(in_state(AppState::Dialog))
                            .or(in_state(AppState::Battle)),
                    )
                    .run_if(terminal_closed),
                right_click_navigate
                    .run_if(in_state(AppState::Exploration))
                    .run_if(terminal_closed),
                advance_exploration_path
                    .run_if(in_state(AppState::Exploration))
                    .run_if(terminal_closed),
                left_click_npc
                    .run_if(in_state(AppState::Exploration))
                    .run_if(terminal_closed),
                advance_dialog_click
                    .run_if(in_state(AppState::Dialog))
                    .run_if(terminal_closed),
                apply_player_choice
                    .run_if(in_state(AppState::Battle))
                    .run_if(terminal_closed),
                left_click_ability_target
                    .run_if(in_state(AppState::Battle))
                    .run_if(terminal_closed),
                right_click_battle_move
                    .run_if(in_state(AppState::Battle))
                    .run_if(terminal_closed),
                advance_combat_path
                    .run_if(in_state(AppState::Battle))
                    .run_if(terminal_closed),
                auto_advance_enemy_turn
                    .run_if(in_state(AppState::Battle))
                    .run_if(terminal_closed),
                detect_game_ending,
                handle_ended_input.run_if(in_state(AppState::Ended)),
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
    session: Res<GameSessionRes>,
    mut path: ResMut<PendingPath>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Some((gx, gy)) = get_cursor_grid(&windows, &camera_q) else {
        return;
    };

    let GamePhase::Exploration(state) = &session.0.phase else {
        return;
    };
    let world = &session.0.world;
    let player_pos = world
        .get::<Position>(state.player_entity)
        .copied()
        .unwrap_or(Position::new(0, 0));

    if player_pos.x == gx && player_pos.y == gy {
        return;
    }

    // Compute BFS path through the exploration map.
    let npc_occupied: HashSet<(i32, i32)> = state.npcs.iter().map(|n| n.pos).collect();
    let bfs = state
        .zone
        .map
        .bfs_path((player_pos.x, player_pos.y), (gx, gy), &npc_occupied);
    if !bfs.is_empty() {
        path.path = bfs;
        path.actor = None;
        path.total_ap_cost = 0;
    }
}

/// Consumes one step of the pending exploration path per frame (when not animating).
fn advance_exploration_path(
    mut session: ResMut<GameSessionRes>,
    mut path: ResMut<PendingPath>,
    mut rng: ResMut<ExplorationRng>,
    anim_q: Query<(), With<CharacterMoveAnim>>,
) {
    if path.path.is_empty() {
        return;
    }
    // Wait until the player entity is no longer animating.
    if !anim_q.is_empty() {
        return;
    }
    // Get current player position.
    let (px, py) = {
        let GamePhase::Exploration(state) = &session.0.phase else {
            path.path.clear();
            return;
        };
        let world = &session.0.world;
        let pos = world
            .get::<Position>(state.player_entity)
            .copied()
            .unwrap_or(Position::new(0, 0));
        (pos.x, pos.y)
    };

    let next = path.path.remove(0);
    let dx = (next.0 - px).signum();
    let dy = (next.1 - py).signum();
    if dx != 0 || dy != 0 {
        session.0.move_player(dx, dy, &mut rng.0);
    }
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
fn advance_dialog_click(mouse: Res<ButtonInput<MouseButton>>, mut session: ResMut<GameSessionRes>) {
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

// ── Combat: left-click to select ability target ───────────────────────────────

fn left_click_ability_target(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<IsometricCamera>>,
    session: Res<GameSessionRes>,
    choices_res: Res<PendingPlayerChoices>,
    mut targeting: ResMut<PendingAbilityTarget>,
    mut selected: ResMut<SelectedChoiceIndex>,
) {
    // Cancel targeting on Escape.
    if targeting.0.is_some() && keyboard.just_pressed(KeyCode::Escape) {
        targeting.0 = None;
        return;
    }
    // Cancel targeting on right-click (right-click is also used for movement below,
    // so we only consume it here when targeting is active).
    if targeting.0.is_some() && mouse.just_pressed(MouseButton::Right) {
        targeting.0 = None;
        return;
    }

    let Some(ability_name) = targeting.0 else {
        return;
    };
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some((gx, gy)) = get_cursor_grid(&windows, &camera_q) else {
        return;
    };

    let world = &session.0.world;
    let found_idx = choices_res.choices.iter().enumerate().find_map(|(i, c)| {
        if let PlayerActionChoice::UseAbility {
            ability,
            target: Some(t),
            ..
        } = c
        {
            if ability.name == ability_name {
                let pos = world.get::<Position>(*t)?;
                if pos.x == gx && pos.y == gy {
                    return Some(i);
                }
            }
        }
        None
    });
    if let Some(idx) = found_idx {
        selected.0 = Some(idx);
        targeting.0 = None;
    }
}

// ── Combat: right-click to move ──────────────────────────────────────────────

fn right_click_battle_move(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<IsometricCamera>>,
    mut session: ResMut<GameSessionRes>,
    mut combat_path: ResMut<PendingPath>,
    targeting: Res<PendingAbilityTarget>,
) {
    // Don't process movement right-click when in targeting mode.
    if targeting.0.is_some() {
        return;
    }
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Some((gx, gy)) = get_cursor_grid(&windows, &camera_q) else {
        return;
    };

    let s = &mut session.0;
    let Some(battle) = s.battle.as_ref() else {
        return;
    };
    if battle.turn != Turn::Player {
        return;
    }
    let Some(actor) = battle.current_actor() else {
        return;
    };

    let Some(actor_pos) = s.world.get::<Position>(actor).copied() else {
        return;
    };
    if actor_pos.x == gx && actor_pos.y == gy {
        return;
    }

    let path = bfs_move_path(&mut s.world, actor, (gx, gy));
    if path.is_empty() {
        return;
    }

    let speed = s.world.get::<Stats>(actor).map(|s| s.speed).unwrap_or(8);
    let cost = move_ap_cost(path.len() as i32, speed);
    let current_ap = s
        .world
        .get::<ActionPoints>(actor)
        .map(|a| a.current)
        .unwrap_or(0);
    if current_ap < cost {
        return;
    }
    combat_path.path = path;
    combat_path.actor = Some(actor);
    combat_path.total_ap_cost = cost;
}

/// Executes one step of the pending combat movement path per frame when not animating.
/// Works for both player and enemy moves. AP is deducted all at once when the full path completes.
fn advance_combat_path(
    mut session: ResMut<GameSessionRes>,
    mut combat_path: ResMut<PendingPath>,
    mut choices_res: ResMut<PendingPlayerChoices>,
    anim_q: Query<(), With<CharacterMoveAnim>>,
) {
    if combat_path.path.is_empty() {
        return;
    }
    if session.0.battle_over() {
        combat_path.path.clear();
        combat_path.actor = None;
        return;
    }
    if !anim_q.is_empty() {
        return;
    }

    let Some(actor) = combat_path.actor else {
        combat_path.path.clear();
        return;
    };

    let next = combat_path.path.remove(0);
    session.0.step_combat_tile(actor, next);
    choices_res.needs_refresh = true;

    if combat_path.path.is_empty() {
        let ap_cost = combat_path.total_ap_cost;
        let final_pos = Position::new(next.0, next.1);
        session.0.finalize_character_move(actor, ap_cost, final_pos);
        combat_path.actor = None;
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
    mut combat_path: ResMut<PendingPath>,
    time: Res<Time>,
    mut enemy_turn_timer: Local<f32>,
    anim_q: Query<(), With<CharacterMoveAnim>>,
) {
    // Don't start a new action while a path is animating.
    if !combat_path.path.is_empty() {
        return;
    }

    let is_enemy_turn = session
        .0
        .battle
        .as_ref()
        .map(|b| b.turn == carbonthrone::combat::Turn::Enemy)
        .unwrap_or(false);
    if !is_enemy_turn || session.0.battle_over() {
        *enemy_turn_timer = 0.0;
        return;
    }
    // Also wait for any ongoing animation (e.g. last path tile still playing).
    if !anim_q.is_empty() {
        return;
    }

    // Small delay between enemy actions for readability.
    *enemy_turn_timer += time.delta_secs();
    if *enemy_turn_timer < 0.3 {
        return;
    }
    *enemy_turn_timer = 0.0;

    let s = &mut session.0;

    // Peek at the next enemy action without applying it.
    let action = {
        let Some(battle) = s.battle.as_mut() else {
            return;
        };
        match battle.peek_enemy_action(&mut s.world) {
            Some(a) => a,
            None => {
                // No actor or no AP — pass to advance the queue.
                let actor = battle.current_actor();
                let (_, outcome) = battle.apply_enemy_action(&mut s.world, &Action::Pass);
                s.last_event = Some(carbonthrone::combat::TurnEvent {
                    actor,
                    turn: carbonthrone::combat::Turn::Enemy,
                    actions: vec![],
                    outcome,
                });
                choices_res.needs_refresh = true;
                return;
            }
        }
    };

    match action {
        Action::Move { destination } => {
            let Some(actor) = s.battle.as_ref().and_then(|b| b.current_actor()) else {
                return;
            };
            let speed = s.world.get::<Stats>(actor).map(|st| st.speed).unwrap_or(8);
            let path = bfs_move_path(&mut s.world, actor, (destination.x, destination.y));

            if path.is_empty() {
                // Destination unreachable — pass instead.
                if let Some(battle) = s.battle.as_mut() {
                    let (_, outcome) = battle.apply_enemy_action(&mut s.world, &Action::Pass);
                    s.last_event = Some(carbonthrone::combat::TurnEvent {
                        actor: Some(actor),
                        turn: carbonthrone::combat::Turn::Enemy,
                        actions: vec![],
                        outcome,
                    });
                }
            } else {
                combat_path.total_ap_cost = move_ap_cost(path.len() as i32, speed);
                combat_path.actor = Some(actor);
                combat_path.path = path;
            }
            choices_res.needs_refresh = true;
        }
        other => {
            // Ability or Pass — apply directly.
            let actor = s.battle.as_ref().and_then(|b| b.current_actor());
            if let Some(battle) = s.battle.as_mut() {
                let (_, outcome) = battle.apply_enemy_action(&mut s.world, &other);
                s.last_event = Some(carbonthrone::combat::TurnEvent {
                    actor,
                    turn: carbonthrone::combat::Turn::Enemy,
                    actions: vec![],
                    outcome,
                });
            }
            choices_res.needs_refresh = true;
        }
    }
}

// ── Escape: save and return to main menu ──────────────────────────────────────

fn escape_to_main_menu(
    keys: Res<ButtonInput<KeyCode>>,
    mut session: ResMut<GameSessionRes>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        let data = session.0.to_save_data();
        let _ = save_game(&data);
        next_state.set(AppState::MainMenu);
    }
}

// ── Ending state ──────────────────────────────────────────────────────────────

/// Transition to `AppState::Ended` when the game phase becomes `Ended`.
fn detect_game_ending(
    session: Res<GameSessionRes>,
    current_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if matches!(session.0.phase, GamePhase::Ended(_)) && *current_state.get() != AppState::Ended {
        next_state.set(AppState::Ended);
    }
}

/// Any keypress on the ending screen exits the application.
fn handle_ended_input(keys: Res<ButtonInput<KeyCode>>) {
    if keys.get_just_pressed().next().is_some() {
        std::process::exit(0);
    }
}
