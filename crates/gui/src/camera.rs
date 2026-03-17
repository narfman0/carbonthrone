use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;

use super::grid::map_center;
use super::resources::ScreenShake;

/// Marker for the primary isometric camera so we can query it specifically.
#[derive(Component)]
pub struct IsometricCamera;

/// Marker for the main directional sun light so zone-tint systems can query it.
#[derive(Component)]
pub struct SunLight;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera)
            .add_systems(Update, (scroll_zoom_system, screen_shake_system).chain());
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
            scale: 0.02,
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_translation(cam_pos).looking_at(center, Vec3::Y),
    ));

    // Directional sun light.
    commands.spawn((
        SunLight,
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Ambient fill so shadows aren't pitch-black.
    commands.spawn(AmbientLight {
        color: Color::WHITE,
        brightness: 180.0,
        ..default()
    });
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

fn screen_shake_system(
    time: Res<Time>,
    mut shake: ResMut<ScreenShake>,
    mut camera_q: Query<&mut Transform, With<IsometricCamera>>,
) {
    let Ok(mut transform) = camera_q.single_mut() else {
        return;
    };

    // Remove the previous frame's shake offset first.
    transform.translation -= shake.current_offset;

    if shake.timer <= 0.0 {
        shake.current_offset = Vec3::ZERO;
        return;
    }

    shake.timer -= time.delta_secs();
    let t = time.elapsed_secs();
    let decay = (shake.timer / 0.3).clamp(0.0, 1.0).sqrt();
    let magnitude = shake.magnitude * decay;

    // Deterministic sine-wave shake — no rand needed.
    let offset = Vec3::new(
        magnitude * (t * 41.0).sin(),
        0.0,
        magnitude * (t * 37.0).cos(),
    );
    shake.current_offset = offset;
    transform.translation += offset;
}
