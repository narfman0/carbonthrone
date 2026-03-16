mod camera;
mod character_visuals;
mod dialog_runner;
mod grid;
mod input;
mod resources;
mod state;
mod sync;
mod tile_mesh;
mod ui;

use bevy::prelude::*;
use bevy_yarnspinner::prelude::*;
use carbonthrone::game::GameSession;

use camera::CameraPlugin;
use character_visuals::CharacterVisualsPlugin;
use dialog_runner::DialogRunnerPlugin;
use input::InputPlugin;
use resources::{
    ActiveSaveSlot, ExplorationRng, GameSessionRes, LastKnownZone, MinimapOpen, PauseMenuOpen,
    PendingAbilityTarget, PendingPath, PendingPlayerChoices, SelectedChoiceIndex, TerminalState,
};
#[cfg(feature = "dev")]
use resources::InspectorOpen;
use state::AppState;
use sync::SyncPlugin;
use tile_mesh::TilePlugin;
use ui::UiPlugin;

fn main() {
    let mut app = App::new();
    app
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Carbonthrone".into(),
                resolution: bevy::window::WindowResolution::new(1280_u32, 720_u32),
                ..default()
            }),
            ..default()
        }))
        // Yarn Spinner
        .add_plugins(YarnSpinnerPlugin::new())
        // State machine
        .init_state::<AppState>()
        // Core resources
        .insert_resource(GameSessionRes(GameSession::new()))
        .init_resource::<ExplorationRng>()
        .init_resource::<LastKnownZone>()
        .init_resource::<PendingPlayerChoices>()
        .init_resource::<SelectedChoiceIndex>()
        .init_resource::<PendingAbilityTarget>()
        .init_resource::<PendingPath>()
        .init_resource::<TerminalState>()
        .init_resource::<PauseMenuOpen>()
        .init_resource::<MinimapOpen>()
        .init_resource::<ActiveSaveSlot>()
        // Plugins
        .add_plugins((
            CameraPlugin,
            TilePlugin,
            CharacterVisualsPlugin,
            SyncPlugin,
            InputPlugin,
            UiPlugin,
            DialogRunnerPlugin,
        ));

    #[cfg(feature = "dev")]
    {
        app.init_resource::<InspectorOpen>();
        app.add_plugins(
            bevy_inspector_egui::quick::WorldInspectorPlugin::new()
                .run_if(|r: Res<InspectorOpen>| r.0),
        );
    }

    app.run();
}
