use serde::{Deserialize, Serialize};

use crate::character::CharacterKind;
use crate::zone::ZoneKind;

const SAVE_PATH: &str = "save.yaml";

/// Minimal persistent state needed to reconstruct a game session.
///
/// Everything else (zone terrain, enemy positions, dialog scenes) is procedural
/// and regenerated on load.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    /// Current loop number (1–5).
    pub loop_number: u32,
    /// All raised dialog flags, sorted for deterministic output.
    pub flags: Vec<String>,
    /// Active companion name, if chosen ("orin", "doss", or "kaleo").
    pub active_companion: Option<String>,
    /// Zone the player was in when they saved.
    pub current_zone: ZoneKind,
    /// `CharacterKind` for each party member (drives stat reconstruction).
    pub party_kinds: Vec<CharacterKind>,
    /// Current HP per party member (parallel to `party_kinds`).
    pub party_hp: Vec<i32>,
}

/// Write a [`SaveData`] to `save.yaml` in the current directory.
pub fn save_game(data: &SaveData) -> Result<(), Box<dyn std::error::Error>> {
    let yaml = serde_yaml::to_string(data)?;
    std::fs::write(SAVE_PATH, yaml)?;
    Ok(())
}

/// Load a [`SaveData`] from `save.yaml` in the current directory.
pub fn load_game() -> Result<SaveData, Box<dyn std::error::Error>> {
    let yaml = std::fs::read_to_string(SAVE_PATH)?;
    let data: SaveData = serde_yaml::from_str(&yaml)?;
    Ok(data)
}
