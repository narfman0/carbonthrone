use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

use super::grid::map_center;

/// Marker for the primary isometric camera so we can query it specifically.
#[derive(Component)]
pub struct IsometricCamera;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera)
            .add_systems(Update, scroll_zoom_system);
    }
}

fn spawn_camera(mut commands: Commands) {
    // Default zone size; camera repositioned when zone loads.
    let center = map_center(16, 12);
    let offset = Vec3::new(18.0, 18.0, 18.0);
    let cam_pos = center + offset;

    commands.spawn((
        IsometricCamera,
        Camera3d::default(),
        Projection::Orthographic(OrthographicProjection {
            scale: 0.07,
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_translation(cam_pos).looking_at(center, Vec3::Y),
    ));

    // Directional sun light.
    commands.spawn((
        DirectionalLight {
            illuminance: 12000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Reposition the camera to look at the center of a new zone.
pub fn reposition_camera(
    mut camera_q: Query<&mut Transform, With<IsometricCamera>>,
    cols: u32,
    rows: u32,
) {
    let center = map_center(cols, rows);
    let offset = Vec3::new(18.0, 18.0, 18.0);
    let cam_pos = center + offset;
    if let Ok(mut transform) = camera_q.single_mut() {
        *transform = Transform::from_translation(cam_pos).looking_at(center, Vec3::Y);
    }
}

fn scroll_zoom_system(
    mut scroll: EventReader<MouseWheel>,
    mut camera_q: Query<&mut Projection, With<IsometricCamera>>,
) {
    let total: f32 = scroll.read().map(|e| e.y).sum();
    if total == 0.0 {
        return;
    }
    let factor = if total > 0.0 { 0.9f32 } else { 1.1f32 };
    if let Ok(mut proj) = camera_q.single_mut() {
        if let Projection::Orthographic(ortho) = &mut *proj {
            ortho.scale = (ortho.scale * factor).clamp(0.02, 0.3);
        }
    }
}
