use bevy::prelude::*;
use carbonthrone::combat::TurnAction;

use super::{StateUiRoot, panel_bg, text_font, white_text};
use crate::resources::GameSessionRes;
use crate::state::AppState;

pub struct TurnLogPlugin;

impl Plugin for TurnLogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TurnLog>()
            .add_systems(OnEnter(AppState::Battle), spawn_turn_log_panel)
            .add_systems(OnExit(AppState::Battle), despawn_turn_log)
            .add_systems(
                Update,
                (collect_turn_events, update_turn_log_display)
                    .chain()
                    .run_if(in_state(AppState::Battle)),
            );
    }
}

/// Rolling log of the last N turn event summaries.
#[derive(Resource, Default)]
pub struct TurnLog {
    pub lines: Vec<String>,
    pub last_processed_id: u64,
}

impl TurnLog {
    const MAX_LINES: usize = 10;

    pub fn push(&mut self, line: String) {
        self.lines.push(line);
        if self.lines.len() > Self::MAX_LINES {
            self.lines.remove(0);
        }
    }
}

#[derive(Component)]
struct TurnLogPanel;

#[derive(Component)]
struct TurnLogText;

fn spawn_turn_log_panel(mut commands: Commands) {
    commands
        .spawn((
            StateUiRoot,
            TurnLogPanel,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(8.0),
                top: Val::Px(8.0),
                width: Val::Px(280.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            panel_bg(),
        ))
        .with_children(|parent| {
            parent.spawn((TurnLogText, Text::new(""), text_font(11.0), white_text()));
        });
}

fn collect_turn_events(session: Res<GameSessionRes>, mut log: ResMut<TurnLog>) {
    if !session.is_changed() {
        return;
    }
    let Some(event) = &session.0.last_event else {
        return;
    };
    // Use a pointer-based ID to avoid re-processing the same event.
    let event_id = event as *const _ as u64;
    if log.last_processed_id == event_id {
        return;
    }
    log.last_processed_id = event_id;

    for action in &event.actions {
        let line = format_action(action, &session);
        log.push(line);
    }
    if let Some(outcome) = &event.outcome {
        log.push(format!("=== {:?} ===", outcome));
    }
}

fn format_action(action: &TurnAction, session: &GameSessionRes) -> String {
    let entity_name = |e| {
        session
            .0
            .world
            .get::<carbonthrone::character::Character>(e)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "?".to_string())
    };
    match action {
        TurnAction::Move { to } => format!("→ ({}, {})", to.x, to.y),
        TurnAction::UseAbility {
            ability_name,
            hit,
            value,
            target,
            ..
        } => {
            let target_name = target
                .map(entity_name)
                .unwrap_or_else(|| "self".to_string());
            if *hit {
                format!("{} → {} ({})", ability_name, target_name, value)
            } else {
                format!("{} → {} MISS", ability_name, target_name)
            }
        }
        TurnAction::DisplacementQueued {
            target,
            pending_damage,
            resolve_round,
            ..
        } => {
            format!(
                "Displacement → {} queued {}dmg (round {})",
                entity_name(*target),
                pending_damage,
                resolve_round
            )
        }
        TurnAction::DisplacementHit { target, damage } => {
            format!("Displacement hits {} for {}", entity_name(*target), damage)
        }
        TurnAction::AccelerationApplied { bonus_ap, .. } => {
            format!("Acceleration: +{} AP (drain next turn)", bonus_ap)
        }
        TurnAction::DrainApplied { target, drained } => {
            format!("AP drain: {} loses {} AP", entity_name(*target), drained)
        }
        TurnAction::TemporalCollapse { victim, damage } => {
            format!(
                "⚡ TEMPORAL COLLAPSE! {} takes {} dmg; all AP drained",
                entity_name(*victim),
                damage
            )
        }
        TurnAction::TemporalRecalled { target, from, to } => {
            format!(
                "{} recalled ({},{}) → ({},{})",
                entity_name(*target),
                from.x,
                from.y,
                to.x,
                to.y
            )
        }
        TurnAction::GlitchTeleport { entity, from, to } => {
            format!(
                "⚡ {} glitch teleports ({},{}) → ({},{})",
                entity_name(*entity),
                from.x,
                from.y,
                to.x,
                to.y
            )
        }
    }
}

fn update_turn_log_display(log: Res<TurnLog>, mut text_q: Query<&mut Text, With<TurnLogText>>) {
    if !log.is_changed() {
        return;
    }
    if let Ok(mut text) = text_q.single_mut() {
        *text = Text::new(log.lines.join("\n"));
    }
}

fn despawn_turn_log(mut commands: Commands, q: Query<Entity, With<TurnLogPanel>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
