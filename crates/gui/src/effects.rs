use bevy::prelude::*;
use carbonthrone::{
    ability::{AbilityEffect, AbilityKind, character_abilities},
    character::Character,
    combat::TurnAction,
    position::Position,
};
use rand::Rng;

use super::camera::IsometricCamera;
use super::character_visuals::{FLOAT_DURATION, FloatingDamageNumber};
use super::grid::grid_to_world;
use super::resources::GameSessionRes;
use super::state::AppState;

// ── Effect kind ───────────────────────────────────────────────────────────────

#[derive(Clone)]
pub enum EffectKind {
    MeleeImpact,
    RangedProjectile,
    HealSparks,
    ApDrainSwirl,
    ApGrantBurst,
    TemporalDisplacement,
    TemporalRecall { from: Vec3, to: Vec3 },
    GlitchTeleport { from: Vec3, to: Vec3 },
    TemporalCollapse,
    Miss,
}

#[derive(Clone)]
pub struct PendingEffect {
    pub kind: EffectKind,
    pub target_pos: Vec3,
    pub source_pos: Vec3,
}

/// Resource queue: detect_ability_events pushes here; spawn_effects drains it.
#[derive(Resource, Default)]
pub struct EffectQueue(pub Vec<PendingEffect>);

// ── Components ────────────────────────────────────────────────────────────────

#[derive(Component)]
struct Particle {
    lifetime: Timer,
    velocity: Vec3,
    start_color: Color,
    end_color: Color,
}

#[derive(Component)]
struct Projectile {
    from: Vec3,
    to: Vec3,
    speed: f32,
    progress: f32,
    impact_target: Vec3,
}

// ── Resources ─────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
struct LastEventId(u64);

#[derive(Resource)]
struct ParticleMeshes {
    quad: Handle<Mesh>,
    bolt: Handle<Mesh>,
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LastEventId>()
            .init_resource::<EffectQueue>()
            .add_systems(Startup, init_particle_meshes)
            .add_systems(
                Update,
                (
                    detect_ability_events,
                    spawn_effects.after(detect_ability_events),
                    update_particles,
                    update_projectiles,
                )
                    .run_if(in_state(AppState::Battle)),
            );
    }
}

fn init_particle_meshes(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let quad = meshes.add(Rectangle::new(0.12, 0.12));
    let bolt = meshes.add(Cuboid::new(0.08, 0.08, 0.3));
    commands.insert_resource(ParticleMeshes { quad, bolt });
}

// ── Detection ─────────────────────────────────────────────────────────────────

fn detect_ability_events(
    session: Res<GameSessionRes>,
    mut last_id: ResMut<LastEventId>,
    mut queue: ResMut<EffectQueue>,
) {
    if !session.is_changed() {
        return;
    }
    let Some(event) = &session.0.last_event else {
        return;
    };
    let event_id = event as *const _ as u64;
    if last_id.0 == event_id {
        return;
    }
    last_id.0 = event_id;

    let world = &session.0.world;
    let actor_pos = event
        .actor
        .and_then(|e| world.get::<Position>(e))
        .map(|p| grid_to_world(p.x, p.y) + Vec3::Y * 0.5)
        .unwrap_or(Vec3::ZERO);

    for action in &event.actions {
        match action {
            TurnAction::UseAbility {
                ability_name,
                target,
                hit,
                ..
            } => {
                let target_pos = target
                    .and_then(|e| world.get::<Position>(e))
                    .map(|p| grid_to_world(p.x, p.y) + Vec3::Y * 0.5)
                    .unwrap_or(actor_pos);
                if !hit {
                    queue.0.push(PendingEffect {
                        kind: EffectKind::Miss,
                        target_pos,
                        source_pos: actor_pos,
                    });
                    continue;
                }
                let kind = determine_effect_kind(ability_name, event.actor, world);
                queue.0.push(PendingEffect {
                    kind,
                    target_pos,
                    source_pos: actor_pos,
                });
            }
            TurnAction::DisplacementQueued {
                target, pushed_to, ..
            } => {
                let target_pos = pushed_to
                    .as_ref()
                    .map(|p| grid_to_world(p.x, p.y) + Vec3::Y * 0.5)
                    .or_else(|| {
                        world
                            .get::<Position>(*target)
                            .map(|p| grid_to_world(p.x, p.y) + Vec3::Y * 0.5)
                    })
                    .unwrap_or(actor_pos);
                queue.0.push(PendingEffect {
                    kind: EffectKind::TemporalDisplacement,
                    target_pos,
                    source_pos: actor_pos,
                });
            }
            TurnAction::TemporalRecalled { from, to, .. } => {
                let from_pos = grid_to_world(from.x, from.y) + Vec3::Y * 0.5;
                let to_pos = grid_to_world(to.x, to.y) + Vec3::Y * 0.5;
                queue.0.push(PendingEffect {
                    kind: EffectKind::TemporalRecall {
                        from: from_pos,
                        to: to_pos,
                    },
                    target_pos: to_pos,
                    source_pos: from_pos,
                });
            }
            TurnAction::GlitchTeleport { from, to, .. } => {
                let from_pos = grid_to_world(from.x, from.y) + Vec3::Y * 0.5;
                let to_pos = grid_to_world(to.x, to.y) + Vec3::Y * 0.5;
                queue.0.push(PendingEffect {
                    kind: EffectKind::GlitchTeleport {
                        from: from_pos,
                        to: to_pos,
                    },
                    target_pos: to_pos,
                    source_pos: from_pos,
                });
            }
            TurnAction::TemporalCollapse { victim, .. } => {
                let victim_pos = world
                    .get::<Position>(*victim)
                    .map(|p| grid_to_world(p.x, p.y) + Vec3::Y * 0.5)
                    .unwrap_or(actor_pos);
                queue.0.push(PendingEffect {
                    kind: EffectKind::TemporalCollapse,
                    target_pos: victim_pos,
                    source_pos: victim_pos,
                });
            }
            _ => {}
        }
    }
}

fn determine_effect_kind(
    name: &str,
    actor: Option<bevy::ecs::entity::Entity>,
    world: &bevy::ecs::world::World,
) -> EffectKind {
    if let Some(actor) = actor {
        if let Some(character) = world.get::<Character>(actor) {
            let abilities = character_abilities(&character.kind);
            if let Some(ability) = abilities.iter().find(|a| a.name == name) {
                return match &ability.effect {
                    AbilityEffect::Heal { .. } => EffectKind::HealSparks,
                    AbilityEffect::DrainAP { .. } => EffectKind::ApDrainSwirl,
                    AbilityEffect::GrantAP { .. } | AbilityEffect::Acceleration { .. } => {
                        EffectKind::ApGrantBurst
                    }
                    AbilityEffect::Displacement { .. } => EffectKind::TemporalDisplacement,
                    // TemporalRecall also fires TurnAction::TemporalRecalled; show as ranged shot here
                    AbilityEffect::TemporalRecall => EffectKind::RangedProjectile,
                    _ => match ability.kind {
                        AbilityKind::Melee => EffectKind::MeleeImpact,
                        AbilityKind::Ranged | AbilityKind::RangedAny | AbilityKind::RangedAlly => {
                            EffectKind::RangedProjectile
                        }
                        AbilityKind::Utility => EffectKind::ApGrantBurst,
                    },
                };
            }
        }
    }
    EffectKind::MeleeImpact
}

// ── Spawn effects ─────────────────────────────────────────────────────────────

fn spawn_effects(
    mut commands: Commands,
    mut queue: ResMut<EffectQueue>,
    particle_meshes: Res<ParticleMeshes>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<IsometricCamera>>,
) {
    let pending: Vec<PendingEffect> = std::mem::take(&mut queue.0);
    for ev in pending {
        match &ev.kind {
            EffectKind::MeleeImpact => {
                spawn_melee_impact(
                    &mut commands,
                    &particle_meshes,
                    &mut materials,
                    ev.target_pos,
                );
            }
            EffectKind::RangedProjectile => {
                spawn_ranged_projectile(
                    &mut commands,
                    &particle_meshes,
                    &mut materials,
                    ev.source_pos,
                    ev.target_pos,
                );
            }
            EffectKind::HealSparks => {
                spawn_heal_sparks(
                    &mut commands,
                    &particle_meshes,
                    &mut materials,
                    ev.target_pos,
                );
            }
            EffectKind::ApDrainSwirl => {
                spawn_ap_drain_swirl(
                    &mut commands,
                    &particle_meshes,
                    &mut materials,
                    ev.target_pos,
                );
            }
            EffectKind::ApGrantBurst => {
                spawn_ap_grant_burst(
                    &mut commands,
                    &particle_meshes,
                    &mut materials,
                    ev.target_pos,
                );
            }
            EffectKind::TemporalDisplacement => {
                spawn_temporal_displacement(
                    &mut commands,
                    &particle_meshes,
                    &mut materials,
                    ev.target_pos,
                );
            }
            EffectKind::TemporalRecall { from, to } => {
                spawn_temporal_flash(
                    &mut commands,
                    &particle_meshes,
                    &mut materials,
                    *from,
                    *to,
                    Color::srgba(0.8, 0.5, 1.0, 1.0),
                );
            }
            EffectKind::GlitchTeleport { from, to } => {
                spawn_temporal_flash(
                    &mut commands,
                    &particle_meshes,
                    &mut materials,
                    *from,
                    *to,
                    Color::srgba(1.0, 1.0, 1.0, 1.0),
                );
            }
            EffectKind::TemporalCollapse => {
                spawn_temporal_collapse(
                    &mut commands,
                    &particle_meshes,
                    &mut materials,
                    ev.target_pos,
                );
            }
            EffectKind::Miss => {
                if let Ok((cam, cam_transform)) = camera_q.single() {
                    if let Ok(screen_pos) = cam.world_to_viewport(cam_transform, ev.target_pos) {
                        commands.spawn((
                            FloatingDamageNumber {
                                timer: FLOAT_DURATION,
                                screen_y: screen_pos.y - 10.0,
                            },
                            Text::new("miss"),
                            TextFont {
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(Color::srgba(0.65, 0.65, 0.65, 1.0)),
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(screen_pos.x - 16.0),
                                top: Val::Px(screen_pos.y - 10.0),
                                ..default()
                            },
                        ));
                    }
                }
            }
        }
    }
}

fn make_particle_mat(
    materials: &mut Assets<StandardMaterial>,
    color: Color,
) -> MeshMaterial3d<StandardMaterial> {
    MeshMaterial3d(materials.add(StandardMaterial {
        base_color: color,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    }))
}

fn spawn_melee_impact(
    commands: &mut Commands,
    meshes: &ParticleMeshes,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
) {
    let mut rng = rand::thread_rng();
    let count = 8;
    for i in 0..count {
        let angle = i as f32 * std::f32::consts::TAU / count as f32;
        let spread = rng.gen_range(0.8_f32..1.2_f32);
        let dir = Vec3::new(angle.cos() * spread, 0.4, angle.sin() * spread).normalize();
        let color = Color::srgba(1.0, 0.5, 0.1, 1.0);
        commands.spawn((
            Particle {
                lifetime: Timer::from_seconds(0.3, TimerMode::Once),
                velocity: dir * 3.0,
                start_color: color,
                end_color: Color::srgba(1.0, 0.5, 0.1, 0.0),
            },
            Mesh3d(meshes.quad.clone()),
            make_particle_mat(materials, color),
            Transform::from_translation(pos),
            GlobalTransform::default(),
        ));
    }
}

fn spawn_ranged_projectile(
    commands: &mut Commands,
    meshes: &ParticleMeshes,
    materials: &mut Assets<StandardMaterial>,
    from: Vec3,
    to: Vec3,
) {
    let color = Color::srgba(1.0, 0.95, 0.5, 1.0);
    let dir = (to - from).normalize_or_zero();
    let rotation = if dir.length_squared() > 0.001 {
        Quat::from_rotation_arc(Vec3::Z, dir)
    } else {
        Quat::IDENTITY
    };
    commands.spawn((
        Projectile {
            from,
            to,
            speed: 8.0,
            progress: 0.0,
            impact_target: to,
        },
        Mesh3d(meshes.bolt.clone()),
        make_particle_mat(materials, color),
        Transform::from_translation(from).with_rotation(rotation),
        GlobalTransform::default(),
    ));
}

fn spawn_heal_sparks(
    commands: &mut Commands,
    meshes: &ParticleMeshes,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
) {
    let mut rng = rand::thread_rng();
    for _ in 0..6 {
        let dx = rng.gen_range(-0.3_f32..0.3_f32);
        let dz = rng.gen_range(-0.3_f32..0.3_f32);
        let color = Color::srgba(0.2, 1.0, 0.3, 1.0);
        commands.spawn((
            Particle {
                lifetime: Timer::from_seconds(0.6, TimerMode::Once),
                velocity: Vec3::new(dx, 1.2, dz),
                start_color: color,
                end_color: Color::srgba(0.2, 1.0, 0.3, 0.0),
            },
            Mesh3d(meshes.quad.clone()),
            make_particle_mat(materials, color),
            Transform::from_translation(pos),
            GlobalTransform::default(),
        ));
    }
}

fn spawn_ap_drain_swirl(
    commands: &mut Commands,
    meshes: &ParticleMeshes,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
) {
    let count = 5;
    for i in 0..count {
        let angle = i as f32 * std::f32::consts::TAU / count as f32;
        let start = pos + Vec3::new(angle.cos() * 0.4, 0.0, angle.sin() * 0.4);
        let tangent = Vec3::new(-angle.sin(), 0.2, angle.cos()).normalize();
        let color = Color::srgba(0.2, 0.4, 1.0, 1.0);
        commands.spawn((
            Particle {
                lifetime: Timer::from_seconds(0.4, TimerMode::Once),
                velocity: tangent * 1.5,
                start_color: color,
                end_color: Color::srgba(0.2, 0.4, 1.0, 0.0),
            },
            Mesh3d(meshes.quad.clone()),
            make_particle_mat(materials, color),
            Transform::from_translation(start),
            GlobalTransform::default(),
        ));
    }
}

fn spawn_ap_grant_burst(
    commands: &mut Commands,
    meshes: &ParticleMeshes,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
) {
    let mut rng = rand::thread_rng();
    for _ in 0..5 {
        let dx = rng.gen_range(-0.4_f32..0.4_f32);
        let dz = rng.gen_range(-0.4_f32..0.4_f32);
        let color = Color::srgba(0.3, 1.0, 1.0, 1.0);
        commands.spawn((
            Particle {
                lifetime: Timer::from_seconds(0.4, TimerMode::Once),
                velocity: Vec3::new(dx, 1.5, dz),
                start_color: color,
                end_color: Color::srgba(0.3, 1.0, 1.0, 0.0),
            },
            Mesh3d(meshes.quad.clone()),
            make_particle_mat(materials, color),
            Transform::from_translation(pos),
            GlobalTransform::default(),
        ));
    }
}

fn spawn_temporal_displacement(
    commands: &mut Commands,
    meshes: &ParticleMeshes,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
) {
    let count = 12;
    for i in 0..count {
        let angle = i as f32 * std::f32::consts::TAU / count as f32;
        let dir = Vec3::new(angle.cos(), 0.0, angle.sin());
        let color = Color::srgba(0.7, 0.2, 1.0, 1.0);
        commands.spawn((
            Particle {
                lifetime: Timer::from_seconds(0.5, TimerMode::Once),
                velocity: dir * 2.5,
                start_color: color,
                end_color: Color::srgba(0.7, 0.2, 1.0, 0.0),
            },
            Mesh3d(meshes.quad.clone()),
            make_particle_mat(materials, color),
            Transform::from_translation(pos),
            GlobalTransform::default(),
        ));
    }
}

fn spawn_temporal_flash(
    commands: &mut Commands,
    meshes: &ParticleMeshes,
    materials: &mut Assets<StandardMaterial>,
    from: Vec3,
    to: Vec3,
    color: Color,
) {
    let sc = color.to_srgba();
    let faded = Color::srgba(sc.red, sc.green, sc.blue, 0.0);
    for center in [from, to] {
        for _ in 0..4 {
            commands.spawn((
                Particle {
                    lifetime: Timer::from_seconds(0.3, TimerMode::Once),
                    velocity: Vec3::ZERO,
                    start_color: color,
                    end_color: faded,
                },
                Mesh3d(meshes.quad.clone()),
                make_particle_mat(materials, color),
                Transform::from_translation(center).with_scale(Vec3::splat(3.0)),
                GlobalTransform::default(),
            ));
        }
    }
}

fn spawn_temporal_collapse(
    commands: &mut Commands,
    meshes: &ParticleMeshes,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
) {
    let count = 16;
    for i in 0..count {
        let phi = i as f32 * std::f32::consts::TAU / count as f32;
        let theta = (i as f32 / count as f32) * std::f32::consts::PI;
        let dir = Vec3::new(
            theta.sin() * phi.cos(),
            theta.cos(),
            theta.sin() * phi.sin(),
        )
        .normalize_or_zero();
        let color = if i % 2 == 0 {
            Color::srgba(1.0, 0.15, 0.1, 1.0)
        } else {
            Color::srgba(1.0, 1.0, 0.9, 1.0)
        };
        let sc = color.to_srgba();
        let faded = Color::srgba(sc.red, sc.green, sc.blue, 0.0);
        commands.spawn((
            Particle {
                lifetime: Timer::from_seconds(0.6, TimerMode::Once),
                velocity: dir * 3.5,
                start_color: color,
                end_color: faded,
            },
            Mesh3d(meshes.quad.clone()),
            make_particle_mat(materials, color),
            Transform::from_translation(pos),
            GlobalTransform::default(),
        ));
    }
}

// ── Update systems ─────────────────────────────────────────────────────────────

fn update_particles(
    time: Res<Time>,
    mut q: Query<(
        Entity,
        &mut Particle,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    for (entity, mut particle, mut transform, mat_handle) in q.iter_mut() {
        particle.lifetime.tick(time.delta());
        let t = particle.lifetime.fraction();
        transform.translation += particle.velocity * dt;

        let sc = particle.start_color.to_srgba();
        let ec = particle.end_color.to_srgba();
        let color = Color::srgba(
            sc.red + (ec.red - sc.red) * t,
            sc.green + (ec.green - sc.green) * t,
            sc.blue + (ec.blue - sc.blue) * t,
            sc.alpha + (ec.alpha - sc.alpha) * t,
        );
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            mat.base_color = color;
        }

        if particle.lifetime.just_finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn update_projectiles(
    time: Res<Time>,
    mut q: Query<(Entity, &mut Projectile, &mut Transform)>,
    mut commands: Commands,
    particle_meshes: Res<ParticleMeshes>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let dt = time.delta_secs();
    for (entity, mut projectile, mut transform) in q.iter_mut() {
        let dist = (projectile.to - projectile.from).length().max(0.001);
        projectile.progress = (projectile.progress + projectile.speed * dt / dist).min(1.0);
        transform.translation = projectile.from.lerp(projectile.to, projectile.progress);

        if projectile.progress >= 1.0 {
            let impact = projectile.impact_target;
            commands.entity(entity).despawn();
            spawn_melee_impact(&mut commands, &particle_meshes, &mut materials, impact);
        }
    }
}
