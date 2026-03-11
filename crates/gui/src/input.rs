use std::collections::HashSet;

use bevy::{prelude::*, window::PrimaryWindow};
use carbonthrone::{
    action_points::ActionPoints,
    character::Character,
    game::GamePhase,
    health::Health,
    player_input::PlayerActionChoice,
    position::Position,
    stats::Stats,
    terrain::LevelMap,
    turn::{Action, MOVE_AP_COST, TurnAction, move_range_per_ap},
};

use super::{
    camera::IsometricCamera,
    character_visuals::CharacterMoveAnim,
    grid::world_to_grid,
    resources::{
        ExplorationRng, GameSessionRes, PendingAbilityTarget, PendingBattlePath, PendingEnemyPath,
        PendingExplorationPath, PendingPlayerChoices, SelectedChoiceIndex,
    },
    state::AppState,
};

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                right_click_navigate.run_if(in_state(AppState::Exploration)),
                advance_exploration_path.run_if(in_state(AppState::Exploration)),
                left_click_npc.run_if(in_state(AppState::Exploration)),
                advance_dialog_click.run_if(in_state(AppState::Dialog)),
                apply_player_choice.run_if(in_state(AppState::Battle)),
                left_click_ability_target.run_if(in_state(AppState::Battle)),
                right_click_battle_move.run_if(in_state(AppState::Battle)),
                advance_battle_path.run_if(in_state(AppState::Battle)),
                advance_enemy_path.run_if(in_state(AppState::Battle)),
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
    session: Res<GameSessionRes>,
    mut path: ResMut<PendingExplorationPath>,
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
        path.0 = bfs;
    }
}

/// Consumes one step of the pending exploration path per frame (when not animating).
fn advance_exploration_path(
    mut session: ResMut<GameSessionRes>,
    mut path: ResMut<PendingExplorationPath>,
    mut rng: ResMut<ExplorationRng>,
    anim_q: Query<(), With<CharacterMoveAnim>>,
) {
    if path.0.is_empty() {
        return;
    }
    // Wait until the player entity is no longer animating.
    if !anim_q.is_empty() {
        return;
    }
    // Get current player position.
    let (px, py) = {
        let GamePhase::Exploration(state) = &session.0.phase else {
            path.0.clear();
            return;
        };
        let world = &session.0.world;
        let pos = world
            .get::<Position>(state.player_entity)
            .copied()
            .unwrap_or(Position::new(0, 0));
        (pos.x, pos.y)
    };

    let next = path.0.remove(0);
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
    mut battle_path: ResMut<PendingBattlePath>,
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
    if battle.turn != carbonthrone::combat::Turn::Player {
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

    // Build set of occupied positions (all living combatants except actor).
    let occupied: HashSet<(i32, i32)> = {
        let mut q = s.world.query::<(&Character, &Position, &Health)>();
        q.iter(&s.world)
            .filter(|(_, _, h)| h.is_alive())
            .map(|(_, p, _)| (p.x, p.y))
            .filter(|&pos| pos != (actor_pos.x, actor_pos.y))
            .collect()
    };

    let path = {
        let Some(map) = s.world.get_resource::<LevelMap>() else {
            return;
        };
        if !map.is_passable(gx, gy) {
            return;
        }
        map.bfs_path((actor_pos.x, actor_pos.y), (gx, gy), &occupied)
    };

    if path.is_empty() {
        return;
    }

    // Check total AP cost for the path.
    let speed = s.world.get::<Stats>(actor).map(|s| s.speed).unwrap_or(8);
    let range = move_range_per_ap(speed);
    let path_len = path.len() as i32;
    let total_cost = MOVE_AP_COST * ((path_len + range - 1) / range.max(1));
    let current_ap = s
        .world
        .get::<ActionPoints>(actor)
        .map(|a| a.current)
        .unwrap_or(0);
    if current_ap < total_cost {
        // Truncate path to what AP allows.
        let max_tiles = current_ap * range;
        let truncated: Vec<(i32, i32)> = path.into_iter().take(max_tiles as usize).collect();
        if !truncated.is_empty() {
            let actual_cost = MOVE_AP_COST * ((truncated.len() as i32 + range - 1) / range.max(1));
            battle_path.path = truncated;
            battle_path.total_ap_cost = actual_cost;
        }
    } else {
        battle_path.path = path;
        battle_path.total_ap_cost = total_cost;
    }
}

/// Executes one step of the battle movement path per frame when not animating.
/// AP is deducted all at once when the full path completes (not per tile).
fn advance_battle_path(
    mut session: ResMut<GameSessionRes>,
    mut battle_path: ResMut<PendingBattlePath>,
    mut choices_res: ResMut<PendingPlayerChoices>,
    anim_q: Query<(), With<CharacterMoveAnim>>,
) {
    if battle_path.path.is_empty() {
        return;
    }
    if !anim_q.is_empty() {
        return;
    }

    let s = &mut session.0;

    // Validate it is still the player's turn and get the current actor.
    let actor = {
        let Some(battle) = s.battle.as_ref() else {
            battle_path.path.clear();
            return;
        };
        if battle.turn != carbonthrone::combat::Turn::Player {
            battle_path.path.clear();
            return;
        }
        let Some(actor) = battle.current_actor() else {
            battle_path.path.clear();
            return;
        };
        actor
    };

    let next = battle_path.path.remove(0);

    // Move position directly — no per-tile AP cost.
    if let Some(mut pos) = s.world.get_mut::<Position>(actor) {
        *pos = Position::new(next.0, next.1);
    }

    choices_res.needs_refresh = true;

    // When the full path is complete, deduct the total AP cost once.
    if battle_path.path.is_empty() {
        let total_cost = battle_path.total_ap_cost;
        if let Some(mut ap) = s.world.get_mut::<ActionPoints>(actor) {
            ap.spend(total_cost);
        }

        let final_pos = Position::new(next.0, next.1);
        let ap_remaining = s
            .world
            .get::<ActionPoints>(actor)
            .map(|a| a.current)
            .unwrap_or(0);

        if ap_remaining == 0 {
            // Formally end this actor's turn via Pass.
            let result = s
                .battle
                .as_mut()
                .unwrap()
                .step_player_action(&mut s.world, &PlayerActionChoice::Pass);
            s.last_event = Some(carbonthrone::combat::TurnEvent {
                actor: Some(actor),
                turn: carbonthrone::combat::Turn::Player,
                actions: vec![TurnAction::Move { to: final_pos }],
                outcome: result.outcome,
            });
        } else {
            s.last_event = Some(carbonthrone::combat::TurnEvent {
                actor: Some(actor),
                turn: carbonthrone::combat::Turn::Player,
                actions: vec![TurnAction::Move { to: final_pos }],
                outcome: None,
            });
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

// ── Combat: advance pending enemy BFS movement path ──────────────────────────

/// Advances a queued enemy movement path one tile per frame (when not animating).
/// AP is deducted in full once the path completes, matching player movement behaviour.
fn advance_enemy_path(
    mut session: ResMut<GameSessionRes>,
    mut enemy_path: ResMut<PendingEnemyPath>,
    mut choices_res: ResMut<PendingPlayerChoices>,
    anim_q: Query<(), With<CharacterMoveAnim>>,
) {
    if enemy_path.path.is_empty() {
        return;
    }
    // Abort if the battle is already decided.
    if session.0.battle_over() {
        enemy_path.path.clear();
        enemy_path.actor = None;
        return;
    }
    if !anim_q.is_empty() {
        return;
    }
    let Some(actor) = enemy_path.actor else {
        enemy_path.path.clear();
        return;
    };

    let next = enemy_path.path.remove(0);
    let s = &mut session.0;
    if let Some(mut pos) = s.world.get_mut::<Position>(actor) {
        *pos = Position::new(next.0, next.1);
    }

    // When the full path is done, deduct the total AP cost once.
    if enemy_path.path.is_empty() {
        let cost = enemy_path.total_ap_cost;
        if let Some(battle) = s.battle.as_mut() {
            let (_, outcome) = battle.charge_enemy_move_ap(&mut s.world, cost);
            s.last_event = Some(carbonthrone::combat::TurnEvent {
                actor: Some(actor),
                turn: carbonthrone::combat::Turn::Enemy,
                actions: vec![TurnAction::Move {
                    to: Position::new(next.0, next.1),
                }],
                outcome,
            });
        }
        choices_res.needs_refresh = true;
    }
}

// ── Combat: auto-advance enemy turn ──────────────────────────────────────────

fn auto_advance_enemy_turn(
    mut session: ResMut<GameSessionRes>,
    mut choices_res: ResMut<PendingPlayerChoices>,
    mut enemy_path: ResMut<PendingEnemyPath>,
    time: Res<Time>,
    mut enemy_turn_timer: Local<f32>,
    anim_q: Query<(), With<CharacterMoveAnim>>,
) {
    // Don't start a new action while a path is animating.
    if !enemy_path.path.is_empty() {
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
            let Some(actor_pos) = s.world.get::<Position>(actor).copied() else {
                return;
            };
            let speed = s.world.get::<Stats>(actor).map(|st| st.speed).unwrap_or(8);
            let range = move_range_per_ap(speed);

            let occupied: std::collections::HashSet<(i32, i32)> = {
                let mut q = s
                    .world
                    .query::<(bevy::ecs::entity::Entity, &Position, &Health)>();
                q.iter(&s.world)
                    .filter(|(e, _, h)| *e != actor && h.is_alive())
                    .map(|(_, p, _)| (p.x, p.y))
                    .collect()
            };

            let path = s
                .world
                .get_resource::<LevelMap>()
                .map(|map| {
                    map.bfs_path(
                        (actor_pos.x, actor_pos.y),
                        (destination.x, destination.y),
                        &occupied,
                    )
                })
                .unwrap_or_default();

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
                let path_len = path.len() as i32;
                let ap_cost = MOVE_AP_COST * ((path_len + range - 1) / range.max(1));
                enemy_path.path = path;
                enemy_path.actor = Some(actor);
                enemy_path.total_ap_cost = ap_cost;
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
