use bevy::{prelude::*, window::PrimaryWindow};
use carbonthrone::{
    character::{Aggression, Character, CharacterKind},
    game::GamePhase,
    health::Health,
    position::Position,
};

use super::camera::IsometricCamera;
use super::grid::{CHARACTER_HEIGHT, FLOOR_HEIGHT, TILE_SIZE, grid_to_world};
use super::resources::GameSessionRes;
use super::state::AppState;

const HEALTH_BAR_WIDTH: f32 = TILE_SIZE;
const HEALTH_BAR_HEIGHT: f32 = 0.055;
const HEALTH_BAR_THICK: f32 = 0.020;
/// Y offset above the character mesh top.
const HEALTH_BAR_Y_ABOVE: f32 = 0.12;
/// Rotation (radians around Y) to align the bar horizontally in the isometric view.
/// Camera right direction is (1,0,-1)/√2; rotating X axis by +45° achieves this.
const HEALTH_BAR_ROTATION: f32 = std::f32::consts::FRAC_PI_4;

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
}

/// Screen-space label showing character kind and level, positioned above the health bar.
#[derive(Component)]
pub struct CharKindLabel(pub bevy::ecs::entity::Entity);

/// Background of the floating HP bar for a character.
#[derive(Component)]
pub struct HealthBarBg(pub bevy::ecs::entity::Entity);

/// Foreground fill of the floating HP bar — scaled along X by the HP fraction.
#[derive(Component)]
pub struct HealthBarFill(pub bevy::ecs::entity::Entity);

pub struct CharacterVisualsPlugin;

impl Plugin for CharacterVisualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Exploration), spawn_exploration_chars)
            .add_systems(OnExit(AppState::Exploration), despawn_char_visuals)
            .add_systems(
                Update,
                (detect_char_move, animate_char_moves, sync_exploration_chars)
                    .chain()
                    .run_if(in_state(AppState::Exploration)),
            )
            .add_systems(
                OnEnter(AppState::Battle),
                (spawn_battle_chars, spawn_char_kind_labels),
            )
            .add_systems(
                OnExit(AppState::Battle),
                (
                    despawn_char_visuals,
                    despawn_health_bars,
                    despawn_char_kind_labels,
                ),
            )
            .add_systems(
                Update,
                (
                    detect_char_move,
                    animate_char_moves,
                    sync_battle_chars,
                    sync_health_bars,
                    update_char_kind_labels,
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

fn npc_color(aggression: &Aggression) -> Color {
    match aggression {
        // Friendly NPCs share the player green so the player knows they're safe.
        Aggression::Friendly => Color::srgb(0.30, 0.80, 0.30),
        // Neutral NPCs are amber — approachable but not allied.
        Aggression::Neutral => Color::srgb(0.90, 0.70, 0.20),
        // Aggressive NPCs are red — hostile, do not approach.
        Aggression::Aggressive => Color::srgb(0.85, 0.15, 0.10),
        // Lethargic NPCs (degraded Abyssal Fauna) are dark purple.
        Aggression::Lethargic => Color::srgb(0.45, 0.20, 0.55),
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
            base_color: npc_color(&npc.aggression),
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

/// Spawn a health bar (background + fill) above a character at `world_pos`.
fn spawn_health_bar(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    game_entity: bevy::ecs::entity::Entity,
    world_pos: Vec3,
    is_player: bool,
) {
    let bar_y = world_pos.y + CHARACTER_HEIGHT * 0.5 + HEALTH_BAR_Y_ABOVE;
    let bg_pos = Vec3::new(world_pos.x, bar_y, world_pos.z);

    let bar_rotation = Quat::from_rotation_y(HEALTH_BAR_ROTATION);

    // Background (dark).
    let bg_mesh = meshes.add(Cuboid::new(
        HEALTH_BAR_WIDTH,
        HEALTH_BAR_THICK,
        HEALTH_BAR_HEIGHT,
    ));
    let bg_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.15, 0.15),
        unlit: true,
        ..default()
    });
    commands.spawn((
        HealthBarBg(game_entity),
        Mesh3d(bg_mesh),
        MeshMaterial3d(bg_mat),
        Transform::from_translation(bg_pos).with_rotation(bar_rotation),
        GlobalTransform::default(),
    ));

    // Fill (coloured, scaled by HP fraction).
    let fill_color = if is_player {
        Color::srgb(0.10, 0.85, 0.20)
    } else {
        Color::srgb(0.85, 0.15, 0.15)
    };
    let fill_mesh = meshes.add(Cuboid::new(
        HEALTH_BAR_WIDTH,
        HEALTH_BAR_THICK,
        HEALTH_BAR_HEIGHT,
    ));
    let fill_mat = materials.add(StandardMaterial {
        base_color: fill_color,
        unlit: true,
        ..default()
    });
    commands.spawn((
        HealthBarFill(game_entity),
        Mesh3d(fill_mesh),
        MeshMaterial3d(fill_mat),
        Transform::from_translation(bg_pos).with_rotation(bar_rotation),
        GlobalTransform::default(),
    ));
}

// ── Detect move → start animation (shared for exploration and battle) ─────────

fn detect_char_move(
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
        commands.entity(entity).insert(CharacterMoveAnim { target });
    }
}

// ── Advance in-flight move animations ─────────────────────────────────────────

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
        if dist <= step {
            transform.translation = anim.target;
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
            transform.translation = world_pos_for_grid(pos.x, pos.y);
        }
    }

    for (nv, mut transform) in &mut npc_q {
        if let Some(npc) = state.npcs.get(nv.npc_index) {
            transform.translation = world_pos_for_grid(npc.pos.0, npc.pos.1);
        }
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
    let chars: Vec<_> = q
        .iter(world)
        .map(|(e, c, p, h)| {
            (
                e,
                c.kind.clone(),
                c.kind.is_player(),
                (p.x, p.y),
                h.is_alive(),
            )
        })
        .collect();
    for (entity, kind, is_player, (gx, gy), alive) in chars {
        let color = if alive {
            character_color(&kind)
        } else {
            dead_color()
        };
        spawn_char_box(
            &mut commands,
            &mut meshes,
            &mut materials,
            entity,
            gx,
            gy,
            color,
        );
        let world_pos = world_pos_for_grid(gx, gy);
        spawn_health_bar(
            &mut commands,
            &mut meshes,
            &mut materials,
            entity,
            world_pos,
            is_player,
        );
    }
}

// ── Battle: sync positions + alive/dead state ────────────────────────────────

fn sync_battle_chars(
    session: Res<GameSessionRes>,
    mut char_q: Query<
        (&CharacterVisual, &mut Transform, &mut Visibility),
        Without<CharacterMoveAnim>,
    >,
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

// ── Health bars ───────────────────────────────────────────────────────────────

fn sync_health_bars(
    session: Res<GameSessionRes>,
    char_q: Query<(&CharacterVisual, &Transform)>,
    mut bg_q: Query<(&HealthBarBg, &mut Transform, &mut Visibility), Without<CharacterVisual>>,
    mut fill_q: Query<
        (&HealthBarFill, &mut Transform, &mut Visibility),
        (Without<CharacterVisual>, Without<HealthBarBg>),
    >,
) {
    let world = &session.0.world;

    // Build a map from game_entity → visual world position.
    let char_positions: std::collections::HashMap<bevy::ecs::entity::Entity, Vec3> = char_q
        .iter()
        .map(|(cv, t)| (cv.game_entity, t.translation))
        .collect();

    let bar_rotation = Quat::from_rotation_y(HEALTH_BAR_ROTATION);
    // Bar direction in world space after rotation: (1,0,-1)/√2.
    let bar_dir = Vec3::new(1.0, 0.0, -1.0) * std::f32::consts::FRAC_1_SQRT_2;

    // Update background positions.
    for (bg, mut transform, mut vis) in &mut bg_q {
        let game_entity = bg.0;
        let Some(&char_pos) = char_positions.get(&game_entity) else {
            *vis = Visibility::Hidden;
            continue;
        };
        let alive = world
            .get::<Health>(game_entity)
            .map(|h| h.is_alive())
            .unwrap_or(false);
        if !alive {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Inherited;
        let bar_y = char_pos.y + CHARACTER_HEIGHT * 0.5 + HEALTH_BAR_Y_ABOVE;
        transform.translation = Vec3::new(char_pos.x, bar_y, char_pos.z);
        transform.rotation = bar_rotation;
    }

    // Update fill scale and position.
    for (fill, mut transform, mut vis) in &mut fill_q {
        let game_entity = fill.0;
        let Some(&char_pos) = char_positions.get(&game_entity) else {
            *vis = Visibility::Hidden;
            continue;
        };
        let health = world.get::<Health>(game_entity);
        let Some(health) = health else {
            *vis = Visibility::Hidden;
            continue;
        };
        if !health.is_alive() {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Inherited;
        let fraction = if health.max > 0 {
            (health.current as f32 / health.max as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let bar_y = char_pos.y + CHARACTER_HEIGHT * 0.5 + HEALTH_BAR_Y_ABOVE;
        // Left-align along bar_dir: shift center toward the "left" end.
        let center = Vec3::new(char_pos.x, bar_y, char_pos.z);
        let fill_center = center + bar_dir * (HEALTH_BAR_WIDTH * (fraction - 1.0) / 2.0);
        transform.translation = fill_center;
        transform.rotation = bar_rotation;
        transform.scale = Vec3::new(fraction.max(0.001), 1.0, 1.0);
    }
}

fn despawn_health_bars(
    mut commands: Commands,
    bg_q: Query<Entity, With<HealthBarBg>>,
    fill_q: Query<Entity, With<HealthBarFill>>,
) {
    for e in bg_q.iter().chain(fill_q.iter()) {
        commands.entity(e).despawn();
    }
}

// ── Character kind / level labels ─────────────────────────────────────────────

fn spawn_char_kind_labels(mut commands: Commands, mut session: ResMut<GameSessionRes>) {
    let world = &mut session.0.world;
    let mut q = world.query::<(bevy::ecs::entity::Entity, &Character)>();
    let chars: Vec<_> = q
        .iter(world)
        .map(|(e, c)| (e, format!("{:?}", c.kind), c.level))
        .collect();

    for (game_entity, kind_str, level) in chars {
        commands.spawn((
            CharKindLabel(game_entity),
            Text::new(format!("{kind_str} Lv.{level}")),
            TextFont {
                font_size: 9.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..default()
            },
            Visibility::Hidden,
        ));
    }
}

fn update_char_kind_labels(
    session: Res<GameSessionRes>,
    char_visual_q: Query<(&CharacterVisual, &Transform)>,
    camera_q: Query<(&Camera, &GlobalTransform), With<IsometricCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut label_q: Query<(&CharKindLabel, &mut Node, &mut Visibility)>,
) {
    let Ok((cam, cam_transform)) = camera_q.single() else {
        return;
    };
    let Ok(_window) = windows.single() else {
        return;
    };
    let world = &session.0.world;

    // Build map from game_entity → visual transform.
    let char_positions: std::collections::HashMap<bevy::ecs::entity::Entity, Vec3> = char_visual_q
        .iter()
        .map(|(cv, t)| (cv.game_entity, t.translation))
        .collect();

    for (label, mut node, mut vis) in &mut label_q {
        let game_entity = label.0;
        let alive = world
            .get::<Health>(game_entity)
            .map(|h| h.is_alive())
            .unwrap_or(false);
        if !alive {
            *vis = Visibility::Hidden;
            continue;
        }
        let Some(&char_pos) = char_positions.get(&game_entity) else {
            *vis = Visibility::Hidden;
            continue;
        };
        // Project a point above the health bar.
        let label_world_pos = Vec3::new(
            char_pos.x,
            char_pos.y + CHARACTER_HEIGHT * 0.5 + HEALTH_BAR_Y_ABOVE + 0.18,
            char_pos.z,
        );
        if let Ok(screen_pos) = cam.world_to_viewport(cam_transform, label_world_pos) {
            node.left = Val::Px(screen_pos.x - 30.0);
            node.top = Val::Px(screen_pos.y - 8.0);
            *vis = Visibility::Inherited;
        } else {
            *vis = Visibility::Hidden;
        }
    }
}

fn despawn_char_kind_labels(mut commands: Commands, q: Query<Entity, With<CharKindLabel>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
