use bevy::prelude::*;
use carbonthrone::game::GamePhase;

use super::{StateUiRoot, accent_text, panel_bg, text_font, white_text};
use crate::resources::GameSessionRes;
use crate::state::AppState;

pub struct DialogPlugin;

impl Plugin for DialogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Dialog), spawn_dialog_panel)
            .add_systems(OnExit(AppState::Dialog), despawn_dialog_panel)
            .add_systems(
                Update,
                (update_dialog_text, handle_choice_buttons)
                    .run_if(in_state(AppState::Dialog)),
            );
    }
}

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

            // Choices container.
            parent
                .spawn((
                    DialogChoicesContainer,
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        ..default()
                    },
                ))
                .with_children(|_| {
                    // Populated dynamically in update_dialog_text.
                });

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
                    parent.spawn((
                        Text::new("Continue"),
                        text_font(13.0),
                        white_text(),
                    ));
                });
        });
}

fn update_dialog_text(
    session: Res<GameSessionRes>,
    mut speaker_q: Query<&mut Text, (With<DialogSpeakerLabel>, Without<DialogTextLabel>)>,
    mut text_q: Query<&mut Text, (With<DialogTextLabel>, Without<DialogSpeakerLabel>)>,
    mut continue_q: Query<&mut Visibility, With<ContinueButton>>,
    choices_container_q: Query<Entity, With<DialogChoicesContainer>>,
    mut commands: Commands,
) {
    if !session.is_changed() {
        return;
    }
    let GamePhase::Exploration(state) = &session.0.phase else {
        return;
    };

    // Update speaker + text.
    if let Some((speaker, text)) = state.scene_lines.get(state.line_index) {
        if let Ok(mut t) = speaker_q.single_mut() {
            *t = Text::new(speaker.clone());
        }
        if let Ok(mut t) = text_q.single_mut() {
            *t = Text::new(text.clone());
        }
    }

    let at_choices = state.at_choice_screen();

    // Toggle continue button.
    if let Ok(mut vis) = continue_q.single_mut() {
        *vis = if at_choices {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }

    // Rebuild choice buttons.
    if let Ok(container_entity) = choices_container_q.single() {
        commands.entity(container_entity).despawn_related::<Children>();
        if at_choices {
            commands
                .entity(container_entity)
                .with_children(|parent| {
                    for (i, choice_text) in state.scene_choices.iter().enumerate() {
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
                                    Text::new(format!("> {}", choice_text)),
                                    text_font(13.0),
                                    white_text(),
                                ));
                            });
                    }
                });
        }
    }
}

fn handle_choice_buttons(
    mut session: ResMut<GameSessionRes>,
    choice_q: Query<(&ChoiceButton, &Interaction), Changed<Interaction>>,
    continue_q: Query<&Interaction, (With<ContinueButton>, Changed<Interaction>)>,
) {
    for (choice_btn, interaction) in &choice_q {
        if *interaction == Interaction::Pressed {
            if let GamePhase::Exploration(e) = &mut session.0.phase {
                e.choice_index = choice_btn.0;
            }
            session.0.select_choice();
        }
    }

    if let Ok(interaction) = continue_q.single() {
        if *interaction == Interaction::Pressed {
            session.0.advance_dialog();
        }
    }
}

fn despawn_dialog_panel(mut commands: Commands, q: Query<Entity, With<StateUiRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
