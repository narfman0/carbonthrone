mod camera;
mod character_visuals;
mod grid;
mod input;
mod resources;
mod state;
mod sync;
mod tile_mesh;
mod ui;

use bevy::prelude::*;
use carbonthrone::game::GameSession;

use camera::CameraPlugin;
use character_visuals::CharacterVisualsPlugin;
use input::InputPlugin;
use resources::{
    ExplorationRng, GameSessionRes, LastKnownZone, PendingAbilityTarget, PendingCombatPath,
    PendingExplorationPath, PendingPlayerChoices, SelectedChoiceIndex,
};
use state::AppState;
use sync::SyncPlugin;
use tile_mesh::TilePlugin;
use ui::UiPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Carbonthrone".into(),
                resolution: bevy::window::WindowResolution::new(1280_u32, 720_u32),
                ..default()
            }),
            ..default()
        }))
        // State machine
        .init_state::<AppState>()
        // Core resources
        .insert_resource(GameSessionRes(GameSession::new()))
        .init_resource::<ExplorationRng>()
        .init_resource::<LastKnownZone>()
        .init_resource::<PendingPlayerChoices>()
        .init_resource::<SelectedChoiceIndex>()
        .init_resource::<PendingAbilityTarget>()
        .init_resource::<PendingExplorationPath>()
        .init_resource::<PendingCombatPath>()
        // Plugins
        .add_plugins((
            CameraPlugin,
            TilePlugin,
            CharacterVisualsPlugin,
            SyncPlugin,
            InputPlugin,
            UiPlugin,
        ))
        .run();
}
