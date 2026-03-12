//! Scripted combat encounters.
//!
//! A [`ScriptedEncounter`] defines fixed combatant placements for a specific
//! zone + loop combination, overriding the zone's random enemy generation.
//! Use [`scripted_encounter_for`] to look up the encounter definition.
//!
//! Allies defined in a scripted encounter are spawned as temporary combatants
//! that fight alongside the player and are despawned at combat end just like
//! enemies.  They are tagged with the [`ScriptedAlly`] Bevy component so the
//! rest of the engine can distinguish them from persistent party members.
//!
//! The [`ScriptedFirstAction`] Bevy component, when attached to an entity,
//! causes the AI to attempt the named ability on its first combat turn before
//! falling back to normal tactical reasoning.

use bevy::prelude::Component;

use crate::character::{Aggression, Character, CharacterKind};
use crate::position::Position;
use crate::zone::ZoneKind;

// ── Spawn location ────────────────────────────────────────────────────────────

/// Where on the map to place a scripted combatant.
///
/// Positions are resolved at encounter setup time once the map dimensions are
/// known, so they remain valid across randomly-sized zone layouts.
#[derive(Debug, Clone)]
pub enum SpawnLocation {
    /// Near the centre of the north (top) edge — low y values.
    NorthWall,
    /// Near the centre of the south (bottom) edge — high y values.
    SouthWall,
    /// Near the centre of the east (right) edge — high x values.
    EastWall,
    /// Near the centre of the west (left) edge — low x values.
    WestWall,
    /// Geometric centre of the map.
    Center,
    /// Arbitrary position expressed as fractions of (cols, rows), clamped to
    /// valid bounds.  `(0.0, 0.0)` is the north-west corner; `(1.0, 1.0)` the
    /// south-east corner.
    Fraction(f32, f32),
}

impl SpawnLocation {
    /// Convert to an absolute grid coordinate given the map dimensions.
    pub fn to_grid(&self, cols: u32, rows: u32) -> (i32, i32) {
        let c = cols as i32;
        let r = rows as i32;
        match self {
            SpawnLocation::NorthWall => (c / 2, 1.max(0)),
            SpawnLocation::SouthWall => (c / 2, (r - 2).max(0)),
            SpawnLocation::EastWall => ((c - 2).max(0), r / 2),
            SpawnLocation::WestWall => (1.max(0), r / 2),
            SpawnLocation::Center => (c / 2, r / 2),
            SpawnLocation::Fraction(fx, fy) => (
                ((cols as f32 * fx) as i32).clamp(0, c - 1),
                ((rows as f32 * fy) as i32).clamp(0, r - 1),
            ),
        }
    }
}

// ── Placement spec ────────────────────────────────────────────────────────────

/// Fixed placement for one combatant within a scripted encounter.
#[derive(Debug, Clone)]
pub struct CombatantPlacement {
    pub kind: CharacterKind,
    pub level: u32,
    pub location: SpawnLocation,
    /// Override the default aggression for this kind; `None` keeps the default.
    pub aggression: Option<Aggression>,
    /// If `Some`, the AI will attempt the ability with this name on the
    /// entity's very first combat turn before falling back to normal tactics.
    pub first_ability: Option<&'static str>,
}

impl CombatantPlacement {
    /// Construct a [`Character`] and [`Position`] from this placement spec.
    pub fn to_character_and_pos(&self, cols: u32, rows: u32) -> (Character, Position) {
        let mut ch = Character::new_character(self.kind.clone(), self.level);
        if let Some(agg) = self.aggression.clone() {
            ch.aggression = agg;
        }
        let (x, y) = self.location.to_grid(cols, rows);
        (ch, Position::new(x, y))
    }
}

// ── ScriptedEncounter ─────────────────────────────────────────────────────────

/// A fully scripted combat encounter that overrides random enemy generation.
///
/// Enemies are placed at fixed positions rather than spawned randomly.
/// Allies fight alongside the player for the duration of combat and are
/// despawned together with enemies when combat ends.
#[derive(Debug, Clone)]
pub struct ScriptedEncounter {
    /// Unique identifier, used in tests and future save/load support.
    pub id: &'static str,
    /// Enemy combatants placed at fixed positions.
    pub enemies: Vec<CombatantPlacement>,
    /// Temporary ally combatants that fight on the player's side.
    pub allies: Vec<CombatantPlacement>,
}

// ── Bevy components ───────────────────────────────────────────────────────────

/// Marker component for temporary scripted allies.
///
/// Entities tagged with this are spawned for a single scripted encounter and
/// despawned when combat ends, even if their `CharacterKind::is_player()` is
/// `true`.
#[derive(Debug, Clone, Component)]
pub struct ScriptedAlly;

/// Marker for a permanent party companion entity.
///
/// Spawned at session start (or on recruitment) and persists through exploration
/// and battle. Distinct from [`ScriptedAlly`] (one-off temporary AI combatants).
#[derive(Debug, Clone, Component)]
pub struct PartyCompanion;

/// Bevy ECS component: instructs the AI to use the named ability on this
/// entity's first combat turn before switching to normal tactical AI.
///
/// Once the scripted turn has been taken, `executed` is set to `true` and
/// subsequent turns use normal AI logic.
#[derive(Debug, Clone, Component)]
pub struct ScriptedFirstAction {
    /// Name of the ability to use (matched against [`crate::ability::Ability::name`]).
    pub ability_name: &'static str,
    /// Set to `true` once the scripted action has been executed.
    pub executed: bool,
}

// ── Encounter registry ────────────────────────────────────────────────────────

/// Return the scripted encounter for the given zone and loop number, or `None`
/// when the encounter for this zone should be generated randomly.
pub fn scripted_encounter_for(zone: ZoneKind, loop_number: u32) -> Option<ScriptedEncounter> {
    match (zone, loop_number) {
        // Loop 1, ResearchWing: tutorial opening fight.
        // Constancy Zealots and a Purifier breach from the north wall;
        // Orin and Doss fight alongside the player at the south end.
        (ZoneKind::ResearchWing, 1) => Some(tutorial_opening()),

        // Loop 4+, MilitaryAnnex: the Archon appears with a ShockTrooper
        // bodyguard.  Pre-combat dialog is driven by the YAML scene
        // `loop4_archon_precombat`.
        (ZoneKind::MilitaryAnnex, 4..) => Some(archon_encounter()),

        _ => None,
    }
}

// ── Concrete encounter definitions ────────────────────────────────────────────

/// Loop 1 tutorial — Constancy breach in Room 7-C (ResearchWing).
///
/// From `docs/loops/loop1.md`:
/// - Orin drops a stabilization field (heals) while calling positions
/// - Doss tanks a charge with a heavy strike
/// - Kaleo is in the terminals — not spawned physically in this fight
/// - Zealots and a Purifier breach from the opposite side of the room (north)
fn tutorial_opening() -> ScriptedEncounter {
    ScriptedEncounter {
        id: "tutorial_opening",
        enemies: vec![
            // Breach point: north wall, left-centre
            CombatantPlacement {
                kind: CharacterKind::Zealot,
                level: 1,
                location: SpawnLocation::Fraction(0.3, 0.08),
                aggression: None,
                first_ability: Some("Zealot Strike"),
            },
            // Breach point: north wall, right-centre
            CombatantPlacement {
                kind: CharacterKind::Zealot,
                level: 1,
                location: SpawnLocation::Fraction(0.7, 0.08),
                aggression: None,
                first_ability: None,
            },
            // Ranged support from the breach doorway
            CombatantPlacement {
                kind: CharacterKind::Purifier,
                level: 1,
                location: SpawnLocation::Fraction(0.5, 0.05),
                aggression: None,
                first_ability: Some("Purifying Round"),
            },
        ],
        allies: vec![
            // Orin: shield projector running, calls positions — heals first
            CombatantPlacement {
                kind: CharacterKind::Orin,
                level: 1,
                location: SpawnLocation::Fraction(0.35, 0.85),
                aggression: Some(Aggression::Friendly),
                first_ability: Some("Heal"),
            },
            // Doss: back to a pillar, kinetic rifle up — power strike first
            CombatantPlacement {
                kind: CharacterKind::Doss,
                level: 1,
                location: SpawnLocation::Fraction(0.65, 0.85),
                aggression: Some(Aggression::Friendly),
                first_ability: Some("Power Strike"),
            },
        ],
    }
}

/// Loop 4+ MilitaryAnnex — Archon encounter.
///
/// From `docs/loops/loop4.md`: The Archon pauses before the fight and
/// acknowledges the player across loops.  Pre-combat dialog is handled by the
/// YAML scene `loop4_archon_precombat`.  The Archon spawns at the far wall
/// with a ShockTrooper bodyguard on the right flank.
fn archon_encounter() -> ScriptedEncounter {
    ScriptedEncounter {
        id: "archon_encounter",
        enemies: vec![
            // Archon at the north wall, facing the player's entry point
            CombatantPlacement {
                kind: CharacterKind::Archon,
                level: 4,
                location: SpawnLocation::NorthWall,
                aggression: None,
                first_ability: Some("Temporal Suppression"),
            },
            // ShockTrooper flanking right
            CombatantPlacement {
                kind: CharacterKind::ShockTrooper,
                level: 3,
                location: SpawnLocation::Fraction(0.8, 0.2),
                aggression: None,
                first_ability: None,
            },
        ],
        allies: vec![],
    }
}
