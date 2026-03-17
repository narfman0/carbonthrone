use bevy::prelude::*;

use crate::state::AppState;

pub struct IntroPlugin;

impl Plugin for IntroPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<IntroProgress>()
            .add_systems(OnEnter(AppState::Intro), spawn_intro)
            .add_systems(OnExit(AppState::Intro), despawn_intro)
            .add_systems(Update, update_intro.run_if(in_state(AppState::Intro)));
    }
}

const FADE_IN: f32 = 1.2;
const HOLD: f32 = 2.5;
const FADE_OUT: f32 = 0.8;
const TOTAL: f32 = FADE_IN + HOLD + FADE_OUT;

#[derive(Resource, Default)]
struct IntroProgress {
    elapsed: f32,
}

#[derive(Component)]
struct IntroRoot;

#[derive(Component)]
struct IntroTitle;

#[derive(Component)]
struct IntroSubtitle;

fn spawn_intro(mut commands: Commands, mut progress: ResMut<IntroProgress>) {
    progress.elapsed = 0.0;

    commands
        .spawn((
            IntroRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .with_children(|parent| {
            parent.spawn((
                IntroTitle,
                Text::new("C A R B O N T H R O N E"),
                TextFont {
                    font_size: 52.0,
                    ..default()
                },
                TextColor(Color::srgba(0.4, 0.8, 1.0, 0.0)),
            ));
            parent.spawn((
                IntroSubtitle,
                Text::new("press any key"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgba(0.6, 0.6, 0.6, 0.0)),
            ));
        });
}

fn update_intro(
    time: Res<Time>,
    mut progress: ResMut<IntroProgress>,
    mut next_state: ResMut<NextState<AppState>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut title_q: Query<&mut TextColor, (With<IntroTitle>, Without<IntroSubtitle>)>,
    mut sub_q: Query<&mut TextColor, (With<IntroSubtitle>, Without<IntroTitle>)>,
) {
    let any_key =
        keys.get_just_pressed().next().is_some() || mouse.get_just_pressed().next().is_some();
    if any_key {
        next_state.set(AppState::MainMenu);
        return;
    }

    progress.elapsed += time.delta_secs();

    let alpha = if progress.elapsed < FADE_IN {
        progress.elapsed / FADE_IN
    } else if progress.elapsed < FADE_IN + HOLD {
        1.0
    } else {
        let out = progress.elapsed - FADE_IN - HOLD;
        (1.0 - out / FADE_OUT).max(0.0)
    };

    if let Ok(mut color) = title_q.single_mut() {
        color.0 = Color::srgba(0.4, 0.8, 1.0, alpha);
    }

    // Subtitle fades in slightly later and pulses.
    let sub_alpha = if progress.elapsed < FADE_IN * 0.5 {
        0.0
    } else {
        alpha * (0.5 + 0.5 * (progress.elapsed * 2.0).sin())
    };
    if let Ok(mut color) = sub_q.single_mut() {
        color.0 = Color::srgba(0.65, 0.65, 0.65, sub_alpha.clamp(0.0, 1.0));
    }

    if progress.elapsed >= TOTAL {
        next_state.set(AppState::MainMenu);
    }
}

fn despawn_intro(mut commands: Commands, q: Query<Entity, With<IntroRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
