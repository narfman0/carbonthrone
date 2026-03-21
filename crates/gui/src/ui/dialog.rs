use bevy::prelude::*;
use bevy_yarnspinner::prelude::*;
use carbonthrone::game::GamePhase;

use super::{StateUiRoot, accent_text, panel_bg, text_font, white_text};
use crate::dialog_runner::{CurrentDialogLine, CurrentDialogOptions, GameDialogueRunner};
use crate::resources::GameSessionRes;
use crate::state::AppState;

pub struct DialogPlugin;

impl Plugin for DialogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Dialog), spawn_dialog_panel)
            .add_systems(
                OnExit(AppState::Dialog),
                (despawn_dialog_panel, reset_dialog_resources),
            )
            .add_systems(
                Update,
                (update_dialog_text, handle_dialog_input).run_if(in_state(AppState::Dialog)),
            );
    }
}

#[derive(Component)]
struct DialogZoneLabel;

#[derive(Component)]
struct DialogSpeakerLabel;

#[derive(Component)]
struct DialogTextLabel;

#[derive(Component)]
struct DialogChoicesContainer;

#[derive(Component)]
struct ChoiceButton(usize);

#[derive(Component)]
struct ContinueButton;

fn spawn_dialog_panel(mut commands: Commands) {
    commands
        .spawn((
            StateUiRoot,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                height: Val::Percent(32.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(6.0),
                ..default()
            },
            panel_bg(),
        ))
        .with_children(|parent| {
            // Zone label (top-right of dialog panel).
            parent.spawn((
                DialogZoneLabel,
                Text::new(""),
                text_font(11.0),
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
            ));

            // Speaker name.
            parent.spawn((
                DialogSpeakerLabel,
                Text::new(""),
                text_font(14.0),
                accent_text(),
            ));

            // Dialog text.
            parent.spawn((
                DialogTextLabel,
                Text::new(""),
                text_font(13.0),
                white_text(),
            ));

            // Spacer — pushes buttons to the bottom of the panel.
            parent.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });

            // Bottom button area: choices or continue, always at the same position.
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    align_self: AlignSelf::FlexEnd,
                    ..default()
                })
                .with_children(|parent| {
                    // Choices container.
                    parent.spawn((
                        DialogChoicesContainer,
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(4.0),
                            ..default()
                        },
                    ));

                    // Continue button (shown when no choices).
                    parent
                        .spawn((
                            ContinueButton,
                            Button,
                            Node {
                                padding: UiRect::axes(Val::Px(16.0), Val::Px(6.0)),
                                align_self: AlignSelf::FlexEnd,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.2, 0.4, 0.6)),
                        ))
                        .with_children(|parent| {
                            parent
                                .spawn((Text::new("Continue"), text_font(13.0), white_text()));
                        });
                });
        });
}

fn update_dialog_text(
    session: Res<GameSessionRes>,
    line_res: Res<CurrentDialogLine>,
    opts_res: Res<CurrentDialogOptions>,
    mut zone_q: Query<
        &mut Text,
        (
            With<DialogZoneLabel>,
            Without<DialogSpeakerLabel>,
            Without<DialogTextLabel>,
        ),
    >,
    mut speaker_q: Query<
        &mut Text,
        (
            With<DialogSpeakerLabel>,
            Without<DialogZoneLabel>,
            Without<DialogTextLabel>,
        ),
    >,
    mut text_q: Query<
        &mut Text,
        (
            With<DialogTextLabel>,
            Without<DialogZoneLabel>,
            Without<DialogSpeakerLabel>,
        ),
    >,
    mut continue_q: Query<&mut Visibility, With<ContinueButton>>,
    choices_container_q: Query<Entity, With<DialogChoicesContainer>>,
    mut commands: Commands,
) {
    // Update zone label.
    if let GamePhase::Exploration(state) = &session.0.phase {
        if let Ok(mut t) = zone_q.single_mut() {
            *t = Text::new(format!("[{}]", state.zone.kind.display_name()));
        }
    }

    // Update speaker + text from resource.
    if let Ok(mut t) = speaker_q.single_mut() {
        *t = Text::new(line_res.speaker.clone());
    }
    if let Ok(mut t) = text_q.single_mut() {
        *t = Text::new(line_res.text.clone());
    }

    let showing_choices = opts_res.waiting;

    // Toggle continue button.
    if let Ok(mut vis) = continue_q.single_mut() {
        *vis = if showing_choices || !line_res.waiting {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }

    // Rebuild choice buttons.
    if let Ok(container_entity) = choices_container_q.single() {
        commands
            .entity(container_entity)
            .despawn_related::<Children>();
        if showing_choices {
            commands.entity(container_entity).with_children(|parent| {
                for (i, (_id, choice_text)) in opts_res.options.iter().enumerate() {
                    parent
                        .spawn((
                            ChoiceButton(i),
                            Button,
                            Node {
                                padding: UiRect::axes(Val::Px(12.0), Val::Px(5.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.15, 0.30, 0.50)),
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new(format!("> {choice_text}")),
                                text_font(13.0),
                                white_text(),
                            ));
                        });
                }
            });
        }
    }
}

fn handle_dialog_input(
    choice_q: Query<(&ChoiceButton, &Interaction), Changed<Interaction>>,
    continue_q: Query<&Interaction, (With<ContinueButton>, Changed<Interaction>)>,
    mut runner_q: Query<&mut DialogueRunner, With<GameDialogueRunner>>,
    mut opts_res: ResMut<CurrentDialogOptions>,
    mut line_res: ResMut<CurrentDialogLine>,
    mut session: ResMut<GameSessionRes>,
) {
    for (choice_btn, interaction) in &choice_q {
        if *interaction == Interaction::Pressed {
            let id = opts_res.options.get(choice_btn.0).map(|(id, _)| *id);
            if let Some(id) = id {
                if let Ok(mut runner) = runner_q.single_mut() {
                    if runner.is_running() {
                        let _ = runner.select_option(id);
                    }
                }
                opts_res.waiting = false;
                opts_res.options.clear();
            }
        }
    }

    if let Ok(interaction) = continue_q.single() {
        if *interaction == Interaction::Pressed {
            if line_res.is_fallback {
                // Dismiss the fallback line cleanly.
                line_res.is_fallback = false;
                line_res.waiting = false;
                session.0.end_dialog(vec![]);
            } else if let Ok(mut runner) = runner_q.single_mut() {
                if runner.is_running() && !runner.is_waiting_for_option_selection() {
                    runner.continue_in_next_update();
                    line_res.waiting = false;
                }
            }
        }
    }
}

fn despawn_dialog_panel(mut commands: Commands, q: Query<Entity, With<StateUiRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

fn reset_dialog_resources(
    mut line_res: ResMut<CurrentDialogLine>,
    mut opts_res: ResMut<CurrentDialogOptions>,
) {
    *line_res = CurrentDialogLine::default();
    *opts_res = CurrentDialogOptions::default();
}
