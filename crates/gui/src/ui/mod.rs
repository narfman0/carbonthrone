pub mod combat;
pub mod dialog;
pub mod ending;
pub mod hud;
pub mod intro;
pub mod main_menu;
pub mod minimap;
pub mod pause_menu;
pub mod save_slot;
pub mod terminal;
pub mod turn_log;
#[cfg(feature = "dev")]
pub mod world_building;

use bevy::prelude::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            intro::IntroPlugin,
            main_menu::MainMenuPlugin,
            hud::HudPlugin,
            dialog::DialogPlugin,
            combat::CombatUiPlugin,
            turn_log::TurnLogPlugin,
            terminal::TerminalPlugin,
            ending::EndingPlugin,
            pause_menu::PauseMenuPlugin,
            minimap::MinimapPlugin,
        ))
        .add_systems(Update, button_hover_system);
    }
}

// ── Shared UI helpers ─────────────────────────────────────────────────────────

/// Marker component for UI root nodes spawned per-state — cleaned up on exit.
#[derive(Component)]
pub struct StateUiRoot;

/// Attach to any `Button` to get automatic hover color transitions.
#[derive(Component)]
pub struct ButtonColors {
    pub normal: Color,
    pub hovered: Color,
}

impl ButtonColors {
    pub fn new(normal: Color) -> Self {
        // Hovered is the normal color brightened slightly.
        let s = normal.to_srgba();
        let hovered = Color::srgba(
            (s.red + 0.18).clamp(0.0, 1.0),
            (s.green + 0.18).clamp(0.0, 1.0),
            (s.blue + 0.18).clamp(0.0, 1.0),
            s.alpha,
        );
        Self { normal, hovered }
    }
}

pub fn button_hover_system(
    mut query: Query<(&Interaction, &ButtonColors, &mut BackgroundColor), Changed<Interaction>>,
) {
    for (interaction, colors, mut bg) in query.iter_mut() {
        bg.0 = match interaction {
            Interaction::Hovered | Interaction::Pressed => colors.hovered,
            Interaction::None => colors.normal,
        };
    }
}

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
