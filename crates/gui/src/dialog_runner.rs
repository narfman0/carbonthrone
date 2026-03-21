use std::collections::HashMap;

use bevy::prelude::*;
use bevy_yarnspinner::events::{DialogueCompleted, PresentLine, PresentOptions};
use bevy_yarnspinner::prelude::*;
use carbonthrone::game::GamePhase;

use crate::resources::{ExplorationRng, GameSessionRes};

pub struct DialogRunnerPlugin;

/// Current dialog line being displayed.
#[derive(Resource, Default)]
pub struct CurrentDialogLine {
    pub speaker: String,
    pub text: String,
    pub waiting: bool,
    /// True when showing a cached last-line fallback (NPC has nothing new to say).
    pub is_fallback: bool,
}

/// Current dialog options (choices).
#[derive(Resource, Default)]
pub struct CurrentDialogOptions {
    pub options: Vec<(OptionId, String)>,
    pub waiting: bool,
}

/// Stores the last line shown for each dialog node (keyed by loop-stripped name).
/// Persists across dialog sessions so NPCs can repeat their last words.
#[derive(Resource, Default)]
pub struct LastDialogLines {
    pub lines: HashMap<String, (String, String)>,
    /// Set when a node starts; used by on_present_line to know which key to update.
    pub current_key: String,
}

impl Plugin for DialogRunnerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentDialogLine>()
            .init_resource::<CurrentDialogOptions>()
            .init_resource::<LastDialogLines>()
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

/// Strip the loop prefix from a node name so lines are keyed by NPC identity,
/// not loop number.  E.g.:
///   "loop1_medical_bay_salvage_operative_on_interact"
///   → "medical_bay_salvage_operative_on_interact"
fn node_key(node_name: &str) -> String {
    node_name
        .strip_prefix("loop")
        .and_then(|s| s.splitn(2, '_').nth(1))
        .unwrap_or(node_name)
        .to_string()
}

/// Each frame: if a pending Yarn node is set on the `ExplorationState`, spawn
/// (or reuse) the `DialogueRunner` and call `start_node`.
fn start_pending_dialog_node(
    mut commands: Commands,
    mut session: ResMut<GameSessionRes>,
    yarn_project: Option<Res<YarnProject>>,
    mut runner_q: Query<&mut DialogueRunner, With<GameDialogueRunner>>,
    mut last_lines: ResMut<LastDialogLines>,
    mut line_res: ResMut<CurrentDialogLine>,
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

    // Validate that the node exists; fall back to last cached line if not.
    if project.headers_for_node(&node_name).is_none() {
        let key = node_key(&node_name);
        if let Some((speaker, text)) = e.last_dialog_lines.get(&key) {
            // Show the cached last line as a fallback.
            line_res.speaker = speaker.clone();
            line_res.text = text.clone();
            line_res.waiting = true;
            line_res.is_fallback = true;
            last_lines.current_key = key;
            // Keep in_dialog = true so the panel stays open.
        } else {
            // No node, no cache — dismiss silently.
            e.in_dialog = false;
        }
        return;
    }

    // Record the key for this node so on_present_line can update the cache.
    last_lines.current_key = node_key(&node_name);

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
    mut session: ResMut<GameSessionRes>,
    mut line_res: ResMut<CurrentDialogLine>,
    mut opts_res: ResMut<CurrentDialogOptions>,
    mut last_lines: ResMut<LastDialogLines>,
) {
    let line = &trigger.line;
    let speaker = line.character_name().unwrap_or("").to_string();
    let text = line.text_without_character_name();

    // Cache this line keyed by the current node.
    if !last_lines.current_key.is_empty() {
        let key = last_lines.current_key.clone();
        last_lines
            .lines
            .insert(key.clone(), (speaker.clone(), text.clone()));
        // Mirror into the core exploration state for save/load persistence.
        if let GamePhase::Exploration(e) = &mut session.0.phase {
            e.last_dialog_lines
                .insert(key, (speaker.clone(), text.clone()));
        }
    }

    line_res.speaker = speaker;
    line_res.text = text;
    line_res.waiting = true;
    line_res.is_fallback = false;
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
    last_lines: Res<LastDialogLines>,
    mut rng: ResMut<ExplorationRng>,
) {
    opts_res.waiting = false;
    opts_res.options.clear();

    // If no line was presented (all <<if>> conditions were false), fall back to
    // the last cached line for this node rather than silently dismissing.
    if line_res.text.is_empty() && !last_lines.current_key.is_empty() {
        let cached = {
            // Check GUI cache first; it matches the core state.
            last_lines
                .lines
                .get(&last_lines.current_key)
                .cloned()
                .or_else(|| {
                    if let GamePhase::Exploration(e) = &session.0.phase {
                        e.last_dialog_lines.get(&last_lines.current_key).cloned()
                    } else {
                        None
                    }
                })
        };
        if let Some((speaker, text)) = cached {
            line_res.speaker = speaker;
            line_res.text = text;
            line_res.waiting = true;
            line_res.is_fallback = true;
            // Keep in_dialog = true — do NOT call end_dialog.
            return;
        }
    }

    line_res.waiting = false;

    let Ok(runner) = runner_q.single() else {
        session.0.dismiss_dialog();
        return;
    };
    let new_flags = collect_set_flags(runner);
    let loop_complete = new_flags.contains(&"loop_complete".to_string());
    session.0.end_dialog(new_flags);
    if loop_complete {
        session.0.reset_loop(&mut rng.0);
    }
}
