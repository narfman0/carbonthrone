use bevy::prelude::*;

/// World-space size of one grid tile.
pub const TILE_SIZE: f32 = 1.0;

/// Y-extent of a flat floor tile mesh (Open tile).
pub const FLOOR_HEIGHT: f32 = 0.1;
/// Y-extent of an impassable obstacle mesh.
pub const OBSTACLE_HEIGHT: f32 = 1.0;
/// Y-extent of a door tile mesh.
pub const DOOR_HEIGHT: f32 = 0.4;
/// Y-extent of a character box mesh.
pub const CHARACTER_HEIGHT: f32 = 0.7;

/// Convert integer grid coordinates to a world-space Vec3 (on the XZ plane).
///
/// Grid x runs west→east, grid y runs north→south.
/// World Y is always 0.0 (floor level).
pub fn grid_to_world(gx: i32, gy: i32) -> Vec3 {
    Vec3::new(gx as f32 * TILE_SIZE, 0.0, gy as f32 * TILE_SIZE)
}

/// Convert a world-space Vec3 back to the nearest grid cell.
pub fn world_to_grid(world: Vec3) -> (i32, i32) {
    (
        (world.x / TILE_SIZE).round() as i32,
        (world.z / TILE_SIZE).round() as i32,
    )
}

/// Compute the center of a zone map in world space given its column/row counts.
pub fn map_center(cols: u32, rows: u32) -> Vec3 {
    Vec3::new(
        (cols as f32 - 1.0) * TILE_SIZE * 0.5,
        0.0,
        (rows as f32 - 1.0) * TILE_SIZE * 0.5,
    )
}
