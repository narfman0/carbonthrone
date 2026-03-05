use std::collections::{HashMap, HashSet};

use serde::Deserialize;

/// Which companion is required to be active for a scene to trigger.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Companion {
    Any,
    Orin,
    Doss,
    Kaleo,
}

/// The in-world event that fires a scene.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    OnEnter,
    OnCombatEnd,
    OnInteract,
    OnChoice,
}

/// Conditions that must hold for a scene to be shown.
#[derive(Debug, Clone, Deserialize)]
pub struct SceneRequires {
    pub companion: Companion,
    #[serde(default)]
    pub flags_set: Vec<String>,
    #[serde(default)]
    pub flags_unset: Vec<String>,
}

/// A single line of spoken dialog.
#[derive(Debug, Clone, Deserialize)]
pub struct DialogLine {
    pub speaker: String,
    pub text: String,
    pub emotion: String,
}

/// A player-selectable option at the end of a scene.
#[derive(Debug, Clone, Deserialize)]
pub struct Choice {
    pub text: String,
    pub leads_to: Option<String>,
    #[serde(default)]
    pub sets_flag: Option<String>,
}

/// A self-contained dialog scene loaded from a loop YAML file.
#[derive(Debug, Clone, Deserialize)]
pub struct Scene {
    pub id: String,
    pub location: String,
    pub trigger: Trigger,
    pub requires: SceneRequires,
    pub lines: Vec<DialogLine>,
    #[serde(default)]
    pub choices: Option<Vec<Choice>>,
    pub sets_flag: Option<String>,
}

/// Top-level structure of a loop YAML file (private; only scenes are exposed).
#[derive(Debug, Deserialize)]
struct DialogScript {
    #[allow(dead_code)]
    #[serde(rename = "loop")]
    loop_num: u8,
    scenes: Vec<Scene>,
}

/// Runtime dialog state machine.
///
/// Load one or more loop YAML scripts, then use [`DialogEngine::trigger`] to
/// fire scenes by event and location.  Player choices are resolved with
/// [`DialogEngine::select_choice`].  Flag state is kept here and checked
/// automatically when evaluating scene requirements.
#[derive(Debug, Default)]
pub struct DialogEngine {
    scenes: HashMap<String, Scene>,
    flags: HashSet<String>,
    current_scene: Option<String>,
    active_companion: Option<String>,
}

impl DialogEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a YAML string and register all its scenes.
    pub fn load_script(&mut self, yaml: &str) -> Result<(), serde_yaml::Error> {
        let script: DialogScript = serde_yaml::from_str(yaml)?;
        for scene in script.scenes {
            self.scenes.insert(scene.id.clone(), scene);
        }
        Ok(())
    }

    /// Set which companion is currently travelling with the player.
    /// Pass `"orin"`, `"doss"`, or `"kaleo"` (case-insensitive stored as-is).
    pub fn set_companion(&mut self, companion: impl Into<String>) {
        self.active_companion = Some(companion.into());
    }

    /// Manually raise a flag (also done automatically by scenes and choices).
    pub fn set_flag(&mut self, flag: impl Into<String>) {
        self.flags.insert(flag.into());
    }

    /// Returns `true` when the named flag has been raised.
    pub fn is_flag_set(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }

    /// Return the currently active scene, if any.
    pub fn current_scene(&self) -> Option<&Scene> {
        self.current_scene
            .as_ref()
            .and_then(|id| self.scenes.get(id))
    }

    // ── Internal helpers ──────────────────────────────────────────────────

    fn companion_matches(active: &Option<String>, required: &Companion) -> bool {
        match required {
            Companion::Any => true,
            Companion::Orin => active.as_deref() == Some("orin"),
            Companion::Doss => active.as_deref() == Some("doss"),
            Companion::Kaleo => active.as_deref() == Some("kaleo"),
        }
    }

    fn scene_available(
        scene: &Scene,
        flags: &HashSet<String>,
        active_companion: &Option<String>,
    ) -> bool {
        let req = &scene.requires;
        if !Self::companion_matches(active_companion, &req.companion) {
            return false;
        }
        req.flags_set.iter().all(|f| flags.contains(f))
            && req.flags_unset.iter().all(|f| !flags.contains(f))
    }

    // ── Public API ────────────────────────────────────────────────────────

    /// Find and activate the first available scene that matches the given
    /// `trigger` and `location`.  Applies the scene's `sets_flag` if present.
    /// Returns `None` when no matching scene passes its requirements.
    pub fn trigger(&mut self, trigger: &Trigger, location: &str) -> Option<&Scene> {
        let scene_id = {
            let flags = &self.flags;
            let companion = &self.active_companion;
            self.scenes
                .values()
                .find(|s| {
                    &s.trigger == trigger
                        && s.location == location
                        && Self::scene_available(s, flags, companion)
                })
                .map(|s| s.id.clone())?
        };

        self.activate_scene(scene_id)
    }

    /// Jump directly to a scene by id (e.g. after resolving a `leads_to`).
    /// Returns `None` if the scene does not exist or its requirements are unmet.
    pub fn go_to_scene(&mut self, scene_id: &str) -> Option<&Scene> {
        let id = scene_id.to_string();
        let available = self
            .scenes
            .get(&id)
            .map(|s| Self::scene_available(s, &self.flags, &self.active_companion))?;
        if !available {
            return None;
        }
        self.activate_scene(id)
    }

    /// Resolve the choice at `choice_index` in the current scene.
    /// Applies the choice's `sets_flag` and navigates to `leads_to` if set.
    /// Returns the destination scene, or `None` when `leads_to` is absent.
    pub fn select_choice(&mut self, choice_index: usize) -> Option<&Scene> {
        let scene_id = self.current_scene.clone()?;
        let (flag_to_set, next_id) = {
            let scene = self.scenes.get(&scene_id)?;
            let choices = scene.choices.as_ref()?;
            let choice = choices.get(choice_index)?;
            (choice.sets_flag.clone(), choice.leads_to.clone())
        };

        if let Some(flag) = flag_to_set {
            self.flags.insert(flag);
        }

        let next_id = next_id?;
        self.go_to_scene(&next_id)
    }

    fn activate_scene(&mut self, scene_id: String) -> Option<&Scene> {
        if let Some(flag) = self.scenes.get(&scene_id)?.sets_flag.clone() {
            self.flags.insert(flag);
        }
        self.current_scene = Some(scene_id.clone());
        self.scenes.get(&scene_id)
    }
}
