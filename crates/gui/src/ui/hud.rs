use bevy::prelude::*;
use carbonthrone::game::GamePhase;

use super::{StateUiRoot, accent_text, panel_bg, text_font, white_text};
use crate::resources::GameSessionRes;
use crate::state::AppState;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Exploration), spawn_exploration_hud)
            .add_systems(OnExit(AppState::Exploration), despawn_state_ui::<AppState>)
            .add_systems(Update, update_hud.run_if(in_state(AppState::Exploration)));
    }
}

#[derive(Component)]
struct HudZoneLabel;

#[derive(Component)]
struct HudLoopLabel;

#[derive(Component)]
struct HudPartyLabel;

fn spawn_exploration_hud(mut commands: Commands) {
    commands
        .spawn((
            StateUiRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(8.0),
                left: Val::Px(8.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            panel_bg(),
        ))
        .with_children(|parent| {
            parent.spawn((
                HudZoneLabel,
                Text::new("Zone: —"),
                text_font(14.0),
                accent_text(),
            ));
            parent.spawn((
                HudLoopLabel,
                Text::new("Loop 1"),
                text_font(12.0),
                white_text(),
            ));
            parent.spawn((HudPartyLabel, Text::new(""), text_font(12.0), white_text()));
        });
}

fn update_hud(
    session: Res<GameSessionRes>,
    mut zone_q: Query<
        &mut Text,
        (
            With<HudZoneLabel>,
            Without<HudLoopLabel>,
            Without<HudPartyLabel>,
        ),
    >,
    mut loop_q: Query<
        &mut Text,
        (
            With<HudLoopLabel>,
            Without<HudZoneLabel>,
            Without<HudPartyLabel>,
        ),
    >,
    mut party_q: Query<
        &mut Text,
        (
            With<HudPartyLabel>,
            Without<HudZoneLabel>,
            Without<HudLoopLabel>,
        ),
    >,
) {
    let GamePhase::Exploration(state) = &session.0.phase else {
        return;
    };

    if let Ok(mut t) = zone_q.single_mut() {
        let zone_name = format!("{:?}", state.zone.kind);
        *t = Text::new(format!("Zone: {}", zone_name));
    }
    if let Ok(mut t) = loop_q.single_mut() {
        *t = Text::new(format!("Loop {}", session.0.loop_number));
    }
    if let Ok(mut t) = party_q.single_mut() {
        let party_str = state
            .party
            .iter()
            .map(|c| format!("{} {}/{}", c.name, c.current_hp, c.stats.max_hp))
            .collect::<Vec<_>>()
            .join("  |  ");
        *t = Text::new(party_str);
    }
}

fn despawn_state_ui<S: States>(mut commands: Commands, q: Query<Entity, With<StateUiRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
