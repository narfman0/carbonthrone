use bevy::prelude::Component;

/// Grid position for any entity on a level. Values are integers; the UI
/// handles visual interpolation between tiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}
