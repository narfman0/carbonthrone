use bevy::prelude::*;
use carbonthrone::{
    character::{Character, CharacterKind},
    game::GamePhase,
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
}

/// Marker for NPC visual entities (NPCs aren't ECS entities in session.world).
#[derive(Component, Clone)]
pub struct NpcVisual {
    pub npc_index: usize,
}

pub struct CharacterVisualsPlugin;

impl Plugin for CharacterVisualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Exploration), spawn_exploration_chars)
            .add_systems(OnExit(AppState::Exploration), despawn_char_visuals)
            .add_systems(
                Update,
                sync_exploration_chars.run_if(in_state(AppState::Exploration)),
            )
            .add_systems(OnEnter(AppState::Battle), spawn_battle_chars)
            .add_systems(OnExit(AppState::Battle), despawn_char_visuals)
            .add_systems(
                Update,
                sync_battle_chars.run_if(in_state(AppState::Battle)),
            );
    }
}

// ── Color coding ─────────────────────────────────────────────────────────────

pub fn character_color(kind: &CharacterKind) -> Color {
    match kind {
        // Player characters — blue
        CharacterKind::Researcher
        | CharacterKind::Orin
        | CharacterKind::Doss
        | CharacterKind::Kaleo => Color::srgb(0.20, 0.60, 1.00),
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
        CharacterVisual { game_entity },
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Transform::from_translation(world_pos),
        GlobalTransform::default(),
    ));
}

// ── Exploration: sync positions ───────────────────────────────────────────────

fn sync_exploration_chars(
    session: Res<GameSessionRes>,
    mut char_q: Query<(&CharacterVisual, &mut Transform)>,
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
    let mut q = world.query::<(
        bevy::ecs::entity::Entity,
        &Character,
        &Position,
        &Health,
    )>();
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
    mut char_q: Query<(&CharacterVisual, &mut Transform, &mut Visibility)>,
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
