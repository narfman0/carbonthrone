use bevy::prelude::*;
use bevy_yarnspinner::events::{DialogueCompleted, PresentLine, PresentOptions};
use bevy_yarnspinner::prelude::*;
use carbonthrone::game::GamePhase;

use crate::resources::GameSessionRes;

pub struct DialogRunnerPlugin;

/// Current dialog line being displayed.
#[derive(Resource, Default)]
pub struct CurrentDialogLine {
    pub speaker: String,
    pub text: String,
    pub waiting: bool,
}

/// Current dialog options (choices).
#[derive(Resource, Default)]
pub struct CurrentDialogOptions {
    pub options: Vec<(OptionId, String)>,
    pub waiting: bool,
}

impl Plugin for DialogRunnerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentDialogLine>()
            .init_resource::<CurrentDialogOptions>()
            .add_systems(
                Update,
                start_pending_dialog_node.before(YarnSpinnerSystemSet),
            )
            .add_observer(on_present_line)
            .add_observer(on_present_options)
            .add_observer(on_dialogue_completed);
    }
}

/// Marker component on the entity that owns the `DialogueRunner`.
#[derive(Component)]
pub struct GameDialogueRunner;

/// Each frame: if a pending Yarn node is set on the `ExplorationState`, spawn
/// (or reuse) the `DialogueRunner` and call `start_node`.
fn start_pending_dialog_node(
    mut commands: Commands,
    mut session: ResMut<GameSessionRes>,
    yarn_project: Option<Res<YarnProject>>,
    mut runner_q: Query<&mut DialogueRunner, With<GameDialogueRunner>>,
) {
    let GamePhase::Exploration(e) = &mut session.0.phase else {
        return;
    };
    let Some(node_name) = e.pending_dialog_node.take() else {
        return;
    };
    let Some(project) = yarn_project else {
        // YarnProject not ready yet — put the node back and try next frame.
        e.pending_dialog_node = Some(node_name);
        return;
    };

    // Validate that the node exists; skip silently if not.
    if project.headers_for_node(&node_name).is_none() {
        // No dialog for this trigger — clear in_dialog so the game can proceed.
        e.in_dialog = false;
        return;
    }

    // Seed variable storage with current flags so .yarn <<if>> guards work.
    let flags: Vec<String> = e.flags.export_flags();
    let companion = e.active_companion.clone();

    if let Ok(mut runner) = runner_q.single_mut() {
        // Reuse existing runner (only if not already running).
        if runner.is_running() {
            // Previous dialog still active — put node back and wait.
            e.pending_dialog_node = Some(node_name);
            return;
        }
        seed_runner_variables(&mut *runner, &flags, companion.as_deref());
        runner.start_node(&node_name);
    } else {
        // Spawn a new runner entity.
        let mut runner = project.create_dialogue_runner(&mut commands);
        seed_runner_variables(&mut runner, &flags, companion.as_deref());
        runner.start_node(&node_name);
        commands.spawn((runner, GameDialogueRunner));
    }
}

fn seed_runner_variables(runner: &mut DialogueRunner, flags: &[String], companion: Option<&str>) {
    for flag in flags {
        let _ = runner
            .variable_storage_mut()
            .set(format!("${flag}"), true.into());
    }
    if let Some(c) = companion {
        let _ = runner
            .variable_storage_mut()
            .set("$companion".to_string(), c.to_string().into());
    }
}

fn on_present_line(
    trigger: On<PresentLine>,
    mut line_res: ResMut<CurrentDialogLine>,
    mut opts_res: ResMut<CurrentDialogOptions>,
) {
    let line = &trigger.line;
    line_res.speaker = line.character_name().unwrap_or("").to_string();
    line_res.text = line.text_without_character_name();
    line_res.waiting = true;
    opts_res.waiting = false;
    opts_res.options.clear();
}

fn on_present_options(
    trigger: On<PresentOptions>,
    mut line_res: ResMut<CurrentDialogLine>,
    mut opts_res: ResMut<CurrentDialogOptions>,
) {
    opts_res.options = trigger
        .options
        .iter()
        .filter(|o| o.is_available)
        .map(|o| (o.id, o.line.text_without_character_name()))
        .collect();
    opts_res.waiting = true;
    line_res.waiting = false;
}

/// Collect all boolean variables set to `true` in the runner's storage and
/// return them as flag names (stripping the leading `$`).
fn collect_set_flags(runner: &DialogueRunner) -> Vec<String> {
    runner
        .variable_storage()
        .variables()
        .into_iter()
        .filter_map(|(name, value)| {
            let is_true = match value {
                YarnValue::Boolean(b) => b,
                _ => false,
            };
            if is_true {
                Some(name.trim_start_matches('$').to_string())
            } else {
                None
            }
        })
        .collect()
}

fn on_dialogue_completed(
    _trigger: On<DialogueCompleted>,
    mut session: ResMut<GameSessionRes>,
    runner_q: Query<&DialogueRunner, With<GameDialogueRunner>>,
    mut line_res: ResMut<CurrentDialogLine>,
    mut opts_res: ResMut<CurrentDialogOptions>,
) {
    line_res.waiting = false;
    opts_res.waiting = false;
    opts_res.options.clear();

    let Ok(runner) = runner_q.single() else {
        session.0.dismiss_dialog();
        return;
    };
    let new_flags = collect_set_flags(runner);
    session.0.end_dialog(new_flags);
}
