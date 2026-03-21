use bevy::prelude::*;
use carbonthrone::{
    ability::{AbilityKind, available_abilities, character_abilities},
    character::{Character, CharacterKind},
    game::GamePhase,
    health::Health,
    stats::Stats,
};

use super::{accent_text, panel_bg, text_font, white_text};
use crate::resources::GameSessionRes;
use crate::state::AppState;
use crate::ui::terminal::terminal_closed;

pub struct CharacterDetailPlugin;

impl Plugin for CharacterDetailPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CharacterDetailOpen>()
            .add_systems(OnExit(AppState::Exploration), cleanup_detail)
            .add_systems(OnExit(AppState::Battle), cleanup_detail)
            .add_systems(
                Update,
                (
                    toggle_character_detail.run_if(terminal_closed),
                    update_character_detail,
                )
                    .run_if(in_state(AppState::Exploration).or(in_state(AppState::Battle))),
            );
    }
}

/// Party index of the character being shown. `None` = panel closed.
#[derive(Resource, Default)]
pub struct CharacterDetailOpen(pub Option<usize>);

#[derive(Component)]
struct CharDetailRoot;

fn cleanup_detail(
    mut detail: ResMut<CharacterDetailOpen>,
    q: Query<Entity, With<CharDetailRoot>>,
    mut commands: Commands,
) {
    detail.0 = None;
    for e in &q {
        commands.entity(e).despawn();
    }
}

fn toggle_character_detail(
    keys: Res<ButtonInput<KeyCode>>,
    mut detail: ResMut<CharacterDetailOpen>,
    session: Res<GameSessionRes>,
) {
    let total = party_len(&session);
    if total == 0 {
        return;
    }

    if keys.just_pressed(KeyCode::KeyC) {
        detail.0 = match detail.0 {
            None => Some(0),
            Some(_) => None,
        };
        return;
    }

    if detail.0.is_some() {
        if keys.just_pressed(KeyCode::Comma) {
            if let Some(i) = detail.0 {
                detail.0 = Some(i.saturating_sub(1));
            }
        } else if keys.just_pressed(KeyCode::Period) {
            if let Some(i) = detail.0 {
                detail.0 = Some((i + 1).min(total - 1));
            }
        }
    }
}

fn update_character_detail(
    detail: Res<CharacterDetailOpen>,
    session: Res<GameSessionRes>,
    detail_root_q: Query<Entity, With<CharDetailRoot>>,
    mut commands: Commands,
) {
    let needs_rebuild = detail.is_changed() || (session.is_changed() && detail.0.is_some());
    if !needs_rebuild {
        return;
    }

    for e in &detail_root_q {
        commands.entity(e).despawn();
    }

    let Some(idx) = detail.0 else {
        return;
    };

    let Some(info) = collect_party_info(&session, idx) else {
        return;
    };

    spawn_panel(&mut commands, &info, idx, party_len(&session));
}

// ── Data helpers ───────────────────────────────────────────────────────────────

struct CharInfo {
    name: String,
    kind: CharacterKind,
    level: u32,
    hp: i32,
    max_hp: i32,
    attack: i32,
    defense: i32,
    speed: i32,
}

fn party_len(session: &GameSessionRes) -> usize {
    match &session.0.phase {
        GamePhase::Exploration(s) | GamePhase::Battle(s) => 1 + s.companion_entities.len(),
        _ => 0,
    }
}

fn collect_party_info(session: &GameSessionRes, idx: usize) -> Option<CharInfo> {
    let state = match &session.0.phase {
        GamePhase::Exploration(s) | GamePhase::Battle(s) => s,
        _ => return None,
    };

    let entity = if idx == 0 {
        state.player_entity
    } else {
        *state.companion_entities.get(idx - 1)?
    };

    let world = &session.0.world;
    let character = world.get::<Character>(entity)?;
    let health = world.get::<Health>(entity)?;
    let stats = world.get::<Stats>(entity)?;

    Some(CharInfo {
        name: character.name.clone(),
        kind: character.kind.clone(),
        level: character.level,
        hp: health.current,
        max_hp: health.max,
        attack: stats.attack,
        defense: stats.defense,
        speed: stats.speed,
    })
}

// ── Panel layout ───────────────────────────────────────────────────────────────

fn spawn_panel(commands: &mut Commands, info: &CharInfo, idx: usize, total: usize) {
    let all_abilities = character_abilities(&info.kind);
    let unlocked: std::collections::HashSet<&'static str> =
        available_abilities(&info.kind, info.level)
            .into_iter()
            .map(|a| a.name)
            .collect();

    commands
        .spawn((
            CharDetailRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.70)),
            GlobalZIndex(50),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(20.0)),
                        row_gap: Val::Px(6.0),
                        min_width: Val::Px(400.0),
                        max_width: Val::Px(520.0),
                        ..default()
                    },
                    panel_bg(),
                ))
                .with_children(|p| {
                    spawn_header(p, info);
                    spawn_stats_row(p, info);
                    spawn_abilities(p, &all_abilities, &unlocked, info.level);
                    spawn_footer(p, idx, total);
                });
        });
}

fn spawn_header(parent: &mut ChildSpawnerCommands, info: &CharInfo) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Baseline,
            margin: UiRect::bottom(Val::Px(4.0)),
            ..default()
        })
        .with_children(|p| {
            p.spawn((Text::new(info.name.clone()), text_font(20.0), accent_text()));
            p.spawn((
                Text::new(format!("Level {}", info.level)),
                text_font(14.0),
                white_text(),
            ));
        });
}

fn spawn_stats_row(parent: &mut ChildSpawnerCommands, info: &CharInfo) {
    let hp_bar = {
        let filled = if info.max_hp > 0 {
            (info.hp * 10 / info.max_hp).clamp(0, 10) as usize
        } else {
            0
        };
        format!("[{}{}]", "█".repeat(filled), "░".repeat(10 - filled))
    };

    parent.spawn((
        Text::new(format!(
            "HP {}  {}/{}    ATK {}    DEF {}    SPD {}",
            hp_bar, info.hp, info.max_hp, info.attack, info.defense, info.speed
        )),
        text_font(12.0),
        TextColor(Color::srgb(0.80, 0.85, 0.92)),
    ));

    // Divider
    parent.spawn((
        Text::new("─────────────────────────────────────"),
        text_font(10.0),
        TextColor(Color::srgb(0.25, 0.28, 0.35)),
    ));
}

fn spawn_abilities(
    parent: &mut ChildSpawnerCommands,
    all_abilities: &[carbonthrone::ability::Ability],
    unlocked: &std::collections::HashSet<&'static str>,
    _level: u32,
) {
    parent.spawn((Text::new("ABILITIES"), text_font(11.0), accent_text()));

    for ability in all_abilities {
        let is_unlocked = unlocked.contains(ability.name);

        let lock = if is_unlocked { "✓" } else { "🔒" };
        let lock_suffix = if !is_unlocked {
            format!("  (unlocks lv {})", ability.level_required)
        } else {
            String::new()
        };
        let flux_str = if ability.flux_generation > 0 {
            format!("  +{} flux", ability.flux_generation)
        } else {
            String::new()
        };
        let kind_label = match ability.kind {
            AbilityKind::Melee => "melee",
            AbilityKind::Ranged => "ranged",
            AbilityKind::RangedAny => "any range",
            AbilityKind::RangedAlly => "ally",
            AbilityKind::Utility => "utility",
        };

        let header_line = format!(
            "{}  {}{}   {} AP  [{}]{}",
            lock, ability.name, lock_suffix, ability.ap_cost, kind_label, flux_str
        );
        let header_color = if is_unlocked {
            Color::WHITE
        } else {
            Color::srgb(0.42, 0.42, 0.50)
        };

        parent.spawn((
            Text::new(header_line),
            text_font(11.5),
            TextColor(header_color),
        ));

        if is_unlocked {
            parent.spawn((
                Text::new(format!("    {}", ability.description)),
                text_font(10.0),
                TextColor(Color::srgb(0.62, 0.68, 0.75)),
            ));
        }
    }
}

fn spawn_footer(parent: &mut ChildSpawnerCommands, idx: usize, total: usize) {
    // Divider
    parent.spawn((
        Text::new("─────────────────────────────────────"),
        text_font(10.0),
        TextColor(Color::srgb(0.25, 0.28, 0.35)),
    ));

    let nav = if total > 1 {
        format!("{}/{}  [,] prev  [.] next    [C] close", idx + 1, total)
    } else {
        "[C] close".to_string()
    };

    parent.spawn((
        Text::new(nav),
        text_font(10.0),
        TextColor(Color::srgb(0.40, 0.45, 0.52)),
    ));
}
