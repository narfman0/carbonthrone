use bevy::{prelude::*, window::PrimaryWindow};
use carbonthrone::{
    combat::Turn,
    terrain::{CoverLevel, Direction, LevelMap, Tile},
};

use super::camera::IsometricCamera;
use super::grid::{
    DOOR_HEIGHT, FLOOR_HEIGHT, OBSTACLE_HEIGHT, TILE_SIZE, grid_to_world, world_to_grid,
};
use super::resources::GameSessionRes;
use super::state::AppState;

/// Marker component for visual tile entities — cleared on zone change.
#[derive(Component)]
pub struct TileVisual;

/// Marker for a battle-phase directional cover icon overlay.
#[derive(Component)]
pub struct CoverIcon;

/// Single tile that highlights the tile currently under the cursor in battle.
#[derive(Component)]
pub struct BattleCursorHighlight;

pub struct TilePlugin;

impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Exploration), spawn_exploration_tiles)
            .add_systems(OnEnter(AppState::Battle), despawn_tile_visuals)
            .add_systems(OnEnter(AppState::MainMenu), despawn_tile_visuals)
            .add_systems(OnEnter(AppState::Ended), despawn_tile_visuals)
            .add_systems(
                OnEnter(AppState::Battle),
                (
                    spawn_battle_tiles,
                    spawn_cover_icons,
                    spawn_cursor_highlight,
                ),
            )
            .add_systems(
                OnExit(AppState::Battle),
                (
                    despawn_tile_visuals,
                    despawn_cover_icons,
                    despawn_cursor_highlight,
                ),
            )
            .add_systems(
                Update,
                update_cursor_highlight.run_if(in_state(AppState::Battle)),
            );
    }
}

fn spawn_exploration_tiles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    session: Res<GameSessionRes>,
    mut camera_q: Query<&mut Transform, With<super::camera::IsometricCamera>>,
    existing: Query<Entity, With<TileVisual>>,
) {
    // Already spawned (e.g. returning from Dialog state) — skip.
    if !existing.is_empty() {
        return;
    }
    // exploration_map() is a method on GameSessionRes
    let Some(zone_map) = session.exploration_map() else {
        return;
    };

    // Center the camera on the new zone exactly once, when tiles spawn.
    if let Ok(mut transform) = camera_q.single_mut() {
        let center = super::grid::map_center(zone_map.cols, zone_map.rows);
        let offset = Vec3::new(18.0, 18.0, 18.0);
        *transform = Transform::from_translation(center + offset).looking_at(center, Vec3::Y);
    }
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

// ── Battle: directional cover icons ──────────────────────────────────────────

fn cover_icon_color(level: CoverLevel) -> Color {
    match level {
        CoverLevel::Partial => Color::srgba(0.9, 0.8, 0.1, 0.75),
        CoverLevel::Full => Color::srgba(1.0, 0.4, 0.0, 0.85),
        CoverLevel::None => unreachable!(),
    }
}

fn spawn_cover_icons(
    session: Res<GameSessionRes>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let world = &session.0.world;
    let Some(map) = world.get_resource::<LevelMap>() else {
        return;
    };

    // Directions and their world-space offsets and strip dimensions (width, depth).
    let dirs: [(Direction, Vec3, f32, f32); 4] = [
        (
            Direction::North,
            Vec3::new(0.0, 0.0, -TILE_SIZE * 0.5),
            TILE_SIZE * 0.88,
            0.06,
        ),
        (
            Direction::South,
            Vec3::new(0.0, 0.0, TILE_SIZE * 0.5),
            TILE_SIZE * 0.88,
            0.06,
        ),
        (
            Direction::East,
            Vec3::new(TILE_SIZE * 0.5, 0.0, 0.0),
            0.06,
            TILE_SIZE * 0.88,
        ),
        (
            Direction::West,
            Vec3::new(-TILE_SIZE * 0.5, 0.0, 0.0),
            0.06,
            TILE_SIZE * 0.88,
        ),
    ];

    for gy in 0..map.rows as i32 {
        for gx in 0..map.cols as i32 {
            if !map.is_passable(gx, gy) {
                continue;
            }
            let tile_center = grid_to_world(gx, gy) + Vec3::Y * (FLOOR_HEIGHT + 0.015);
            for (dir, offset, sx, sz) in &dirs {
                let level = map.get_cover(gx, gy, *dir);
                if level == CoverLevel::None {
                    continue;
                }
                let color = cover_icon_color(level);
                let mesh = meshes.add(Cuboid::new(*sx, 0.02, *sz));
                let mat = materials.add(StandardMaterial {
                    base_color: color,
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                });
                commands.spawn((
                    CoverIcon,
                    Mesh3d(mesh),
                    MeshMaterial3d(mat),
                    Transform::from_translation(tile_center + *offset),
                    GlobalTransform::default(),
                ));
            }
        }
    }
}

fn despawn_cover_icons(mut commands: Commands, q: Query<Entity, With<CoverIcon>>) {
    for e in &q {
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

fn despawn_cursor_highlight(mut commands: Commands, q: Query<Entity, With<BattleCursorHighlight>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
