use bevy::prelude::*;
use carbonthrone::{game::GameSession, save::load_game};
use rand::SeedableRng;

use super::{StateUiRoot, accent_text, panel_bg, text_font};
use crate::{resources::GameSessionRes, state::AppState};

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::MainMenu), spawn_main_menu)
            .add_systems(OnExit(AppState::MainMenu), despawn_main_menu)
            .add_systems(
                Update,
                (handle_menu_buttons,).run_if(in_state(AppState::MainMenu)),
            );
    }
}

#[derive(Component)]
struct NewGameButton;

#[derive(Component)]
struct LoadGameButton;

#[derive(Component)]
struct ExitButton;

fn spawn_main_menu(mut commands: Commands) {
    let save_exists = load_game().is_ok();

    commands
        .spawn((
            StateUiRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
            panel_bg(),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("C A R B O N T H R O N E"),
                text_font(42.0),
                accent_text(),
            ));

            parent.spawn(Node {
                height: Val::Px(32.0),
                ..default()
            });

            // New Game
            parent
                .spawn((
                    NewGameButton,
                    Button,
                    Node {
                        width: Val::Px(220.0),
                        height: Val::Px(48.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.25, 0.40)),
                ))
                .with_children(|btn| {
                    btn.spawn((Text::new("New Game"), text_font(20.0), TextColor(Color::WHITE)));
                });

            // Load Game
            let load_bg = if save_exists {
                Color::srgb(0.15, 0.25, 0.40)
            } else {
                Color::srgb(0.12, 0.12, 0.15)
            };
            let load_text_color = if save_exists {
                Color::WHITE
            } else {
                Color::srgb(0.4, 0.4, 0.4)
            };
            parent
                .spawn((
                    LoadGameButton,
                    Button,
                    Node {
                        width: Val::Px(220.0),
                        height: Val::Px(48.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(load_bg),
                ))
                .with_children(|btn| {
                    btn.spawn((Text::new("Load Game"), text_font(20.0), TextColor(load_text_color)));
                });

            // Exit
            parent
                .spawn((
                    ExitButton,
                    Button,
                    Node {
                        width: Val::Px(220.0),
                        height: Val::Px(48.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.25, 0.40)),
                ))
                .with_children(|btn| {
                    btn.spawn((Text::new("Exit"), text_font(20.0), TextColor(Color::WHITE)));
                });
        });
}

fn despawn_main_menu(mut commands: Commands, roots: Query<Entity, With<StateUiRoot>>) {
    for e in &roots {
        commands.entity(e).despawn();
    }
}

fn handle_menu_buttons(
    new_game_q: Query<&Interaction, (With<NewGameButton>, Changed<Interaction>)>,
    load_game_q: Query<&Interaction, (With<LoadGameButton>, Changed<Interaction>)>,
    exit_q: Query<&Interaction, (With<ExitButton>, Changed<Interaction>)>,
    mut session: ResMut<GameSessionRes>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &new_game_q {
        if *interaction == Interaction::Pressed {
            session.0 = GameSession::new();
            next_state.set(AppState::Exploration);
        }
    }

    for interaction in &load_game_q {
        if *interaction == Interaction::Pressed {
            if let Ok(data) = load_game() {
                let mut rng = rand::rngs::StdRng::from_entropy();
                session.0 = GameSession::from_save_data(data, &mut rng);
                next_state.set(AppState::Exploration);
            }
        }
    }

    for interaction in &exit_q {
        if *interaction == Interaction::Pressed {
            std::process::exit(0);
        }
    }
}
