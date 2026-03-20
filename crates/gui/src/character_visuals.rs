use bevy::{prelude::*, window::PrimaryWindow};
use carbonthrone::{
    character::{Aggression, Character, CharacterKind},
    game::GamePhase,
    health::Health,
    position::Position,
};

use super::camera::IsometricCamera;
use super::grid::{FLOOR_HEIGHT, TILE_SIZE, grid_to_world};
use super::resources::{GameSessionRes, ScreenShake};
use super::state::AppState;

const HEALTH_BAR_WIDTH: f32 = TILE_SIZE;
const HEALTH_BAR_HEIGHT: f32 = 0.055;
const HEALTH_BAR_THICK: f32 = 0.020;
const HEALTH_BAR_Y_ABOVE: f32 = 0.12;
const HEALTH_BAR_ROTATION: f32 = std::f32::consts::FRAC_PI_4;

pub const FLOAT_DURATION: f32 = 0.8;
const FLOAT_SPEED: f32 = 55.0; // pixels per second upward

// ── Components ────────────────────────────────────────────────────────────────

/// Marker for character visual entities: links GUI entity to its game entity.
#[derive(Component, Clone)]
pub struct CharacterVisual {
    pub game_entity: bevy::ecs::entity::Entity,
    pub last_grid: (i32, i32),
    /// Last known HP — used to detect damage and trigger flash.
    pub last_hp: i32,
    /// Base (un-flashed) color for restoration after damage flash.
    pub base_color: Color,
}

/// Marker for NPC visual entities (NPCs aren't ECS entities in session.world).
#[derive(Component, Clone)]
pub struct NpcVisual {
    pub npc_index: usize,
}

/// Added to a character visual while it is sliding to a new grid cell.
#[derive(Component)]
pub struct CharacterMoveAnim {
    pub start: Vec3,
    pub target: Vec3,
    /// Animation progress 0.0 → 1.0.
    pub progress: f32,
}

/// Active while a character is flashing red from a hit.
#[derive(Component)]
pub struct DamageFlash {
    pub timer: Timer,
    pub original_color: Color,
}

/// Dissolve-out effect for a just-died character visual.
#[derive(Component)]
pub struct Dissolve {
    pub timer: Timer,
    pub mat_handle: Handle<StandardMaterial>,
}

/// Screen-space floating damage number that drifts upward and fades.
#[derive(Component)]
pub struct FloatingDamageNumber {
    pub timer: f32,
    pub screen_y: f32,
}

/// Screen-space label showing character kind and level.
#[derive(Component)]
pub struct CharKindLabel(pub bevy::ecs::entity::Entity);

/// Background of the floating HP bar for a character.
#[derive(Component)]
pub struct HealthBarBg(pub bevy::ecs::entity::Entity);

/// Foreground fill of the floating HP bar.
#[derive(Component)]
pub struct HealthBarFill(pub bevy::ecs::entity::Entity);

// ── Event ─────────────────────────────────────────────────────────────────────

// ── Plugin ────────────────────────────────────────────────────────────────────

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
                    despawn_floating_damage,
                ),
            )
            .add_systems(
                Update,
                (
                    detect_char_move,
                    animate_char_moves,
                    detect_damage_and_spawn_effects,
                    update_damage_flash,
                    update_floating_damage_text,
                )
                    .chain()
                    .run_if(in_state(AppState::Battle)),
            )
            .add_systems(
                Update,
                (
                    sync_battle_chars,
                    sync_health_bars,
                    update_char_kind_labels,
                    update_dissolve,
                )
                    .chain()
                    .run_if(in_state(AppState::Battle)),
            );
    }
}

// ── Color helpers ─────────────────────────────────────────────────────────────

pub fn character_color(kind: &CharacterKind) -> Color {
    match kind {
        CharacterKind::Researcher
        | CharacterKind::Orin
        | CharacterKind::Doss
        | CharacterKind::Kaleo => Color::srgb(0.20, 0.85, 0.20),
        CharacterKind::Zealot
        | CharacterKind::Preacher
        | CharacterKind::Purifier
        | CharacterKind::Archon => Color::srgb(0.80, 0.20, 0.10),
        CharacterKind::Scavenger | CharacterKind::VoidRaider | CharacterKind::DrifterBoss => {
            Color::srgb(0.90, 0.70, 0.10)
        }
        CharacterKind::MaintenanceDrone
        | CharacterKind::SecurityUnit
        | CharacterKind::CombatFrame => Color::srgb(0.50, 0.50, 0.60),
        CharacterKind::MoonCrawler | CharacterKind::VoidSpitter | CharacterKind::AbyssalBrute => {
            Color::srgb(0.30, 0.80, 0.30)
        }
        CharacterKind::SalvageOperative
        | CharacterKind::GunForHire
        | CharacterKind::StationGuard
        | CharacterKind::ShockTrooper => Color::srgb(0.70, 0.50, 0.30),
    }
}

fn npc_color(aggression: &Aggression) -> Color {
    match aggression {
        Aggression::Friendly => Color::srgb(0.30, 0.80, 0.30),
        Aggression::Neutral => Color::srgb(0.90, 0.70, 0.20),
        Aggression::Aggressive => Color::srgb(0.85, 0.15, 0.10),
        Aggression::Lethargic => Color::srgb(0.45, 0.20, 0.55),
    }
}

fn dead_color() -> Color {
    Color::srgb(0.20, 0.20, 0.20)
}

// ── Mesh sizing by CharacterKind ──────────────────────────────────────────────

fn char_mesh_for(kind: &CharacterKind) -> Cuboid {
    let (w, h, d) = match kind {
        // Player characters — tall and slim.
        CharacterKind::Researcher
        | CharacterKind::Orin
        | CharacterKind::Doss
        | CharacterKind::Kaleo => (0.55, 1.3, 0.55),
        // Large heavy enemies — wide and squat.
        CharacterKind::AbyssalBrute | CharacterKind::CombatFrame => (0.75, 1.0, 0.75),
        // Small light enemies — short.
        CharacterKind::MoonCrawler | CharacterKind::MaintenanceDrone => (0.45, 0.6, 0.45),
        // All others — standard.
        _ => (0.55, 0.9, 0.55),
    };
    Cuboid::new(TILE_SIZE * w, h, TILE_SIZE * d)
}

fn char_height_for(kind: &CharacterKind) -> f32 {
    match kind {
        CharacterKind::Researcher
        | CharacterKind::Orin
        | CharacterKind::Doss
        | CharacterKind::Kaleo => 1.3,
        CharacterKind::AbyssalBrute | CharacterKind::CombatFrame => 1.0,
        CharacterKind::MoonCrawler | CharacterKind::MaintenanceDrone => 0.6,
        _ => 0.9,
    }
}

fn char_y_offset(kind: &CharacterKind) -> f32 {
    FLOOR_HEIGHT + char_height_for(kind) * 0.5
}

fn world_pos_for_grid(gx: i32, gy: i32, kind: &CharacterKind) -> Vec3 {
    grid_to_world(gx, gy) + Vec3::Y * char_y_offset(kind)
}

// ── Smoothstep ────────────────────────────────────────────────────────────────

fn smoothstep(x: f32) -> f32 {
    x * x * (3.0 - 2.0 * x)
}

// ── Color lerp helper ─────────────────────────────────────────────────────────

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let la = a.to_srgba();
    let lb = b.to_srgba();
    Color::srgba(
        la.red + (lb.red - la.red) * t,
        la.green + (lb.green - la.green) * t,
        la.blue + (lb.blue - la.blue) * t,
        la.alpha + (lb.alpha - la.alpha) * t,
    )
}

// ── Exploration: spawn ────────────────────────────────────────────────────────

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
        &kind,
        pos.x,
        pos.y,
        color,
        0,
    );

    for (i, npc) in state.npcs.iter().enumerate() {
        let npc_kind = CharacterKind::Researcher; // NPCs in exploration use default size
        let mesh = meshes.add(char_mesh_for(&npc_kind));
        let mat = materials.add(StandardMaterial {
            base_color: npc_color(&npc.aggression),
            ..default()
        });
        let world_p = world_pos_for_grid(npc.pos.0, npc.pos.1, &npc_kind);
        commands.spawn((
            NpcVisual { npc_index: i },
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::from_translation(world_p),
            GlobalTransform::default(),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_char_box(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    game_entity: bevy::ecs::entity::Entity,
    kind: &CharacterKind,
    gx: i32,
    gy: i32,
    color: Color,
    initial_hp: i32,
) {
    let mesh = meshes.add(char_mesh_for(kind));
    let mat = materials.add(StandardMaterial {
        base_color: color,
        ..default()
    });
    let world_pos = world_pos_for_grid(gx, gy, kind);
    commands.spawn((
        CharacterVisual {
            game_entity,
            last_grid: (gx, gy),
            last_hp: initial_hp,
            base_color: color,
        },
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Transform::from_translation(world_pos),
        GlobalTransform::default(),
    ));
}

// ── Health bar spawn ──────────────────────────────────────────────────────────

fn spawn_health_bar(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    game_entity: bevy::ecs::entity::Entity,
    world_pos: Vec3,
    char_h: f32,
    is_player: bool,
) {
    let bar_y = world_pos.y + char_h * 0.5 + HEALTH_BAR_Y_ABOVE;
    let bg_pos = Vec3::new(world_pos.x, bar_y, world_pos.z);
    let bar_rotation = Quat::from_rotation_y(HEALTH_BAR_ROTATION);

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

// ── Detect move → start animation ────────────────────────────────────────────

fn detect_char_move(
    session: Res<GameSessionRes>,
    mut char_q: Query<(Entity, &mut CharacterVisual, &Transform), Without<CharacterMoveAnim>>,
    mut commands: Commands,
) {
    let world = &session.0.world;
    for (entity, mut cv, transform) in char_q.iter_mut() {
        let Some(pos) = world.get::<Position>(cv.game_entity) else {
            continue;
        };
        let new_grid = (pos.x, pos.y);
        if new_grid == cv.last_grid {
            continue;
        }
        cv.last_grid = new_grid;
        let kind = world
            .get::<Character>(cv.game_entity)
            .map(|c| c.kind.clone())
            .unwrap_or(CharacterKind::Researcher);
        let target = world_pos_for_grid(pos.x, pos.y, &kind);
        commands.entity(entity).insert(CharacterMoveAnim {
            start: transform.translation,
            target,
            progress: 0.0,
        });
    }
}

// ── Advance animations with smoothstep ───────────────────────────────────────

fn animate_char_moves(
    time: Res<Time>,
    mut q: Query<(Entity, &mut Transform, &mut CharacterMoveAnim)>,
    mut commands: Commands,
) {
    const SPEED: f32 = 3.0;
    for (entity, mut transform, mut anim) in q.iter_mut() {
        let total = (anim.target - anim.start).length().max(0.001);
        anim.progress = (anim.progress + SPEED * time.delta_secs() / total).min(1.0);
        let t = smoothstep(anim.progress);
        transform.translation = anim.start.lerp(anim.target, t);
        if anim.progress >= 1.0 {
            transform.translation = anim.target;
            commands.entity(entity).remove::<CharacterMoveAnim>();
        }
    }
}

// ── Exploration: sync positions ───────────────────────────────────────────────

fn sync_exploration_chars(
    session: Res<GameSessionRes>,
    mut char_q: Query<(&CharacterVisual, &mut Transform), Without<CharacterMoveAnim>>,
    mut npc_q: Query<(&NpcVisual, &mut Transform), Without<CharacterVisual>>,
) {
    let GamePhase::Exploration(state) = &session.0.phase else {
        return;
    };
    let world = &session.0.world;
    let default_kind = CharacterKind::Researcher;

    for (cv, mut transform) in &mut char_q {
        if let Some(pos) = world.get::<Position>(cv.game_entity) {
            let kind = world
                .get::<Character>(cv.game_entity)
                .map(|c| c.kind.clone())
                .unwrap_or(CharacterKind::Researcher);
            transform.translation = world_pos_for_grid(pos.x, pos.y, &kind);
        }
    }

    for (nv, mut transform) in &mut npc_q {
        if let Some(npc) = state.npcs.get(nv.npc_index) {
            transform.translation = world_pos_for_grid(npc.pos.0, npc.pos.1, &default_kind);
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
                h.current,
            )
        })
        .collect();
    for (entity, kind, is_player, (gx, gy), alive, current_hp) in chars {
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
            &kind,
            gx,
            gy,
            color,
            current_hp,
        );
        let world_pos = world_pos_for_grid(gx, gy, &kind);
        let char_h = char_height_for(&kind);
        spawn_health_bar(
            &mut commands,
            &mut meshes,
            &mut materials,
            entity,
            world_pos,
            char_h,
            is_player,
        );
    }
}

// ── Battle: sync positions + alive/dead ──────────────────────────────────────

fn sync_battle_chars(
    session: Res<GameSessionRes>,
    mut char_q: Query<
        (
            Entity,
            &CharacterVisual,
            &mut Transform,
            &mut Visibility,
            Option<&Dissolve>,
            &MeshMaterial3d<StandardMaterial>,
        ),
        Without<CharacterMoveAnim>,
    >,
    mut commands: Commands,
) {
    let world = &session.0.world;
    for (entity, cv, mut transform, mut vis, dissolve, mat_handle) in &mut char_q {
        if let Some(pos) = world.get::<Position>(cv.game_entity) {
            let kind = world
                .get::<Character>(cv.game_entity)
                .map(|c| c.kind.clone())
                .unwrap_or(CharacterKind::Researcher);
            transform.translation = world_pos_for_grid(pos.x, pos.y, &kind);
        }
        if let Some(health) = world.get::<Health>(cv.game_entity) {
            if health.is_alive() {
                *vis = Visibility::Inherited;
            } else if dissolve.is_none() {
                commands.entity(entity).insert(Dissolve {
                    timer: Timer::from_seconds(0.5, TimerMode::Once),
                    mat_handle: mat_handle.0.clone(),
                });
                *vis = Visibility::Inherited;
            }
            // If Dissolve is present, update_dissolve manages visibility.
        }
    }
}

// ── Damage detection + flash + floating text (combined) ───────────────────────

fn detect_damage_and_spawn_effects(
    session: Res<GameSessionRes>,
    mut char_q: Query<(Entity, &mut CharacterVisual, &Transform)>,
    mut commands: Commands,
    mut shake: ResMut<ScreenShake>,
    camera_q: Query<(&Camera, &GlobalTransform), With<IsometricCamera>>,
) {
    let Ok((cam, cam_transform)) = camera_q.single() else {
        return;
    };
    let world = &session.0.world;
    for (entity, mut cv, transform) in char_q.iter_mut() {
        let Some(health) = world.get::<Health>(cv.game_entity) else {
            continue;
        };
        let current_hp = health.current;
        if current_hp < cv.last_hp {
            let damage = cv.last_hp - current_hp;
            // Start damage flash.
            commands.entity(entity).insert(DamageFlash {
                timer: Timer::from_seconds(0.25, TimerMode::Once),
                original_color: cv.base_color,
            });
            // Spawn floating damage number at screen position.
            let text_world = transform.translation + Vec3::Y * 0.4;
            if let Ok(screen_pos) = cam.world_to_viewport(cam_transform, text_world) {
                commands.spawn((
                    FloatingDamageNumber {
                        timer: FLOAT_DURATION,
                        screen_y: screen_pos.y - 10.0,
                    },
                    Text::new(format!("-{damage}")),
                    TextFont {
                        font_size: 15.0,
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 0.85, 0.2, 1.0)),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(screen_pos.x - 12.0),
                        top: Val::Px(screen_pos.y - 10.0),
                        ..default()
                    },
                ));
            }
            // Screen shake on significant hits.
            if damage >= 3 {
                shake.trigger(0.05 + damage as f32 * 0.004);
            }
        }
        cv.last_hp = current_hp;
    }
}

fn update_damage_flash(
    time: Res<Time>,
    mut flash_q: Query<(Entity, &mut DamageFlash, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    let flash_color = Color::srgb(1.0, 0.12, 0.08);
    for (entity, mut flash, mat_handle) in flash_q.iter_mut() {
        flash.timer.tick(time.delta());
        let t = flash.timer.fraction(); // 0 = just hit (full red), 1 = restored
        let current = lerp_color(flash_color, flash.original_color, t);
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            mat.base_color = current;
        }
        if flash.timer.just_finished() {
            if let Some(mat) = materials.get_mut(&mat_handle.0) {
                mat.base_color = flash.original_color;
            }
            commands.entity(entity).remove::<DamageFlash>();
        }
    }
}

fn update_floating_damage_text(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut FloatingDamageNumber, &mut Node, &mut TextColor)>,
) {
    let dt = time.delta_secs();
    for (entity, mut fdn, mut node, mut color) in q.iter_mut() {
        fdn.timer -= dt;
        if fdn.timer <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        fdn.screen_y -= FLOAT_SPEED * dt;
        node.top = Val::Px(fdn.screen_y);
        let alpha = (fdn.timer / FLOAT_DURATION).clamp(0.0, 1.0);
        color.0 = Color::srgba(1.0, 0.85, 0.2, alpha);
    }
}

fn despawn_floating_damage(mut commands: Commands, q: Query<Entity, With<FloatingDamageNumber>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

// ── Dissolve ──────────────────────────────────────────────────────────────────

fn update_dissolve(
    time: Res<Time>,
    mut q: Query<(Entity, &mut Dissolve, &mut Visibility)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    for (entity, mut dissolve, mut vis) in q.iter_mut() {
        dissolve.timer.tick(time.delta());
        let alpha = 1.0 - dissolve.timer.fraction();
        if let Some(mat) = materials.get_mut(&dissolve.mat_handle) {
            let base = mat.base_color.to_srgba();
            mat.base_color = Color::srgba(base.red, base.green, base.blue, alpha);
            mat.alpha_mode = AlphaMode::Blend;
        }
        *vis = Visibility::Inherited;
        if dissolve.timer.just_finished() {
            commands.entity(entity).despawn();
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
    time: Res<Time>,
    char_q: Query<(&CharacterVisual, &Transform)>,
    mut bg_q: Query<(&HealthBarBg, &mut Transform, &mut Visibility), Without<CharacterVisual>>,
    mut fill_q: Query<
        (
            &HealthBarFill,
            &mut Transform,
            &mut Visibility,
            &MeshMaterial3d<StandardMaterial>,
        ),
        (Without<CharacterVisual>, Without<HealthBarBg>),
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let world = &session.0.world;
    let active_entity = session.0.battle.as_ref().and_then(|b| b.current_actor());

    let char_positions: std::collections::HashMap<bevy::ecs::entity::Entity, Vec3> = char_q
        .iter()
        .map(|(cv, t)| (cv.game_entity, t.translation))
        .collect();

    let bar_rotation = Quat::from_rotation_y(HEALTH_BAR_ROTATION);
    let bar_dir = Vec3::new(1.0, 0.0, -1.0) * std::f32::consts::FRAC_1_SQRT_2;

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
        let kind = world
            .get::<Character>(game_entity)
            .map(|c| c.kind.clone())
            .unwrap_or(CharacterKind::Researcher);
        *vis = Visibility::Inherited;
        let bar_y = char_pos.y + char_height_for(&kind) * 0.5 + HEALTH_BAR_Y_ABOVE;
        transform.translation = Vec3::new(char_pos.x, bar_y, char_pos.z);
        transform.rotation = bar_rotation;
    }

    // Smooth gradient pulse for the active-turn unit (period ≈ 3 s).
    let pulse_t = (time.elapsed_secs() * 2.0).sin() * 0.5 + 0.5;

    for (fill, mut transform, mut vis, mat_handle) in &mut fill_q {
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
        let kind = world
            .get::<Character>(game_entity)
            .map(|c| c.kind.clone())
            .unwrap_or(CharacterKind::Researcher);
        let is_player = kind.is_player();
        *vis = Visibility::Inherited;
        let fraction = if health.max > 0 {
            (health.current as f32 / health.max as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let bar_y = char_pos.y + char_height_for(&kind) * 0.5 + HEALTH_BAR_Y_ABOVE;
        let center = Vec3::new(char_pos.x, bar_y, char_pos.z);
        // Offset slightly above bg to prevent z-fighting.
        let fill_center =
            center + bar_dir * (HEALTH_BAR_WIDTH * (fraction - 1.0) / 2.0) + Vec3::Y * 0.001;
        transform.translation = fill_center;
        transform.rotation = bar_rotation;
        transform.scale = Vec3::new(fraction.max(0.001), 1.0, 1.0);

        // Update fill color: smooth gradient pulse for active entity, base color otherwise.
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            let (base_color, bright_color) = if is_player {
                (Color::srgb(0.10, 0.85, 0.20), Color::srgb(0.40, 1.0, 0.55))
            } else {
                (Color::srgb(0.85, 0.15, 0.15), Color::srgb(1.0, 0.50, 0.50))
            };
            mat.base_color = if Some(game_entity) == active_entity {
                lerp_color(base_color, bright_color, pulse_t)
            } else {
                base_color
            };
        }
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
        let kind = world
            .get::<Character>(game_entity)
            .map(|c| c.kind.clone())
            .unwrap_or(CharacterKind::Researcher);
        let label_world_pos = Vec3::new(
            char_pos.x,
            char_pos.y + char_height_for(&kind) * 0.5 + HEALTH_BAR_Y_ABOVE + 0.18,
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
