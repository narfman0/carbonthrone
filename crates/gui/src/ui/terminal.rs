use bevy::{
    input::keyboard::{Key, KeyboardInput},
    prelude::*,
};
use carbonthrone::{
    combat::{BattleOutcome, check_outcome},
    console::{execute_command, parse_command},
};

use crate::{
    resources::{GameSessionRes, TerminalState},
    state::AppState,
};

use super::{panel_bg, text_font, white_text};

pub struct TerminalPlugin;

impl Plugin for TerminalPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_terminal_toggle,
                handle_terminal_keyboard.run_if(terminal_open),
                update_terminal_ui.run_if(terminal_open),
            )
                .chain(),
        )
        .add_systems(
            Update,
            spawn_terminal_panel.run_if(resource_changed::<TerminalState>),
        );
    }
}

// ── Condition helpers ─────────────────────────────────────────────────────────

pub fn terminal_open(state: Res<TerminalState>) -> bool {
    state.open
}

pub fn terminal_closed(state: Res<TerminalState>) -> bool {
    !state.open
}

// ── Marker components ─────────────────────────────────────────────────────────

#[derive(Component)]
struct TerminalPanel;

#[derive(Component)]
struct TerminalOutputText;

#[derive(Component)]
struct TerminalInputText;

// ── Systems ───────────────────────────────────────────────────────────────────

fn handle_terminal_toggle(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<TerminalState>) {
    if keys.just_pressed(KeyCode::Backquote) {
        state.open = !state.open;
    }
}

fn handle_terminal_keyboard(
    mut key_events: MessageReader<KeyboardInput>,
    mut state: ResMut<TerminalState>,
    mut session: ResMut<GameSessionRes>,
    app_state: Res<State<AppState>>,
) {
    for ev in key_events.read() {
        if !ev.state.is_pressed() {
            continue;
        }
        match &ev.logical_key {
            Key::Escape => {
                state.open = false;
            }
            Key::Enter => {
                let input = state.input.clone();
                let cmd = parse_command(&input);
                let output = execute_command(cmd, &mut session.0);
                state.last_output = output;
                state.input.clear();

                // Check if battle ended due to command (e.g. kill).
                if *app_state.get() == AppState::Battle {
                    if let Some(outcome) = check_outcome(&mut session.0.world) {
                        session.0.last_event = Some(carbonthrone::combat::TurnEvent {
                            actor: None,
                            turn: carbonthrone::combat::Turn::Player,
                            actions: vec![],
                            outcome: Some(outcome.clone()),
                        });
                        if outcome == BattleOutcome::PlayerVictory {
                            state.open = false;
                        }
                    }
                }
            }
            Key::Backspace => {
                state.input.pop();
            }
            _ => {
                if let Some(text) = &ev.text {
                    let filtered: String = text
                        .chars()
                        .filter(|&c| !c.is_ascii_control() && c != '`')
                        .collect();
                    state.input.push_str(&filtered);
                }
            }
        }
    }
}

fn update_terminal_ui(
    state: Res<TerminalState>,
    mut output_q: Query<&mut Text, (With<TerminalOutputText>, Without<TerminalInputText>)>,
    mut input_q: Query<&mut Text, (With<TerminalInputText>, Without<TerminalOutputText>)>,
) {
    if !state.is_changed() {
        return;
    }
    for mut text in output_q.iter_mut() {
        *text = Text::new(format!("> {}", state.last_output));
    }
    for mut text in input_q.iter_mut() {
        *text = Text::new(format!("> {}█", state.input));
    }
}

fn spawn_terminal_panel(
    mut commands: Commands,
    state: Res<TerminalState>,
    panel_q: Query<Entity, With<TerminalPanel>>,
) {
    // Always despawn existing panel first.
    for entity in panel_q.iter() {
        commands.entity(entity).despawn();
    }

    if !state.open {
        return;
    }

    commands
        .spawn((
            TerminalPanel,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                padding: UiRect::all(Val::Px(12.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            panel_bg(),
            ZIndex(100),
        ))
        .with_children(|parent| {
            // Output line
            parent.spawn((
                TerminalOutputText,
                Text::new(format!("> {}", state.last_output)),
                text_font(16.0),
                white_text(),
            ));
            // Input line
            parent.spawn((
                TerminalInputText,
                Text::new(format!("> {}█", state.input)),
                text_font(16.0),
                white_text(),
            ));
        });
}
