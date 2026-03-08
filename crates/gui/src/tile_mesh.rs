use bevy::prelude::*;
use carbonthrone::terrain::Tile;

use super::grid::{DOOR_HEIGHT, FLOOR_HEIGHT, OBSTACLE_HEIGHT, TILE_SIZE, grid_to_world};
use super::resources::GameSessionRes;
use super::state::AppState;

/// Marker component for visual tile entities — cleared on zone change.
#[derive(Component)]
pub struct TileVisual;

pub struct TilePlugin;

impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Exploration), spawn_exploration_tiles)
            .add_systems(OnExit(AppState::Exploration), despawn_tile_visuals)
            .add_systems(OnEnter(AppState::Battle), spawn_battle_tiles)
            .add_systems(OnExit(AppState::Battle), despawn_tile_visuals);
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
    spawn_tiles(
        &mut commands,
        &mut meshes,
        &mut materials,
        tiles,
    );
}

fn spawn_battle_tiles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    session: Res<GameSessionRes>,
) {
    use carbonthrone::terrain::LevelMap;
    let map = session.0.world.get_resource::<LevelMap>();
    let Some(map) = map else { return };
    let cols = map.cols;
    let rows = map.rows;
    let tiles: Vec<((i32, i32), Tile)> = (0..rows as i32)
        .flat_map(|gy| (0..cols as i32).map(move |gx| ((gx, gy), map.get(gx, gy))))
        .collect();
    spawn_tiles(
        &mut commands,
        &mut meshes,
        &mut materials,
        tiles,
    );
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
