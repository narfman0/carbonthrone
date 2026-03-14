pub mod combat;
pub mod dialog;
pub mod ending;
pub mod hud;
pub mod main_menu;
pub mod pause_menu;
pub mod save_slot;
pub mod terminal;
pub mod turn_log;

use bevy::prelude::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            main_menu::MainMenuPlugin,
            hud::HudPlugin,
            dialog::DialogPlugin,
            combat::CombatUiPlugin,
            turn_log::TurnLogPlugin,
            terminal::TerminalPlugin,
            ending::EndingPlugin,
            pause_menu::PauseMenuPlugin,
        ));
    }
}

// ── Shared UI helpers ─────────────────────────────────────────────────────────

/// Marker component for UI root nodes spawned per-state — cleaned up on exit.
#[derive(Component)]
pub struct StateUiRoot;

pub fn panel_bg() -> BackgroundColor {
    BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.88))
}

pub fn text_font(size: f32) -> TextFont {
    TextFont {
        font_size: size,
        ..default()
    }
}

pub fn white_text() -> TextColor {
    TextColor(Color::WHITE)
}

pub fn accent_text() -> TextColor {
    TextColor(Color::srgb(0.4, 0.8, 1.0))
}
