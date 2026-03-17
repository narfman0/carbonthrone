# Carbonthrone — Roadmap Plan

## Context

After reviewing the design docs and codebase (~60% complete), this plan identifies what remains to be implemented. Core combat/exploration loops are functional. Major gaps are: companion/party mechanics, dialog enforcement, story endings, and advanced systems (temporal flux, armor layering).

---

## HIGH PRIORITY — Core Gameplay Gaps---

### Dynamic NPC Placement and interaction by Zone and loop

**Why**: `zone_npcs()` in `game.rs` hardcodes 4 NPCs at map center, ignoring `docs/world.md` zone-specific NPC lists.
**Files**:

- `crates/core/src/game.rs` — `zone_npcs()`
- `crates/core/src/zone.rs` — `ZoneKind`
  **Task**: Map each `ZoneKind` to its faction NPCs from `docs/world.md`. Respect `loop_aggression(loop_number)` from `character.rs`. Observe and support narrative and dialog from loop yaml.

---

## MEDIUM PRIORITY — Depth & Progression

### Temporal Flux Resource System

**Why**: Core thematic mechanic described in `docs/weapons_and_abilities.md`; currently absent from code.
**Files**:

- New: `crates/core/src/temporal_flux.rs` — `TemporalFlux` Bevy Resource
- `crates/core/src/ability.rs` — add `GeneratesFlux` / `ConsumesFlux` to `AbilityEffect`
- `crates/core/src/combat.rs` — track flux changes per turn
- `crates/gui/src/ui/hud.rs` or `combat.rs` — display zone flux level
  **Task**: Add a zone-level `TemporalFlux` counter (0–100). Temporal abilities generate/consume flux. At high flux (>75): hit chance penalty to all. At 100: Temporal Collapse event (random debuff).

---

### Advanced Temporal Abilities

**Why**: Docs describe 6 temporal abilities; only basic analogs (Stasis=DrainAP, Rewind=Heal) are implemented.
**Files**: `crates/core/src/ability.rs` — `AbilityEffect` enum and Researcher ability table
**New effects to add**:

- `Displacement { delay_rounds: u8 }` — damage lands N rounds later
- `Acceleration { bonus_ap: i32 }` — grants extra AP but ages target (debuff after)
- `EntropicRounds` — damage bypasses armor, ignores cover
- `EchoStrike` — copies last used ability of target

---

### Loop-Based Zone State Changes

**Why**: `docs/world.md` and loop docs describe zones changing between loops (collapsed sections open, relay array signals change).
**Files**:

- `crates/core/src/zone.rs` — `Zone::enter()` signature
- `crates/core/src/game.rs` — pass `loop_number` to zone generation
  **Task**: `Zone::enter()` should accept `loop_number` and conditionally open/block tiles (e.g., Excavation collapsed section opens in loop 3+).

---

### Aesthetic details

**Why**: player enjoyment and engagement

What: need consistent and good looking: menus, environment, meshes, textures, lighting, effects, animations, audio, intro, outro
**Task**: Update all aesthetic systems

---

## LOW PRIORITY — Advanced Systems

### Layered Armor System

**Why**: Rich mechanic in `docs/armor_and_shields.md`; substantial design work already done.
**Files**: New `crates/core/src/armor.rs`, modify `health.rs` and `turn.rs`
**Task**: Add `ArmorLayers { ablative: u8, reactive: u8, thermal: u8 }` component. Damage routing: physical hits ablative first, explosive hits reactive, energy hits thermal. Simple version: each layer absorbs flat damage before HP.

---

### Directional Shields

**Why**: Described in `docs/armor_and_shields.md`; requires facing/direction concept.
**Dependency**: Requires armor system (#9) first.
**Task**: Add `ShieldFacing` component and `hunker_down` action that regenerates shield and locks facing.

---

### Equipment Degradation & Consumables

**Why**: Described in `docs/weapons_and_abilities.md` (repair kits, weapon heat).
**Dependency**: Armor system (#9) first; inventory system needed.
**Task**: Track armor integrity per layer; add repair consumable to loot tables.
