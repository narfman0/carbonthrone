pub mod combat;
pub mod dialog;
pub mod hud;
pub mod turn_log;

use bevy::prelude::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            hud::HudPlugin,
            dialog::DialogPlugin,
            combat::CombatUiPlugin,
            turn_log::TurnLogPlugin,
        ));
    }
}

// ── Shared UI helpers ─────────────────────────────────────────────────────────

/// Marker component for UI root nodes spawned per-state — cleaned up on exit.
#[derive(Component)]
pub struct StateUiRoot;

/// Common semi-transparent dark panel style.
pub fn dark_panel() -> Node {
    Node {
        position_type: PositionType::Absolute,
        padding: UiRect::all(Val::Px(8.0)),
        ..default()
    }
}

pub fn panel_bg() -> BackgroundColor {
    BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.88))
}

pub fn label_text(text: impl Into<String>, size: f32) -> Text {
    Text::new(text)
}

pub fn text_font(size: f32) -> TextFont {
    TextFont {
        font_size: size,
        ..default()
    }
}

pub fn text_color(color: Color) -> TextColor {
    TextColor(color)
}

pub fn white_text() -> TextColor {
    TextColor(Color::WHITE)
}

pub fn accent_text() -> TextColor {
    TextColor(Color::srgb(0.4, 0.8, 1.0))
}
