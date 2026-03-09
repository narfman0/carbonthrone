use bevy::{prelude::*, window::PrimaryWindow};
use carbonthrone::{
    action_points::ActionPoints,
    combat::Turn,
    position::Position,
    stats::Stats,
    terrain::{LevelMap, Tile},
    turn::{MOVE_AP_COST, move_range_per_ap},
};

use super::camera::IsometricCamera;
use super::grid::{DOOR_HEIGHT, FLOOR_HEIGHT, OBSTACLE_HEIGHT, TILE_SIZE, grid_to_world, world_to_grid};
use super::resources::{GameSessionRes, PendingPlayerChoices};
use super::state::AppState;

/// Marker component for visual tile entities — cleared on zone change.
#[derive(Component)]
pub struct TileVisual;

/// Marker for a battle-phase movement-range overlay tile.
#[derive(Component)]
pub struct BattleMoveOverlayTile;

/// Single tile that highlights the tile currently under the cursor in battle.
#[derive(Component)]
pub struct BattleCursorHighlight;

pub struct TilePlugin;

impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Exploration), spawn_exploration_tiles)
            .add_systems(OnExit(AppState::Exploration), despawn_tile_visuals)
            .add_systems(
                OnEnter(AppState::Battle),
                (spawn_battle_tiles, spawn_battle_overlays, spawn_cursor_highlight),
            )
            .add_systems(
                OnExit(AppState::Battle),
                (despawn_tile_visuals, despawn_battle_overlays, despawn_cursor_highlight),
            )
            .add_systems(
                Update,
                (
                    refresh_battle_overlays,
                    update_cursor_highlight,
                )
                    .run_if(in_state(AppState::Battle)),
            );
    }
}

fn spawn_exploration_tiles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    session: Res<GameSessionRes>,
) {
    // exploration_map() is a method on GameSessionRes
    let Some(zone_map) = session.exploration_map() else {
        return;
    };
    let cols = zone_map.cols;
    let rows = zone_map.rows;
    // Collect tiles before closure to avoid borrow conflict
    let tiles: Vec<((i32, i32), Tile)> = (0..rows as i32)
        .flat_map(|gy| (0..cols as i32).map(move |gx| ((gx, gy), zone_map.get(gx, gy))))
        .collect();
    spawn_tiles(&mut commands, &mut meshes, &mut materials, tiles);
}

fn spawn_battle_tiles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    session: Res<GameSessionRes>,
) {
    let map = session.0.world.get_resource::<LevelMap>();
    let Some(map) = map else { return };
    let cols = map.cols;
    let rows = map.rows;
    let tiles: Vec<((i32, i32), Tile)> = (0..rows as i32)
        .flat_map(|gy| (0..cols as i32).map(move |gx| ((gx, gy), map.get(gx, gy))))
        .collect();
    spawn_tiles(&mut commands, &mut meshes, &mut materials, tiles);
}

fn spawn_tiles(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    tiles: Vec<((i32, i32), Tile)>,
) {
    for ((gx, gy), tile) in tiles {
        let (height, color) = tile_appearance(tile);
        let mesh = meshes.add(Cuboid::new(TILE_SIZE * 0.98, height, TILE_SIZE * 0.98));
        let mat = materials.add(StandardMaterial {
            base_color: color,
            ..default()
        });
        let world_pos = grid_to_world(gx, gy) + Vec3::Y * (height * 0.5);
        commands.spawn((
            TileVisual,
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::from_translation(world_pos),
            GlobalTransform::default(),
        ));
    }
}

fn despawn_tile_visuals(mut commands: Commands, q: Query<Entity, With<TileVisual>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

fn tile_appearance(tile: Tile) -> (f32, Color) {
    match tile {
        Tile::Open => (FLOOR_HEIGHT, Color::srgb(0.32, 0.32, 0.32)),
        Tile::Obstacle => (OBSTACLE_HEIGHT, Color::srgb(0.50, 0.42, 0.32)),
        Tile::Door => (DOOR_HEIGHT, Color::srgb(0.15, 0.72, 0.92)),
    }
}

// ── Battle: movement range overlays ──────────────────────────────────────────

fn ap_cost_color(cost: i32) -> Color {
    match cost {
        1 => Color::srgba(0.10, 1.00, 0.30, 0.40),
        2 => Color::srgba(1.00, 1.00, 0.10, 0.40),
        3 => Color::srgba(1.00, 0.55, 0.10, 0.40),
        _ => Color::srgba(1.00, 0.20, 0.10, 0.40),
    }
}

fn spawn_reachable_overlays(
    session: &GameSessionRes,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let Some(battle) = session.0.battle.as_ref() else {
        return;
    };
    if battle.turn != Turn::Player {
        return;
    }
    let Some(actor) = battle.current_actor() else {
        return;
    };
    let world = &session.0.world;
    let Some(actor_pos) = world.get::<Position>(actor).copied() else {
        return;
    };
    let ap = world
        .get::<ActionPoints>(actor)
        .map(|a| a.current)
        .unwrap_or(0);
    let speed = world.get::<Stats>(actor).map(|s| s.speed).unwrap_or(8);
    let range = move_range_per_ap(speed);
    let Some(map) = world.get_resource::<LevelMap>() else {
        return;
    };

    // Draw tiles reachable within `ap` AP using speed-scaled movement.
    // Max Manhattan distance reachable = ap * range.
    let max_dist = ap * range;
    for dy in -max_dist..=max_dist {
        for dx in -max_dist..=max_dist {
            let dist = dx.abs() + dy.abs();
            let ap_cost = MOVE_AP_COST * ((dist + range - 1) / range.max(1));
            if dist == 0 || ap_cost > ap {
                continue;
            }
            let tx = actor_pos.x + dx;
            let ty = actor_pos.y + dy;
            if tx < 0 || ty < 0 || tx >= map.cols as i32 || ty >= map.rows as i32 {
                continue;
            }
            if !map.is_passable(tx, ty) {
                continue;
            }

            let color = ap_cost_color(ap_cost);
            let mesh = meshes.add(Cuboid::new(TILE_SIZE * 0.95, 0.02, TILE_SIZE * 0.95));
            let mat = materials.add(StandardMaterial {
                base_color: color,
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            });
            let world_pos = grid_to_world(tx, ty) + Vec3::Y * (FLOOR_HEIGHT + 0.011);
            commands.spawn((
                BattleMoveOverlayTile,
                Mesh3d(mesh),
                MeshMaterial3d(mat),
                Transform::from_translation(world_pos),
                GlobalTransform::default(),
            ));
        }
    }
}

fn spawn_battle_overlays(
    session: Res<GameSessionRes>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    spawn_reachable_overlays(&session, &mut commands, &mut meshes, &mut materials);
}

fn refresh_battle_overlays(
    session: Res<GameSessionRes>,
    choices: Res<PendingPlayerChoices>,
    overlay_q: Query<Entity, With<BattleMoveOverlayTile>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !choices.is_changed() {
        return;
    }
    for e in &overlay_q {
        commands.entity(e).despawn();
    }
    spawn_reachable_overlays(&session, &mut commands, &mut meshes, &mut materials);
}

fn despawn_battle_overlays(
    overlay_q: Query<Entity, With<BattleMoveOverlayTile>>,
    mut commands: Commands,
) {
    for e in &overlay_q {
        commands.entity(e).despawn();
    }
}

// ── Cursor highlight ─────────────────────────────────────────────────────────

fn spawn_cursor_highlight(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Cuboid::new(TILE_SIZE * 0.97, 0.025, TILE_SIZE * 0.97));
    let mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 1.0, 0.55),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    commands.spawn((
        BattleCursorHighlight,
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Transform::from_translation(Vec3::new(0.0, -100.0, 0.0)), // hidden below map initially
        GlobalTransform::default(),
        Visibility::Hidden,
    ));
}

fn update_cursor_highlight(
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<IsometricCamera>>,
    session: Res<GameSessionRes>,
    mut highlight_q: Query<(&mut Transform, &mut Visibility), With<BattleCursorHighlight>>,
) {
    let Ok((mut transform, mut visibility)) = highlight_q.single_mut() else {
        return;
    };

    // Only show during player turn.
    let is_player_turn = session
        .0
        .battle
        .as_ref()
        .map(|b| b.turn == Turn::Player)
        .unwrap_or(false);
    if !is_player_turn {
        *visibility = Visibility::Hidden;
        return;
    }

    let Ok(window) = windows.single() else {
        *visibility = Visibility::Hidden;
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        *visibility = Visibility::Hidden;
        return;
    };
    let Ok((cam, cam_transform)) = camera_q.single() else {
        *visibility = Visibility::Hidden;
        return;
    };
    let Ok(ray) = cam.viewport_to_world(cam_transform, cursor) else {
        *visibility = Visibility::Hidden;
        return;
    };
    if ray.direction.y.abs() < 1e-6 {
        *visibility = Visibility::Hidden;
        return;
    }
    let t = -ray.origin.y / ray.direction.y;
    if t < 0.0 {
        *visibility = Visibility::Hidden;
        return;
    }
    let hit = ray.origin + ray.direction * t;
    let (gx, gy) = world_to_grid(hit);

    // Check bounds.
    let world = &session.0.world;
    let Some(map) = world.get_resource::<LevelMap>() else {
        *visibility = Visibility::Hidden;
        return;
    };
    if gx < 0 || gy < 0 || gx >= map.cols as i32 || gy >= map.rows as i32 {
        *visibility = Visibility::Hidden;
        return;
    }

    let world_pos = grid_to_world(gx, gy) + Vec3::Y * (FLOOR_HEIGHT + 0.013);
    transform.translation = world_pos;
    *visibility = Visibility::Inherited;
}

fn despawn_cursor_highlight(
    mut commands: Commands,
    q: Query<Entity, With<BattleCursorHighlight>>,
) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
