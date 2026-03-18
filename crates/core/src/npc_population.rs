use std::collections::HashSet;

use crate::character::{CharacterKind, loop_aggression};
use crate::dialog::DialogFlags;
use crate::game::NpcData;
use crate::zone::ZoneKind;

/// Return the Yarn node slug for non-companion NPCs, or `None` for companion kinds.
///
/// Companion NPCs (Orin, Doss, Kaleo) share the zone-level interact node that
/// already exists in the Yarn files (`loop{N}_{location}_on_interact`).
/// Non-companion NPCs use a per-character slug so multi-NPC zones can have
/// independent dialog trees.
pub fn npc_interact_slug(kind: &CharacterKind) -> Option<&'static str> {
    match kind {
        CharacterKind::SalvageOperative => Some("salvage_operative"),
        CharacterKind::GunForHire => Some("gun_for_hire"),
        CharacterKind::StationGuard => Some("station_guard"),
        _ => None,
    }
}

/// Return the NPCs that should populate `kind` given the current `loop_number` and flag state.
///
/// Placement follows `docs/world.md` zone NPC lists.  Aggression is derived from
/// `loop_aggression` so the caller can colour-code and gate interaction accordingly.
pub fn zone_npcs(
    kind: ZoneKind,
    cols: u32,
    rows: u32,
    loop_number: u32,
    flags: &HashSet<String>,
) -> Vec<NpcData> {
    let cx = (cols as i32 / 2).max(1);
    let cy = (rows as i32 / 2).max(1);
    let ci = cols as i32;
    let ri = rows as i32;

    // Clamp a position to the valid grid interior (1 tile from each edge).
    let clamp = |x: i32, y: i32| -> (i32, i32) { (x.max(1).min(ci - 2), y.max(1).min(ri - 2)) };

    // Build one NpcData, deriving aggression from loop_aggression.
    let npc = |npc_kind: CharacterKind, pos: (i32, i32), name: &'static str, glyph: char| {
        let aggression = loop_aggression(&npc_kind, loop_number);
        NpcData {
            pos,
            name,
            glyph,
            kind: npc_kind,
            aggression,
        }
    };

    match kind {
        // Research Wing: Salvage Operative present loops 1-2 (Friendly).
        ZoneKind::ResearchWing => {
            if loop_number <= 2 {
                vec![npc(
                    CharacterKind::SalvageOperative,
                    clamp(cx + 2, cy),
                    "Salvage Operative",
                    'S',
                )]
            } else {
                vec![]
            }
        }

        // Command Deck: Dr. Orin (unless already a companion).
        ZoneKind::CommandDeck => {
            if !flags.contains("companion_orin") {
                vec![npc(CharacterKind::Orin, clamp(cx, cy - 2), "Dr. Orin", 'O')]
            } else {
                vec![]
            }
        }

        // Military Annex: Recruiter Doss (unless already a companion).
        ZoneKind::MilitaryAnnex => {
            if !flags.contains("companion_doss") {
                vec![npc(
                    CharacterKind::Doss,
                    clamp(cx - 2, cy),
                    "Recruiter Doss",
                    'D',
                )]
            } else {
                vec![]
            }
        }

        // Systems Core: Unit Kaleo (loop 2+, unless recruited as companion).
        ZoneKind::SystemsCore => {
            if loop_number >= 2 && !flags.contains("kaleo_recruited") {
                vec![npc(
                    CharacterKind::Kaleo,
                    clamp(cx, cy + 2),
                    "Unit Kaleo",
                    'K',
                )]
            } else {
                vec![]
            }
        }

        // Medical Bay: Salvage Operative (loops 1-4) + Station Guard (loops 1-2).
        ZoneKind::MedicalBay => {
            let mut npcs = vec![npc(
                CharacterKind::SalvageOperative,
                clamp(cx + 2, cy),
                "Salvage Operative",
                'S',
            )];
            if loop_number <= 2 {
                npcs.push(npc(
                    CharacterKind::StationGuard,
                    clamp(cx - 2, cy + 1),
                    "Station Guard",
                    'G',
                ));
            }
            npcs
        }

        // Docking Bay: Gun-for-Hire (loops 1-4; Doss flips them in loop 5).
        ZoneKind::DockingBay => {
            if loop_number <= 4 {
                vec![npc(
                    CharacterKind::GunForHire,
                    clamp(cx, cy - 2),
                    "Gun-for-Hire",
                    'H',
                )]
            } else {
                vec![]
            }
        }

        // Station Exterior: Salvage Operative in loop 3 only (last one alive outside).
        ZoneKind::StationExterior => {
            if loop_number == 3 {
                vec![npc(
                    CharacterKind::SalvageOperative,
                    clamp(cx + 3, cy + 3),
                    "Salvage Operative",
                    'S',
                )]
            } else {
                vec![]
            }
        }

        // Relay Array, Excavation Site, Hallway: no NPCs.
        _ => vec![],
    }
}

/// Derive the active companion name from the current flag state.
pub fn derive_companion(flags: &DialogFlags) -> Option<String> {
    if flags.is_flag_set("companion_orin") {
        Some("orin".to_string())
    } else if flags.is_flag_set("companion_doss") {
        Some("doss".to_string())
    } else if flags.is_flag_set("kaleo_recruited") {
        Some("kaleo".to_string())
    } else {
        None
    }
}
