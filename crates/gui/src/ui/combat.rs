use bevy::prelude::*;
use carbonthrone::{
    character::Character,
    combat::{BattleOutcome, Turn},
    health::Health,
};

use super::{StateUiRoot, accent_text, panel_bg, text_font, white_text};
use crate::resources::{GameSessionRes, PendingPlayerChoices, SelectedChoiceIndex};
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
                )
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
    choices: Res<PendingPlayerChoices>,
    action_panel_q: Query<Entity, With<ActionPanel>>,
    mut commands: Commands,
) {
    if !choices.is_changed() {
        return;
    }
    let Ok(panel_entity) = action_panel_q.single() else {
        return;
    };
    commands.entity(panel_entity).despawn_related::<Children>();

    if choices.choices.is_empty() {
        commands.entity(panel_entity).with_children(|parent| {
            parent.spawn((
                Text::new("Waiting for player turn..."),
                text_font(12.0),
                white_text(),
            ));
        });
        return;
    }

    commands.entity(panel_entity).with_children(|parent| {
        parent.spawn((Text::new("Choose action:"), text_font(12.0), accent_text()));
        for (i, choice) in choices.choices.iter().enumerate() {
            parent
                .spawn((
                    AbilityButton(i),
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(12.0), Val::Px(5.0)),
                        margin: UiRect::bottom(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.25, 0.45)),
                ))
                .with_children(|parent| {
                    parent.spawn((Text::new(choice.display()), text_font(12.0), white_text()));
                });
        }
    });
}

fn handle_ability_buttons(
    button_q: Query<(&AbilityButton, &Interaction), Changed<Interaction>>,
    mut selected: ResMut<SelectedChoiceIndex>,
) {
    for (btn, interaction) in &button_q {
        if *interaction == Interaction::Pressed {
            selected.0 = Some(btn.0);
        }
    }
}

// ── Update: battle outcome ────────────────────────────────────────────────────

fn update_battle_outcome(
    session: Res<GameSessionRes>,
    mut outcome_panel_q: Query<&mut Visibility, With<OutcomePanel>>,
    mut outcome_label_q: Query<&mut Text, (With<OutcomeLabel>, Without<OutcomeContinueButton>)>,
    continue_q: Query<&Interaction, (With<OutcomeContinueButton>, Changed<Interaction>)>,
    mut session_res: ResMut<GameSessionRes>,
) {
    // Show outcome panel when battle is over.
    if session.is_changed() {
        if let Some(outcome) = session
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
    if let Ok(interaction) = continue_q.single() {
        if *interaction == Interaction::Pressed {
            session_res.0.transition_to_exploration();
        }
    }
}

// ── Cleanup ───────────────────────────────────────────────────────────────────

fn despawn_combat_ui(mut commands: Commands, q: Query<Entity, With<StateUiRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
