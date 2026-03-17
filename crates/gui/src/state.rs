use bevy::prelude::*;

/// Top-level GUI state machine.
///
/// `OnEnter`/`OnExit` hooks drive visual entity lifecycle.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    /// Intro title sequence shown on first launch.
    #[default]
    Intro,
    /// Main menu: new game, load game, or exit.
    MainMenu,
    /// In-world exploration: player visible on zone map, NPCs present.
    Exploration,
    /// Dialog overlay active on top of the exploration view.
    Dialog,
    /// Combat encounter active.
    Battle,
    /// A story ending has been reached; shows the ending screen.
    Ended,
}
