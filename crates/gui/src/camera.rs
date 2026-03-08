use bevy::input::mouse::AccumulatedMouseScroll;
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

fn scroll_zoom_system(
    scroll: Res<AccumulatedMouseScroll>,
    mut camera_q: Query<&mut Projection, With<IsometricCamera>>,
) {
    let total = scroll.delta.y;
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
