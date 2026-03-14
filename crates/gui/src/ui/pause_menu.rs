use bevy::prelude::*;
use carbonthrone::save::{load_all_slots, save_game};

use super::{accent_text, panel_bg, text_font};
use crate::{
    resources::{ActiveSaveSlot, GameSessionRes, PauseMenuOpen},
    state::AppState,
    ui::save_slot::slot_label,
};

pub struct PauseMenuPlugin;

impl Plugin for PauseMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PauseSubstate>()
            .add_systems(OnEnter(AppState::MainMenu), despawn_pause_menu)
            .add_systems(
                Update,
                (update_pause_menu, handle_pause_buttons).run_if(
                    in_state(AppState::Exploration)
                        .or(in_state(AppState::Dialog))
                        .or(in_state(AppState::Battle)),
                ),
            );
    }
}

#[derive(Resource, Default, PartialEq)]
enum PauseSubstate {
    #[default]
    Main,
    SaveSlots,
}

/// Root of the pause overlay — despawn this to remove the whole menu.
#[derive(Component)]
struct PauseMenuRoot;

#[derive(Component)]
struct ResumeButton;

#[derive(Component)]
struct SaveButton;

#[derive(Component)]
struct ExitMenuButton;

#[derive(Component)]
struct SlotSaveButton(u8);

#[derive(Component)]
struct SaveSlotsBackButton;

fn despawn_pause_menu(
    mut commands: Commands,
    root_q: Query<Entity, With<PauseMenuRoot>>,
    mut pause: ResMut<PauseMenuOpen>,
    mut substate: ResMut<PauseSubstate>,
) {
    for e in &root_q {
        commands.entity(e).despawn();
    }
    pause.0 = false;
    *substate = PauseSubstate::Main;
}

fn update_pause_menu(
    mut commands: Commands,
    pause: Res<PauseMenuOpen>,
    substate: Res<PauseSubstate>,
    root_q: Query<Entity, With<PauseMenuRoot>>,
    _session: Res<GameSessionRes>,
    active_slot: Res<ActiveSaveSlot>,
) {
    let needs_rebuild = pause.is_changed() || substate.is_changed();
    if !needs_rebuild {
        return;
    }

    // Despawn old overlay.
    for e in &root_q {
        commands.entity(e).despawn();
    }

    if !pause.0 {
        return;
    }

    let slots = load_all_slots();
    let in_save_mode = *substate == PauseSubstate::SaveSlots;

    commands
        .spawn((
            PauseMenuRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(0.0),
                top: Val::Percent(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(14.0),
                    padding: UiRect::all(Val::Px(32.0)),
                    min_width: Val::Px(340.0),
                    ..default()
                },
                panel_bg(),
            ))
            .with_children(|panel| {
                panel.spawn((Text::new("— PAUSED —"), text_font(30.0), accent_text()));

                if !in_save_mode {
                    // Main pause buttons
                    spawn_pause_btn(panel, "Resume", Color::srgb(0.15, 0.30, 0.15), ResumeButton);
                    spawn_pause_btn(panel, "Save", Color::srgb(0.15, 0.25, 0.40), SaveButton);
                    spawn_pause_btn(
                        panel,
                        "Exit to Menu",
                        Color::srgb(0.30, 0.12, 0.10),
                        ExitMenuButton,
                    );
                } else {
                    // Save slot picker
                    panel.spawn((Text::new("Select Slot to Save"), text_font(18.0), accent_text()));

                    for i in 0..4u8 {
                        let label = slot_label(i, slots[i as usize].as_ref());
                        let is_active = active_slot.0 == Some(i);
                        let bg = if is_active {
                            Color::srgb(0.10, 0.30, 0.45)
                        } else {
                            Color::srgb(0.15, 0.25, 0.40)
                        };
                        panel
                            .spawn((
                                SlotSaveButton(i),
                                Button,
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Px(42.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(bg),
                            ))
                            .with_children(|b| {
                                b.spawn((Text::new(label), text_font(14.0), TextColor(Color::WHITE)));
                            });
                    }

                    spawn_pause_btn(
                        panel,
                        "Back",
                        Color::srgb(0.20, 0.10, 0.10),
                        SaveSlotsBackButton,
                    );
                }
            });
        });
}

fn spawn_pause_btn<M: Component>(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    bg: Color,
    marker: M,
) {
    parent
        .spawn((
            marker,
            Button,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(46.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(bg),
        ))
        .with_children(|b| {
            b.spawn((Text::new(label.to_string()), text_font(18.0), TextColor(Color::WHITE)));
        });
}

fn handle_pause_buttons(
    resume_q: Query<&Interaction, (With<ResumeButton>, Changed<Interaction>)>,
    save_q: Query<&Interaction, (With<SaveButton>, Changed<Interaction>)>,
    exit_q: Query<&Interaction, (With<ExitMenuButton>, Changed<Interaction>)>,
    slot_q: Query<(&Interaction, &SlotSaveButton), Changed<Interaction>>,
    back_q: Query<&Interaction, (With<SaveSlotsBackButton>, Changed<Interaction>)>,
    mut pause: ResMut<PauseMenuOpen>,
    mut substate: ResMut<PauseSubstate>,
    mut next_state: ResMut<NextState<AppState>>,
    mut session: ResMut<GameSessionRes>,
    mut active_slot: ResMut<ActiveSaveSlot>,
) {
    for interaction in &resume_q {
        if *interaction == Interaction::Pressed {
            pause.0 = false;
            *substate = PauseSubstate::Main;
        }
    }

    for interaction in &save_q {
        if *interaction == Interaction::Pressed {
            *substate = PauseSubstate::SaveSlots;
        }
    }

    for (interaction, SlotSaveButton(slot)) in &slot_q {
        if *interaction == Interaction::Pressed {
            let data = session.0.to_save_data();
            let _ = save_game(&data, *slot);
            active_slot.0 = Some(*slot);
            *substate = PauseSubstate::Main;
        }
    }

    for interaction in &back_q {
        if *interaction == Interaction::Pressed {
            *substate = PauseSubstate::Main;
        }
    }

    for interaction in &exit_q {
        if *interaction == Interaction::Pressed {
            pause.0 = false;
            *substate = PauseSubstate::Main;
            next_state.set(AppState::MainMenu);
        }
    }
}
