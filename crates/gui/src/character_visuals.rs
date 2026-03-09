use bevy::prelude::*;
use carbonthrone::{
    character::{Character, CharacterKind},
    game::{GamePhase, NpcData},
    health::Health,
    position::Position,
};

use super::grid::{CHARACTER_HEIGHT, FLOOR_HEIGHT, TILE_SIZE, grid_to_world};
use super::resources::GameSessionRes;
use super::state::AppState;

/// Marker for character visual entities: links GUI entity to its game entity.
#[derive(Component, Clone)]
pub struct CharacterVisual {
    pub game_entity: bevy::ecs::entity::Entity,
    /// Last grid position the animation system has acknowledged (to detect moves).
    pub last_grid: (i32, i32),
}

/// Marker for NPC visual entities (NPCs aren't ECS entities in session.world).
#[derive(Component, Clone)]
pub struct NpcVisual {
    pub npc_index: usize,
}

/// Added to a character visual while it is sliding to a new grid cell.
#[derive(Component)]
pub struct CharacterMoveAnim {
    pub target: Vec3,
    /// World position to face after arriving (nearest enemy/NPC), if any.
    pub face_after: Option<Vec3>,
}

/// Thin flat mesh placed on the floor under the currently acting character.
#[derive(Component)]
pub struct ActiveCharOutline;

pub struct CharacterVisualsPlugin;

impl Plugin for CharacterVisualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Exploration), spawn_exploration_chars)
            .add_systems(OnExit(AppState::Exploration), despawn_char_visuals)
            .add_systems(
                Update,
                (
                    detect_player_move,
                    animate_char_moves,
                    sync_exploration_chars,
                )
                    .chain()
                    .run_if(in_state(AppState::Exploration)),
            )
            .add_systems(OnEnter(AppState::Battle), spawn_battle_chars)
            .add_systems(
                OnExit(AppState::Battle),
                (despawn_char_visuals, despawn_active_char_outline),
            )
            .add_systems(
                Update,
                (
                    detect_battle_move,
                    animate_char_moves,
                    sync_battle_chars,
                    update_active_char_outline,
                )
                    .chain()
                    .run_if(in_state(AppState::Battle)),
            );
    }
}

// ── Color coding ─────────────────────────────────────────────────────────────

pub fn character_color(kind: &CharacterKind) -> Color {
    match kind {
        // Player characters — green
        CharacterKind::Researcher
        | CharacterKind::Orin
        | CharacterKind::Doss
        | CharacterKind::Kaleo => Color::srgb(0.20, 0.85, 0.20),
        // The Constancy — red
        CharacterKind::Zealot
        | CharacterKind::Preacher
        | CharacterKind::Purifier
        | CharacterKind::Archon => Color::srgb(0.80, 0.20, 0.10),
        // Drifters — amber
        CharacterKind::Scavenger | CharacterKind::VoidRaider | CharacterKind::DrifterBoss => {
            Color::srgb(0.90, 0.70, 0.10)
        }
        // Automata — steel
        CharacterKind::MaintenanceDrone
        | CharacterKind::SecurityUnit
        | CharacterKind::CombatFrame => Color::srgb(0.50, 0.50, 0.60),
        // Abyssal Fauna — green
        CharacterKind::MoonCrawler | CharacterKind::VoidSpitter | CharacterKind::AbyssalBrute => {
            Color::srgb(0.30, 0.80, 0.30)
        }
        // Station Personnel — tan
        CharacterKind::SalvageOperative
        | CharacterKind::GunForHire
        | CharacterKind::StationGuard
        | CharacterKind::ShockTrooper => Color::srgb(0.70, 0.50, 0.30),
    }
}

fn dead_color() -> Color {
    Color::srgb(0.20, 0.20, 0.20)
}

fn char_mesh() -> Cuboid {
    Cuboid::new(TILE_SIZE * 0.55, CHARACTER_HEIGHT, TILE_SIZE * 0.55)
}

fn char_y_offset() -> f32 {
    FLOOR_HEIGHT + CHARACTER_HEIGHT * 0.5
}

fn world_pos_for_grid(gx: i32, gy: i32) -> Vec3 {
    grid_to_world(gx, gy) + Vec3::Y * char_y_offset()
}

// ── Exploration: spawn ───────────────────────────────────────────────────────

fn spawn_exploration_chars(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    session: Res<GameSessionRes>,
) {
    let GamePhase::Exploration(state) = &session.0.phase else {
        return;
    };
    let world = &session.0.world;

    // Player entity.
    let pos = world
        .get::<Position>(state.player_entity)
        .copied()
        .unwrap_or(Position::new(1, 1));
    let kind = world
        .get::<Character>(state.player_entity)
        .map(|c| c.kind.clone())
        .unwrap_or(CharacterKind::Researcher);
    let color = character_color(&kind);
    spawn_char_box(
        &mut commands,
        &mut meshes,
        &mut materials,
        state.player_entity,
        pos.x,
        pos.y,
        color,
    );

    // NPCs.
    for (i, npc) in state.npcs.iter().enumerate() {
        let mesh = meshes.add(char_mesh());
        let mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.90, 0.55, 0.10),
            ..default()
        });
        let world_pos = world_pos_for_grid(npc.pos.0, npc.pos.1);
        commands.spawn((
            NpcVisual { npc_index: i },
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::from_translation(world_pos),
            GlobalTransform::default(),
        ));
    }
}

fn spawn_char_box(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    game_entity: bevy::ecs::entity::Entity,
    gx: i32,
    gy: i32,
    color: Color,
) {
    let mesh = meshes.add(char_mesh());
    let mat = materials.add(StandardMaterial {
        base_color: color,
        ..default()
    });
    let world_pos = world_pos_for_grid(gx, gy);
    commands.spawn((
        CharacterVisual {
            game_entity,
            last_grid: (gx, gy),
        },
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Transform::from_translation(world_pos),
        GlobalTransform::default(),
    ));
}

// ── Exploration: detect move → start animation ────────────────────────────────

fn detect_player_move(
    session: Res<GameSessionRes>,
    mut char_q: Query<(Entity, &mut CharacterVisual)>,
    mut commands: Commands,
) {
    let GamePhase::Exploration(state) = &session.0.phase else {
        return;
    };
    let world = &session.0.world;
    for (entity, mut cv) in char_q.iter_mut() {
        let Some(pos) = world.get::<Position>(cv.game_entity) else {
            continue;
        };
        let new_grid = (pos.x, pos.y);
        if new_grid == cv.last_grid {
            continue;
        }
        cv.last_grid = new_grid;
        let target = world_pos_for_grid(pos.x, pos.y) + Vec3::Y * char_y_offset();
        let face_after = nearest_npc_world_pos(&state.npcs, pos.x, pos.y);
        commands
            .entity(entity)
            .insert(CharacterMoveAnim { target, face_after });
    }
}

/// Return the world-space floor position of the NPC nearest to (from_x, from_y).
fn nearest_npc_world_pos(npcs: &[NpcData], from_x: i32, from_y: i32) -> Option<Vec3> {
    npcs.iter()
        .min_by_key(|n| {
            let dx = n.pos.0 - from_x;
            let dy = n.pos.1 - from_y;
            dx * dx + dy * dy
        })
        .map(|n| world_pos_for_grid(n.pos.0, n.pos.1))
}

/// Rotation angle (around Y) to make the mesh face toward `dir` in the XZ plane.
fn face_angle(dir: Vec3) -> f32 {
    f32::atan2(-dir.x, -dir.z)
}

// ── Exploration: advance in-flight move animations ────────────────────────────

fn animate_char_moves(
    time: Res<Time>,
    mut q: Query<(Entity, &mut Transform, &CharacterMoveAnim)>,
    mut commands: Commands,
) {
    const SPEED: f32 = 3.0;
    for (entity, mut transform, anim) in q.iter_mut() {
        let to_target = anim.target - transform.translation;
        let dist = Vec2::new(to_target.x, to_target.z).length();
        let step = SPEED * time.delta_secs();

        // Always face the direction of movement.
        if to_target.xz().length_squared() > 0.001 {
            transform.rotation = Quat::from_rotation_y(face_angle(to_target));
        }

        if dist <= step {
            transform.translation = anim.target;
            // After arriving, look toward the nearest NPC/enemy.
            if let Some(face_pos) = anim.face_after {
                let dir = face_pos - transform.translation;
                if dir.xz().length_squared() > 0.001 {
                    transform.rotation = Quat::from_rotation_y(face_angle(dir));
                }
            }
            commands.entity(entity).remove::<CharacterMoveAnim>();
        } else {
            transform.translation += to_target.normalize() * step;
        }
    }
}

// ── Exploration: sync positions (non-animating only) ─────────────────────────

fn sync_exploration_chars(
    session: Res<GameSessionRes>,
    mut char_q: Query<(&CharacterVisual, &mut Transform), Without<CharacterMoveAnim>>,
    mut npc_q: Query<(&NpcVisual, &mut Transform), Without<CharacterVisual>>,
) {
    let GamePhase::Exploration(state) = &session.0.phase else {
        return;
    };
    let world = &session.0.world;

    for (cv, mut transform) in &mut char_q {
        if let Some(pos) = world.get::<Position>(cv.game_entity) {
            transform.translation = world_pos_for_grid(pos.x, pos.y) + Vec3::Y * char_y_offset();
        }
    }

    for (nv, mut transform) in &mut npc_q {
        if let Some(npc) = state.npcs.get(nv.npc_index) {
            transform.translation = world_pos_for_grid(npc.pos.0, npc.pos.1);
        }
    }
}

// ── Battle: detect move → start animation ─────────────────────────────────────

fn detect_battle_move(
    session: Res<GameSessionRes>,
    mut char_q: Query<(Entity, &mut CharacterVisual)>,
    mut commands: Commands,
) {
    let world = &session.0.world;
    for (entity, mut cv) in char_q.iter_mut() {
        let Some(pos) = world.get::<Position>(cv.game_entity) else {
            continue;
        };
        let new_grid = (pos.x, pos.y);
        if new_grid == cv.last_grid {
            continue;
        }
        cv.last_grid = new_grid;
        let target = world_pos_for_grid(pos.x, pos.y);
        commands
            .entity(entity)
            .insert(CharacterMoveAnim { target, face_after: None });
    }
}

// ── Battle: spawn ─────────────────────────────────────────────────────────────

fn spawn_battle_chars(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut session: ResMut<GameSessionRes>,
) {
    let world = &mut session.0.world;
    let mut q = world.query::<(bevy::ecs::entity::Entity, &Character, &Position, &Health)>();
    for (entity, character, pos, health) in q.iter(world) {
        let color = if health.is_alive() {
            character_color(&character.kind)
        } else {
            dead_color()
        };
        spawn_char_box(
            &mut commands,
            &mut meshes,
            &mut materials,
            entity,
            pos.x,
            pos.y,
            color,
        );
    }
}

// ── Battle: sync positions + alive/dead state ────────────────────────────────

fn sync_battle_chars(
    session: Res<GameSessionRes>,
    mut char_q: Query<(&CharacterVisual, &mut Transform, &mut Visibility), Without<CharacterMoveAnim>>,
) {
    let world = &session.0.world;
    for (cv, mut transform, mut vis) in &mut char_q {
        if let Some(pos) = world.get::<Position>(cv.game_entity) {
            transform.translation = world_pos_for_grid(pos.x, pos.y);
        }
        if let Some(health) = world.get::<Health>(cv.game_entity) {
            *vis = if health.is_alive() {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}

// ── Cleanup ───────────────────────────────────────────────────────────────────

fn despawn_char_visuals(
    mut commands: Commands,
    char_q: Query<Entity, Or<(With<CharacterVisual>, With<NpcVisual>)>>,
) {
    for e in &char_q {
        commands.entity(e).despawn();
    }
}

// ── Battle: active character outline ─────────────────────────────────────────

fn update_active_char_outline(
    session: Res<GameSessionRes>,
    mut outline_q: Query<(Entity, &mut Transform), With<ActiveCharOutline>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !session.is_changed() {
        return;
    }

    let actor_world_pos: Option<Vec3> = session
        .0
        .battle
        .as_ref()
        .and_then(|b| b.current_actor())
        .and_then(|a| session.0.world.get::<Position>(a).copied())
        .map(|pos| grid_to_world(pos.x, pos.y) + Vec3::Y * (FLOOR_HEIGHT + 0.006));

    match actor_world_pos {
        None => {
            let entities: Vec<Entity> = outline_q.iter().map(|(e, _)| e).collect();
            for e in entities {
                commands.entity(e).despawn();
            }
        }
        Some(world_pos) => {
            if let Ok((_, mut transform)) = outline_q.single_mut() {
                transform.translation = world_pos;
            } else {
                let mesh = meshes.add(Cuboid::new(TILE_SIZE * 0.72, 0.025, TILE_SIZE * 0.72));
                let mat = materials.add(StandardMaterial {
                    base_color: Color::srgba(0.20, 1.00, 0.35, 0.90),
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                });
                commands.spawn((
                    ActiveCharOutline,
                    Mesh3d(mesh),
                    MeshMaterial3d(mat),
                    Transform::from_translation(world_pos),
                    GlobalTransform::default(),
                ));
            }
        }
    }
}

fn despawn_active_char_outline(mut commands: Commands, q: Query<Entity, With<ActiveCharOutline>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
