use bevy::prelude::*;
use carbonthrone::{
    game::GameSession,
    save::{load_all_slots, load_game},
};
use rand::SeedableRng;

use super::{StateUiRoot, accent_text, panel_bg, text_font};
use crate::{
    resources::{ActiveSaveSlot, GameSessionRes},
    state::AppState,
    ui::save_slot::slot_label,
};

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MainMenuSubstate>()
            .add_systems(OnEnter(AppState::MainMenu), spawn_main_menu)
            .add_systems(OnExit(AppState::MainMenu), (despawn_main_menu, reset_substate))
            .add_systems(
                Update,
                (handle_menu_buttons, update_slot_panel).run_if(in_state(AppState::MainMenu)),
            );
    }
}

#[derive(Resource, Default, PartialEq)]
enum MainMenuSubstate {
    #[default]
    Normal,
    LoadSlots,
}

#[derive(Component)]
struct NewGameButton;

#[derive(Component)]
struct LoadGameButton;

#[derive(Component)]
struct ExitButton;

/// Marker for the load-slots overlay panel.
#[derive(Component)]
struct LoadSlotsPanel;

/// Identifies a slot button with its index.
#[derive(Component)]
struct SlotLoadButton(u8);

/// Back button in load slots panel.
#[derive(Component)]
struct LoadSlotsBackButton;

fn spawn_main_menu(mut commands: Commands) {
    let slots = load_all_slots();
    let any_saves = slots.iter().any(|s| s.is_some());

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
            let load_bg = if any_saves {
                Color::srgb(0.15, 0.25, 0.40)
            } else {
                Color::srgb(0.12, 0.12, 0.15)
            };
            let load_text_color = if any_saves {
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

fn reset_substate(mut substate: ResMut<MainMenuSubstate>) {
    *substate = MainMenuSubstate::Normal;
}

/// Spawn or despawn the load-slots panel based on `MainMenuSubstate`.
fn update_slot_panel(
    mut commands: Commands,
    substate: Res<MainMenuSubstate>,
    panel_q: Query<Entity, With<LoadSlotsPanel>>,
) {
    if !substate.is_changed() {
        return;
    }
    // Despawn existing panel.
    for e in &panel_q {
        commands.entity(e).despawn();
    }
    if *substate == MainMenuSubstate::LoadSlots {
        let slots = load_all_slots();
        commands
            .spawn((
                LoadSlotsPanel,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(25.0),
                    top: Val::Percent(20.0),
                    width: Val::Percent(50.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(12.0),
                    padding: UiRect::all(Val::Px(24.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.05, 0.05, 0.12, 0.96)),
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new("Select Save Slot"),
                    text_font(26.0),
                    accent_text(),
                ));

                for i in 0..4u8 {
                    let label = slot_label(i, slots[i as usize].as_ref());
                    let has_data = slots[i as usize].is_some();
                    let (bg, text_col) = if has_data {
                        (Color::srgb(0.15, 0.25, 0.40), Color::WHITE)
                    } else {
                        (Color::srgb(0.10, 0.10, 0.14), Color::srgb(0.4, 0.4, 0.4))
                    };

                    let mut btn = parent.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(44.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(bg),
                    ));
                    if has_data {
                        btn.insert((SlotLoadButton(i), Button));
                    }
                    btn.with_children(|b| {
                        b.spawn((Text::new(label), text_font(16.0), TextColor(text_col)));
                    });
                }

                // Back button
                parent
                    .spawn((
                        LoadSlotsBackButton,
                        Button,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(40.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            margin: UiRect::top(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.20, 0.10, 0.10)),
                    ))
                    .with_children(|b| {
                        b.spawn((Text::new("Back"), text_font(16.0), TextColor(Color::WHITE)));
                    });
            });
    }
}

fn handle_menu_buttons(
    new_game_q: Query<&Interaction, (With<NewGameButton>, Changed<Interaction>)>,
    load_game_q: Query<&Interaction, (With<LoadGameButton>, Changed<Interaction>)>,
    exit_q: Query<&Interaction, (With<ExitButton>, Changed<Interaction>)>,
    slot_q: Query<(&Interaction, &SlotLoadButton), Changed<Interaction>>,
    back_q: Query<&Interaction, (With<LoadSlotsBackButton>, Changed<Interaction>)>,
    mut session: ResMut<GameSessionRes>,
    mut next_state: ResMut<NextState<AppState>>,
    mut substate: ResMut<MainMenuSubstate>,
    mut active_slot: ResMut<ActiveSaveSlot>,
) {
    for interaction in &new_game_q {
        if *interaction == Interaction::Pressed {
            session.0 = GameSession::new();
            next_state.set(AppState::Exploration);
        }
    }

    for interaction in &load_game_q {
        if *interaction == Interaction::Pressed {
            let slots = load_all_slots();
            if slots.iter().any(|s| s.is_some()) {
                *substate = MainMenuSubstate::LoadSlots;
            }
        }
    }

    for (interaction, SlotLoadButton(slot)) in &slot_q {
        if *interaction == Interaction::Pressed {
            if let Ok(data) = load_game(*slot) {
                let mut rng = rand::rngs::StdRng::from_entropy();
                session.0 = GameSession::from_save_data(data, &mut rng);
                active_slot.0 = Some(*slot);
                next_state.set(AppState::Exploration);
            }
        }
    }

    for interaction in &back_q {
        if *interaction == Interaction::Pressed {
            *substate = MainMenuSubstate::Normal;
        }
    }

    for interaction in &exit_q {
        if *interaction == Interaction::Pressed {
            std::process::exit(0);
        }
    }
}
