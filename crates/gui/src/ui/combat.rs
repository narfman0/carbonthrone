use bevy::{prelude::*, window::PrimaryWindow};
use carbonthrone::{
    ability::character_abilities,
    action_points::ActionPoints,
    character::Character,
    combat::{BattleOutcome, Turn},
    game::GamePhase,
    health::Health,
    player_input::PlayerActionChoice,
    position::Position,
    stats::Stats,
    terrain::{CoverLevel, Direction, LevelMap},
    turn::{MOVE_AP_COST, move_range_per_ap},
};

use super::{StateUiRoot, accent_text, panel_bg, text_font, white_text};
use crate::camera::IsometricCamera;
use crate::grid::world_to_grid;
use crate::resources::{
    GameSessionRes, PendingAbilityTarget, PendingPlayerChoices, SelectedChoiceIndex,
};
use crate::state::AppState;

pub struct CombatUiPlugin;

impl Plugin for CombatUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Battle), spawn_combat_ui)
            .add_systems(OnExit(AppState::Battle), despawn_combat_ui)
            .add_systems(
                Update,
                (
                    update_combatant_list,
                    update_action_panel,
                    handle_ability_buttons,
                    update_battle_outcome,
                    update_party_portraits,
                    handle_portrait_clicks,
                    update_info_panel,
                )
                    .chain()
                    .run_if(in_state(AppState::Battle)),
            );
    }
}

// ── Component markers ─────────────────────────────────────────────────────────

#[derive(Component)]
struct CombatantListPanel;

#[derive(Component)]
struct ActionPanel;

#[derive(Component)]
struct TurnLabel;

#[derive(Component)]
struct AbilityButton(usize);

#[derive(Component)]
struct OutcomePanel;

#[derive(Component)]
struct OutcomeLabel;

#[derive(Component)]
struct OutcomeContinueButton;

/// Portrait button linking back to its game world entity.
#[derive(Component, Clone, Copy)]
struct PortraitButton(bevy::ecs::entity::Entity);

#[derive(Component)]
struct PartyPortraitPanel;

#[derive(Component)]
struct InfoPanel;

#[derive(Component)]
struct InfoPanelText;

// ── Spawn ─────────────────────────────────────────────────────────────────────

fn spawn_combat_ui(mut commands: Commands) {
    // Combatant status — top left.
    commands
        .spawn((
            StateUiRoot,
            CombatantListPanel,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(8.0),
                left: Val::Px(8.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(3.0),
                width: Val::Px(280.0),
                ..default()
            },
            panel_bg(),
        ))
        .with_children(|parent| {
            parent.spawn((
                TurnLabel,
                Text::new("Player turn"),
                text_font(13.0),
                accent_text(),
            ));
        });

    // Action panel — bottom.
    commands.spawn((
        StateUiRoot,
        ActionPanel,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            min_height: Val::Px(80.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(10.0)),
            row_gap: Val::Px(4.0),
            ..default()
        },
        panel_bg(),
    ));

    // Party portrait panel — left side, vertically centred.
    commands.spawn((
        StateUiRoot,
        PartyPortraitPanel,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(8.0),
            top: Val::Percent(30.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            padding: UiRect::all(Val::Px(6.0)),
            width: Val::Px(110.0),
            ..default()
        },
        panel_bg(),
    ));

    // Info panel — right side, middle.
    commands
        .spawn((
            StateUiRoot,
            InfoPanel,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(8.0),
                top: Val::Percent(25.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                width: Val::Px(200.0),
                row_gap: Val::Px(3.0),
                ..default()
            },
            panel_bg(),
        ))
        .with_children(|parent| {
            parent.spawn((
                InfoPanelText,
                Text::new("Hover a tile or\nability for info"),
                text_font(11.0),
                white_text(),
            ));
        });

    // Outcome panel — hidden by default.
    commands
        .spawn((
            StateUiRoot,
            OutcomePanel,
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(30.0),
                left: Val::Percent(25.0),
                right: Val::Percent(25.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(20.0)),
                row_gap: Val::Px(12.0),
                ..default()
            },
            panel_bg(),
        ))
        .with_children(|parent| {
            parent.spawn((OutcomeLabel, Text::new(""), text_font(20.0), accent_text()));
            parent
                .spawn((
                    OutcomeContinueButton,
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(24.0), Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.4, 0.6)),
                ))
                .with_children(|parent| {
                    parent.spawn((Text::new("Continue"), text_font(14.0), white_text()));
                });
        });
}

// ── Update: combatant list ────────────────────────────────────────────────────

fn update_combatant_list(
    mut session: ResMut<GameSessionRes>,
    panel_q: Query<Entity, With<CombatantListPanel>>,
    mut turn_q: Query<&mut Text, With<TurnLabel>>,
    mut commands: Commands,
) {
    if !session.is_changed() {
        return;
    }

    let world = &mut session.0.world;
    let mut char_q = world.query::<(bevy::ecs::entity::Entity, &Character, &Health)>();
    let all_chars: Vec<_> = char_q
        .iter(world)
        .map(|(_, c, h)| (c.name.clone(), c.kind.is_player(), h.current, h.max))
        .collect();

    // Update turn label.
    if let Some(battle) = &session.0.battle {
        if let Ok(mut t) = turn_q.single_mut() {
            let label = match battle.turn {
                Turn::Player => "Player Turn",
                Turn::Enemy => "Enemy Turn",
            };
            *t = Text::new(label);
        }
    }

    // Rebuild combatant rows.
    let Ok(panel_entity) = panel_q.single() else {
        return;
    };
    // Remove old combatant rows (keep turn label).
    commands.entity(panel_entity).despawn_related::<Children>();
    commands.entity(panel_entity).with_children(|parent| {
        // Re-add turn label.
        parent.spawn((
            TurnLabel,
            Text::new(
                session
                    .0
                    .battle
                    .as_ref()
                    .map(|b| match b.turn {
                        Turn::Player => "Player Turn",
                        Turn::Enemy => "Enemy Turn",
                    })
                    .unwrap_or("—"),
            ),
            text_font(13.0),
            accent_text(),
        ));

        // Players section.
        parent.spawn((Text::new("── PLAYERS ──"), text_font(11.0), white_text()));
        for (name, is_player, hp, max_hp) in &all_chars {
            if !is_player {
                continue;
            }
            let bar = hp_bar(*hp, *max_hp);
            parent.spawn((
                Text::new(format!("{} {} {}/{}", name, bar, hp, max_hp)),
                text_font(11.0),
                if *hp > 0 {
                    white_text()
                } else {
                    TextColor(Color::srgb(0.5, 0.5, 0.5))
                },
            ));
        }

        // Enemies section.
        parent.spawn((Text::new("── ENEMIES ──"), text_font(11.0), white_text()));
        for (name, is_player, hp, max_hp) in &all_chars {
            if *is_player {
                continue;
            }
            let bar = hp_bar(*hp, *max_hp);
            parent.spawn((
                Text::new(format!("{} {} {}/{}", name, bar, hp, max_hp)),
                text_font(11.0),
                if *hp > 0 {
                    TextColor(Color::srgb(1.0, 0.5, 0.5))
                } else {
                    TextColor(Color::srgb(0.5, 0.5, 0.5))
                },
            ));
        }
    });
}

fn hp_bar(hp: i32, max_hp: i32) -> String {
    let filled = if max_hp > 0 {
        (hp * 8 / max_hp).clamp(0, 8) as usize
    } else {
        0
    };
    let empty = 8 - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

// ── Update: action panel ──────────────────────────────────────────────────────

fn update_action_panel(
    session: Res<GameSessionRes>,
    choices: Res<PendingPlayerChoices>,
    targeting: Res<PendingAbilityTarget>,
    action_panel_q: Query<Entity, With<ActionPanel>>,
    mut commands: Commands,
) {
    if !choices.is_changed() && !targeting.is_changed() {
        return;
    }
    let Ok(panel_entity) = action_panel_q.single() else {
        return;
    };
    commands.entity(panel_entity).despawn_related::<Children>();

    // Build actor header text.
    let header = match session.0.battle.as_ref() {
        None => "Waiting...".to_string(),
        Some(battle) => match battle.turn {
            Turn::Enemy => "Enemy turn".to_string(),
            Turn::Player => match battle.current_actor() {
                None => "Waiting for player turn".to_string(),
                Some(actor) => {
                    let world = &session.0.world;
                    let name = world
                        .get::<Character>(actor)
                        .map(|c| c.name.as_str())
                        .unwrap_or("?");
                    let ap = world
                        .get::<ActionPoints>(actor)
                        .map(|a| a.current)
                        .unwrap_or(0);
                    format!("► {name} — {ap} AP")
                }
            },
        },
    };

    if choices.choices.is_empty() {
        commands.entity(panel_entity).with_children(|parent| {
            parent.spawn((Text::new(header), text_font(13.0), accent_text()));
        });
        return;
    }

    commands.entity(panel_entity).with_children(|parent| {
        parent.spawn((Text::new(header), text_font(13.0), accent_text()));

        // Show targeting mode indicator.
        if let Some(ability_name) = targeting.0 {
            parent.spawn((
                Text::new(format!("⊕ Select target for: {ability_name}\n  (left-click enemy, right-click to cancel)")),
                text_font(11.0),
                TextColor(Color::srgb(1.0, 0.8, 0.1)),
            ));
            return;
        }

        // ── Abilities (one button per unique ability name, including unavailable) ──
        // Collect available abilities from active choices.
        let mut available_names: std::collections::HashSet<&'static str> =
            std::collections::HashSet::new();
        let mut ability_entries: Vec<(Option<usize>, &'static str, i32, bool)> = Vec::new();
        for (i, choice) in choices.choices.iter().enumerate() {
            if let PlayerActionChoice::UseAbility { ability, target, .. } = choice {
                if available_names.insert(ability.name) {
                    ability_entries.push((Some(i), ability.name, ability.ap_cost, target.is_some()));
                }
            }
        }
        // Also add abilities from the actor's full ability table that aren't yet shown.
        if let Some(battle) = session.0.battle.as_ref() {
            if let Some(actor) = battle.current_actor() {
                let world = &session.0.world;
                if let Some(ch) = world.get::<Character>(actor) {
                    let all_abilities = character_abilities(&ch.kind);
                    let mut seen: std::collections::HashSet<&'static str> =
                        available_names.iter().copied().collect();
                    for ab in &all_abilities {
                        if ab.level_required <= ch.level && seen.insert(ab.name) {
                            ability_entries.push((None, ab.name, ab.ap_cost, false));
                        }
                    }
                }
            }
        }
        if !ability_entries.is_empty() {
            parent.spawn((Text::new("─── ABILITIES"), text_font(10.0), white_text()));
            for (choice_idx, name, ap_cost, needs_target) in ability_entries {
                let available = choice_idx.is_some();
                let target_hint = if needs_target { " [click target]" } else { "" };
                let label = format!("{name} ({ap_cost}AP){target_hint}");
                let text_color = if available {
                    white_text()
                } else {
                    TextColor(Color::srgb(0.4, 0.4, 0.4))
                };
                if available {
                    let bg = Color::srgb(0.15, 0.25, 0.45);
                    parent
                        .spawn((
                            AbilityButton(choice_idx.unwrap()),
                            Button,
                            Node {
                                padding: UiRect::axes(Val::Px(12.0), Val::Px(5.0)),
                                margin: UiRect::bottom(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(bg),
                        ))
                        .with_children(|p| {
                            p.spawn((Text::new(label), text_font(12.0), text_color));
                        });
                } else {
                    // Non-interactive dimmed row for unavailable abilities.
                    parent
                        .spawn((
                            Node {
                                padding: UiRect::axes(Val::Px(12.0), Val::Px(5.0)),
                                margin: UiRect::bottom(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.10, 0.10, 0.16)),
                        ))
                        .with_children(|p| {
                            p.spawn((Text::new(label), text_font(12.0), text_color));
                        });
                }
            }
        }

        // ── Pass ──
        for (i, choice) in choices.choices.iter().enumerate() {
            if matches!(choice, PlayerActionChoice::Pass) {
                let label = choice.display();
                parent
                    .spawn((
                        AbilityButton(i),
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(12.0), Val::Px(5.0)),
                            margin: UiRect::bottom(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.25, 0.18, 0.18)),
                    ))
                    .with_children(|p| {
                        p.spawn((Text::new(label), text_font(12.0), white_text()));
                    });
            }
        }
    });
}

fn handle_ability_buttons(
    button_q: Query<(&AbilityButton, &Interaction), Changed<Interaction>>,
    mut selected: ResMut<SelectedChoiceIndex>,
    mut targeting: ResMut<PendingAbilityTarget>,
    choices: Res<PendingPlayerChoices>,
) {
    for (btn, interaction) in &button_q {
        if *interaction == Interaction::Pressed {
            if let Some(choice) = choices.choices.get(btn.0) {
                match choice {
                    PlayerActionChoice::UseAbility {
                        ability, target, ..
                    } => {
                        if target.is_some() {
                            // Targeted ability — enter targeting mode.
                            targeting.0 = Some(ability.name);
                        } else {
                            // Utility ability (self-targeted) — execute directly.
                            selected.0 = Some(btn.0);
                        }
                    }
                    _ => {
                        // Pass or move — execute directly.
                        selected.0 = Some(btn.0);
                    }
                }
            }
        }
    }
}

// ── Update: battle outcome ────────────────────────────────────────────────────

fn update_battle_outcome(
    mut outcome_panel_q: Query<&mut Visibility, With<OutcomePanel>>,
    mut outcome_label_q: Query<&mut Text, (With<OutcomeLabel>, Without<OutcomeContinueButton>)>,
    continue_q: Query<&Interaction, (With<OutcomeContinueButton>, Changed<Interaction>)>,
    mut session_res: ResMut<GameSessionRes>,
) {
    // Show outcome panel when battle is over.
    if session_res.is_changed() {
        if let Some(outcome) = session_res
            .0
            .last_event
            .as_ref()
            .and_then(|e| e.outcome.as_ref())
        {
            if let Ok(mut vis) = outcome_panel_q.single_mut() {
                *vis = Visibility::Inherited;
            }
            let msg = match outcome {
                BattleOutcome::PlayerVictory => "Victory!",
                BattleOutcome::PlayerDefeated => "Defeated...",
                BattleOutcome::Draw => "Draw",
            };
            if let Ok(mut t) = outcome_label_q.single_mut() {
                *t = Text::new(msg);
            }
        }
    }

    // Continue button: transition back to exploration.
    // Guard on still being in Battle phase so the button can't double-fire
    // on the frame after transition_to_exploration() already ran.
    if matches!(&session_res.0.phase, GamePhase::Battle(_)) {
        if let Ok(interaction) = continue_q.single() {
            if *interaction == Interaction::Pressed {
                session_res.0.transition_to_exploration();
            }
        }
    }
}

// ── Update: party portrait panel ─────────────────────────────────────────────

fn update_party_portraits(
    mut session: ResMut<GameSessionRes>,
    portrait_panel_q: Query<Entity, With<PartyPortraitPanel>>,
    mut commands: Commands,
) {
    if !session.is_changed() {
        return;
    }
    let Ok(panel_entity) = portrait_panel_q.single() else {
        return;
    };

    // Collect player data: (entity, name, hp, max_hp, ap, is_active).
    let active_entity = session.0.battle.as_ref().and_then(|b| b.current_actor());
    let queued: Vec<bevy::ecs::entity::Entity> = session
        .0
        .battle
        .as_ref()
        .map(|b| b.player_queue().iter().copied().collect())
        .unwrap_or_default();

    let world = &mut session.0.world;
    let mut char_q = world.query::<(
        bevy::ecs::entity::Entity,
        &Character,
        &Health,
        &ActionPoints,
    )>();
    let players: Vec<_> = char_q
        .iter(world)
        .filter(|(_, c, _, _)| c.kind.is_player())
        .map(|(e, c, h, ap)| (e, c.name.clone(), h.current, h.max, ap.current, ap.max))
        .collect();

    commands.entity(panel_entity).despawn_related::<Children>();
    commands.entity(panel_entity).with_children(|parent| {
        parent.spawn((Text::new("PARTY"), text_font(10.0), accent_text()));
        for (entity, name, hp, max_hp, ap, max_ap) in &players {
            let is_active = Some(*entity) == active_entity;
            let queued_idx = queued.iter().position(|&e| e == *entity);
            let bg = if is_active {
                Color::srgb(0.15, 0.40, 0.15) // bright green — acting now
            } else if queued_idx.is_some() {
                Color::srgb(0.10, 0.20, 0.10) // dim green — queued
            } else {
                Color::srgb(0.15, 0.15, 0.15) // grey — already acted
            };
            let label = format!("{}\nHP {}/{}\nAP {}/{}", name, hp, max_hp, ap, max_ap);
            parent
                .spawn((
                    PortraitButton(*entity),
                    Button,
                    Node {
                        padding: UiRect::all(Val::Px(6.0)),
                        margin: UiRect::bottom(Val::Px(2.0)),
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    BackgroundColor(bg),
                ))
                .with_children(|p| {
                    p.spawn((Text::new(label), text_font(10.0), white_text()));
                });
        }
    });
}

fn handle_portrait_clicks(
    button_q: Query<(&PortraitButton, &Interaction), Changed<Interaction>>,
    mut session: ResMut<GameSessionRes>,
    mut choices: ResMut<PendingPlayerChoices>,
) {
    for (btn, interaction) in &button_q {
        if *interaction == Interaction::Pressed {
            if let Some(battle) = session.0.battle.as_mut() {
                if battle.set_active_player(btn.0) {
                    choices.needs_refresh = true;
                }
            }
        }
    }
}

// ── Update: hover info panel ──────────────────────────────────────────────────

fn update_info_panel(
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<IsometricCamera>>,
    mut session: ResMut<GameSessionRes>,
    choices: Res<PendingPlayerChoices>,
    targeting: Res<PendingAbilityTarget>,
    ability_hover_q: Query<(&AbilityButton, &Interaction)>,
    mut info_text_q: Query<&mut Text, With<InfoPanelText>>,
) {
    let Ok(mut text) = info_text_q.single_mut() else {
        return;
    };

    // Priority 1: in targeting mode — show hit/damage for the hovered character.
    if let Some(ability_name) = targeting.0 {
        if let Some(info) =
            targeting_hover_info(&windows, &camera_q, &mut session, &choices, ability_name)
        {
            *text = Text::new(info);
            return;
        }
    }

    // Priority 2: hovered ability button (shows ability details).
    for (btn, interaction) in &ability_hover_q {
        if *interaction == Interaction::Hovered {
            if let Some(choice) = choices.choices.get(btn.0) {
                *text = Text::new(choice.display());
                return;
            }
        }
    }

    // Priority 3: tile under cursor.
    let info = cursor_tile_info(&windows, &camera_q, &mut session);
    *text = Text::new(info);
}

/// When in targeting mode, returns info about the ability vs. the hovered character.
fn targeting_hover_info(
    windows: &Query<&Window, With<PrimaryWindow>>,
    camera_q: &Query<(&Camera, &GlobalTransform), With<IsometricCamera>>,
    session: &mut ResMut<GameSessionRes>,
    choices: &Res<PendingPlayerChoices>,
    ability_name: &'static str,
) -> Option<String> {
    let Ok(window) = windows.single() else {
        return None;
    };
    let cursor = window.cursor_position()?;
    let Ok((cam, cam_transform)) = camera_q.single() else {
        return None;
    };
    let ray = cam.viewport_to_world(cam_transform, cursor).ok()?;
    if ray.direction.y.abs() < 1e-6 {
        return None;
    }
    let t = -ray.origin.y / ray.direction.y;
    if t < 0.0 {
        return None;
    }
    let (gx, gy) = world_to_grid(ray.origin + ray.direction * t);

    // Find a matching choice for this tile.
    let choice = choices.choices.iter().find(|c| {
        if let PlayerActionChoice::UseAbility {
            ability, target, ..
        } = c
        {
            if ability.name != ability_name {
                return false;
            }
            if let Some(target_entity) = target {
                let world = &session.0.world;
                if let Some(pos) = world.get::<Position>(*target_entity) {
                    return pos.x == gx && pos.y == gy;
                }
            }
        }
        false
    });

    let Some(PlayerActionChoice::UseAbility {
        ability: _,
        hit_chance,
        damage,
        cover,
        ..
    }) = choice
    else {
        return None;
    };

    let world = &session.0.world;
    // Get target character info.
    let target_entity = choices.choices.iter().find_map(|c| {
        if let PlayerActionChoice::UseAbility {
            ability: a,
            target: Some(t),
            ..
        } = c
        {
            if a.name == ability_name {
                let pos = world.get::<Position>(*t)?;
                if pos.x == gx && pos.y == gy {
                    return Some(*t);
                }
            }
        }
        None
    })?;

    let char = world.get::<carbonthrone::character::Character>(target_entity)?;
    let hp = world.get::<Health>(target_entity)?;
    let kind_str = format!("{:?}", char.kind);

    let hit_pct = hit_chance.map(|h| (h * 100.0).round() as i32).unwrap_or(90);
    let dmg_str = damage
        .map(|d| format!("{d}"))
        .unwrap_or_else(|| "?".to_string());
    let cover_str = match cover.unwrap_or(CoverLevel::None) {
        CoverLevel::None => "",
        CoverLevel::Partial => " (partial cover)",
        CoverLevel::Full => " (full cover)",
    };
    Some(format!(
        "► {ability_name}\nTarget: {kind_str} Lv.{}\nHP: {}/{}\n{}% hit, {dmg_str} dmg{}",
        char.level, hp.current, hp.max, hit_pct, cover_str
    ))
}

fn cursor_tile_info(
    windows: &Query<&Window, With<PrimaryWindow>>,
    camera_q: &Query<(&Camera, &GlobalTransform), With<IsometricCamera>>,
    session: &mut ResMut<GameSessionRes>,
) -> String {
    let Ok(window) = windows.single() else {
        return "Hover a tile or\nability for info".to_string();
    };
    let Some(cursor) = window.cursor_position() else {
        return "Hover a tile or\nability for info".to_string();
    };
    let Ok((cam, cam_transform)) = camera_q.single() else {
        return String::new();
    };
    let Ok(ray) = cam.viewport_to_world(cam_transform, cursor) else {
        return String::new();
    };
    if ray.direction.y.abs() < 1e-6 {
        return String::new();
    }
    let t = -ray.origin.y / ray.direction.y;
    if t < 0.0 {
        return String::new();
    }
    let hit = ray.origin + ray.direction * t;
    let (gx, gy) = world_to_grid(hit);

    let world = &mut session.0.world;

    // Check for a character at this position.
    let mut char_q = world.query::<(&Character, &Health, &ActionPoints, &Stats, &Position)>();
    let char_data: Vec<_> = char_q
        .iter(world)
        .filter(|(_, _, _, _, pos)| pos.x == gx && pos.y == gy)
        .map(|(ch, hp, ap, stats, _)| {
            (
                ch.name.clone(),
                format!("{:?}", ch.kind),
                ch.level,
                ch.kind.is_player(),
                hp.current,
                hp.max,
                ap.current,
                ap.max,
                stats.attack,
                stats.defense,
            )
        })
        .collect();
    if let Some((name, kind_str, level, is_player, hp, max_hp, ap, max_ap, atk, def)) =
        char_data.into_iter().next()
    {
        let side = if is_player { "Player" } else { "Enemy" };
        return format!(
            "{kind_str}  Lv.{level}\n{name} ({side})\nHP: {hp}/{max_hp}\nAP: {ap}/{max_ap}\nATK: {atk}  DEF: {def}"
        );
    }

    // No character — show terrain info.
    let Some(map) = world.get_resource::<LevelMap>() else {
        return format!("({}, {})", gx, gy);
    };
    if gx < 0 || gy < 0 || gx >= map.cols as i32 || gy >= map.rows as i32 {
        return String::new();
    }

    let tile = map.get(gx, gy);
    let tile_name = match tile {
        carbonthrone::terrain::Tile::Open => "Open ground",
        carbonthrone::terrain::Tile::Obstacle => "Obstacle",
        carbonthrone::terrain::Tile::Door => "Door",
    };

    let cover_n = map.get_cover(gx, gy, Direction::North);
    let cover_s = map.get_cover(gx, gy, Direction::South);
    let cover_e = map.get_cover(gx, gy, Direction::East);
    let cover_w = map.get_cover(gx, gy, Direction::West);
    let best_cover = [cover_n, cover_s, cover_e, cover_w]
        .iter()
        .copied()
        .max()
        .unwrap_or(CoverLevel::None);
    let cover_label = match best_cover {
        CoverLevel::None => "No cover",
        CoverLevel::Partial => "Partial cover",
        CoverLevel::Full => "Full cover",
    };

    // Show movement cost for passable tiles.
    let move_info = move_cost_for_tile(world, gx, gy);

    if let Some((ap_cost, can_afford)) = move_info {
        let afford_str = if can_afford {
            "Right-click to move"
        } else {
            "Not enough AP"
        };
        format!(
            "{}\n{}\nMove cost: {} AP\n{}",
            tile_name, cover_label, ap_cost, afford_str
        )
    } else {
        format!("{}\n{}\n({}, {})", tile_name, cover_label, gx, gy)
    }
}

/// Returns `Some((ap_cost, can_afford))` if the active player can move to `(gx, gy)`,
/// or `None` if the tile is impassable or there is no active player.
fn move_cost_for_tile(
    world: &mut bevy::ecs::world::World,
    gx: i32,
    gy: i32,
) -> Option<(i32, bool)> {
    let map = world.get_resource::<LevelMap>()?;
    if !map.is_passable(gx, gy) {
        return None;
    }

    // Find active player actor via BattleStep (stored in GameSession.battle).
    // We access the actor through ActionPoints/Stats/Position query.
    // The active player is whichever player character currently has a non-exhausted turn.
    // We identify them by checking all player characters and picking the one whose entity
    // is the "current_actor" — but we can't access BattleStep from here easily.
    // Instead, find the player character with the most AP as a proxy.
    let mut actor_data: Vec<(Position, i32, i32)> = {
        let mut q = world.query::<(
            &carbonthrone::character::Character,
            &Position,
            &ActionPoints,
            &Stats,
        )>();
        q.iter(world)
            .filter(|(c, _, _, _)| c.kind.is_player())
            .map(|(_, pos, ap, stats)| (*pos, ap.current, stats.speed))
            .collect()
    };
    // Pick the player with highest AP (most likely to be the active one).
    actor_data.sort_by(|a, b| b.1.cmp(&a.1));
    let (actor_pos, actor_ap, actor_speed) = actor_data.into_iter().next()?;

    if actor_ap == 0 {
        return None;
    }

    let range = move_range_per_ap(actor_speed);
    let dist = (gx - actor_pos.x).abs() + (gy - actor_pos.y).abs();
    if dist == 0 {
        return None;
    }
    let ap_cost = MOVE_AP_COST * ((dist + range - 1) / range.max(1));
    let max_dist = actor_ap * range;
    if dist > max_dist {
        return None; // Out of range
    }
    Some((ap_cost, actor_ap >= ap_cost))
}

// ── Cleanup ───────────────────────────────────────────────────────────────────

fn despawn_combat_ui(mut commands: Commands, q: Query<Entity, With<StateUiRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
