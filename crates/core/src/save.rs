use serde::{Deserialize, Serialize};

use crate::character::{Aggression, CharacterKind};
use crate::combat::Turn;
use crate::game::EndingKind;
use crate::terrain::Tile;
use crate::zone::ZoneKind;

/// Role a combatant plays in the battle, used to route save/restore logic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CombatantRole {
    /// The main player character (Researcher).
    Player,
    /// A persistent party companion.
    Companion,
    /// An enemy entity (despawned after combat).
    Enemy,
    /// A scripted temporary ally (despawned after combat).
    ScriptedAlly,
}

/// Full state of one combatant entity during a battle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatantSnapshot {
    pub role: CombatantRole,
    pub kind: CharacterKind,
    pub level: u32,
    pub current_hp: i32,
    pub max_hp: i32,
    pub current_ap: i32,
    pub max_ap: i32,
    pub x: i32,
    pub y: i32,
    pub aggression: Aggression,
    /// Ability name for `ScriptedFirstAction`, if present.
    pub scripted_first_action_name: Option<String>,
    /// Whether the scripted first action has already been executed.
    pub scripted_first_action_executed: bool,
}

/// Complete battle state snapshot — map, combatants, and turn order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleSnapshot {
    pub round: u32,
    pub turn: Turn,
    /// Indices into `combatants` for the remaining actor queue (front = currently active).
    pub actor_queue: Vec<usize>,
    pub combatants: Vec<CombatantSnapshot>,
    pub map_cols: u32,
    pub map_rows: u32,
    pub zone_kind: ZoneKind,
    /// Non-Open tiles only (Open is the default; omitting it keeps the list compact).
    pub map_tiles: Vec<((i32, i32), Tile)>,
}

const SAVE_DIR: &str = "Carbonthrone";

/// Returns the save file path: `{Documents}/Carbonthrone/save_{slot}.yaml`.
/// Falls back to `save_{slot}.yaml` in the current directory if the Documents
/// folder cannot be determined.
fn save_path(slot: u8) -> std::path::PathBuf {
    if let Some(docs) = dirs::document_dir() {
        let dir = docs.join(SAVE_DIR);
        let _ = std::fs::create_dir_all(&dir);
        dir.join(format!("save_{slot}.yaml"))
    } else {
        std::path::PathBuf::from(format!("save_{slot}.yaml"))
    }
}

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
    /// Level per party member (parallel to `party_kinds`).
    #[serde(default)]
    pub party_levels: Vec<u32>,
    /// Scripted encounter ids the player has already fought.
    #[serde(default)]
    pub fought_scripted_encounters: Vec<String>,
    /// Story ending reached (if the game has ended).
    #[serde(default)]
    pub ending: Option<EndingKind>,
    /// Full battle state snapshot (present only when saved mid-battle).
    #[serde(default)]
    pub battle_snapshot: Option<BattleSnapshot>,
}

/// Write a [`SaveData`] to `{Documents}/Carbonthrone/save_{slot}.yaml`.
pub fn save_game(data: &SaveData, slot: u8) -> Result<(), Box<dyn std::error::Error>> {
    let yaml = serde_yaml::to_string(data)?;
    std::fs::write(save_path(slot), yaml)?;
    Ok(())
}

/// Load a [`SaveData`] from `{Documents}/Carbonthrone/save_{slot}.yaml`.
pub fn load_game(slot: u8) -> Result<SaveData, Box<dyn std::error::Error>> {
    let yaml = std::fs::read_to_string(save_path(slot))?;
    let data: SaveData = serde_yaml::from_str(&yaml)?;
    Ok(data)
}

/// Read all 4 save slots. Returns `None` for missing or corrupt files.
pub fn load_all_slots() -> [Option<SaveData>; 4] {
    std::array::from_fn(|i| load_game(i as u8).ok())
}
