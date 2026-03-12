use bevy::prelude::*;
use carbonthrone::game::{EndingKind, GamePhase};

use super::{StateUiRoot, accent_text, panel_bg, text_font, white_text};
use crate::{resources::GameSessionRes, state::AppState};

pub struct EndingPlugin;

impl Plugin for EndingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Ended), spawn_ending_ui)
            .add_systems(OnExit(AppState::Ended), despawn_ending_ui);
    }
}

fn spawn_ending_ui(mut commands: Commands, session: Res<GameSessionRes>) {
    let GamePhase::Ended(ending) = &session.0.phase else {
        return;
    };
    let (title, body) = ending_text(ending);

    commands
        .spawn((
            StateUiRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            panel_bg(),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("C A R B O N T H R O N E"),
                text_font(36.0),
                accent_text(),
            ));
            parent.spawn((
                Text::new("- - - F I N - - -"),
                text_font(20.0),
                white_text(),
            ));
            parent.spawn(Node {
                height: Val::Px(30.0),
                ..default()
            });
            parent.spawn((Text::new(title), text_font(24.0), accent_text()));
            parent.spawn(Node {
                height: Val::Px(16.0),
                ..default()
            });
            parent.spawn((Text::new(body), text_font(16.0), white_text()));
            parent.spawn(Node {
                height: Val::Px(40.0),
                ..default()
            });
            parent.spawn((
                Text::new("Press any key to quit"),
                text_font(14.0),
                white_text(),
            ));
        });
}

fn despawn_ending_ui(mut commands: Commands, roots: Query<Entity, With<StateUiRoot>>) {
    for e in &roots {
        commands.entity(e).despawn();
    }
}

fn ending_text(ending: &EndingKind) -> (&'static str, &'static str) {
    match ending {
        EndingKind::DossPreserved => (
            "ENDING: The Unaltered Line",
            "You chose to preserve the timeline.\nThe loop ends. The war happens as it always did.\nSomewhere, the station drifts. Nobody knows why the loop began.",
        ),
        EndingKind::KaleoCompromise => (
            "ENDING: The Redirected Anomaly",
            "You sided with Kaleo.\nThe anomaly folds harmlessly. The loop dissolves.\nThe war is slightly altered — for better or worse, only time will tell.",
        ),
        EndingKind::SableDestroyed => (
            "ENDING: The Destroyed Program",
            "You helped Orin destroy the program.\nThe loop ends. Orin doesn't survive.\nShe got what she came for. She knew she wouldn't make it out.",
        ),
        EndingKind::SableStopped => (
            "ENDING: The Preserved Truth",
            "You stopped Orin. The program persists. The loop ends.\nShe nods, once. The grief in her is quiet and enormous\nand she doesn't try to make you feel it.",
        ),
        EndingKind::Default => (
            "ENDING: Contained",
            "The anomaly is contained. The loop ends.\nThe war happens. Nobody knows why it began.\nPerhaps that's enough.",
        ),
    }
}
