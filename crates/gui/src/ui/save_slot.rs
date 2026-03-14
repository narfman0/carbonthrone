use carbonthrone::{character::CharacterKind, save::SaveData};

/// Format a save slot label for display in menus.
pub fn slot_label(slot: u8, data: Option<&SaveData>) -> String {
    match data {
        None => format!("Slot {}  [ Empty ]", slot + 1),
        Some(d) => {
            let researcher_level = d
                .party_kinds
                .iter()
                .position(|k| *k == CharacterKind::Researcher)
                .and_then(|i| d.party_levels.get(i).copied())
                .unwrap_or(1);
            format!(
                "Slot {}  Loop {} — {} — Researcher Lv.{}",
                slot + 1,
                d.loop_number,
                d.current_zone.display_name(),
                researcher_level
            )
        }
    }
}
