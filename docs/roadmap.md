# Carbonthrone — Roadmap Plan

## Context

Core combat/exploration loops are functional. Story endings, companion dialog, scripted encounters, and NPC aggression are implemented. Remaining work falls into three categories: Depth & Progression (temporal mechanics, loop-aware zone state), Polish & Presentation (visuals, audio, animations, UI), and Advanced Systems (armor layering, shields, equipment).

---


## Depth & Progression

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

## Polish & Presentation

### Menus & UI Typography Polish

**Why**: Menus are functional but use only default Bevy fonts. Better fonts, hover transitions, and visual hierarchy improve first impressions.
**Files**: `crates/gui/src/ui/main_menu.rs`, `crates/gui/src/ui/pause_menu.rs`, `crates/gui/src/ui/mod.rs`
**Task**: Load a custom monospace/sci-fi TTF font (add to `assets/fonts/`). Add button hover color transition (interpolate background color on `Interaction::Hovered`). Polish main menu with a subtle background image or animated gradient. Ensure consistent font sizing hierarchy across all UI panels.

---

### Environment Textures & Materials

**Why**: All tiles are solid-color Cuboids — functional for gameplay but visually primitive.
**Files**: `crates/gui/src/tile_mesh.rs`
**Task**: Replace `StandardMaterial` solid colors with textured materials (or at minimum, add slight roughness/emissive variation). Load tile textures from `assets/textures/`. Distinguish tile types visually: grid lines for open floor, concrete/rubble texture for obstacles, glowing cyan strip for doors. Keep geometry simple (Cuboids stay), upgrade materials only.

---

### Character Visual Upgrade

**Why**: Characters are solid-color cubes; factions are only distinguishable by color. Better silhouettes improve readability and identity.
**Files**: `crates/gui/src/character_visuals.rs`
**Task**: Differentiate character geometry beyond same-sized cubes: player characters slightly taller, larger enemies (AbyssalBrute, CombatFrame) wider, small enemies (MoonCrawler, MaintenanceDrone) shorter. Alternatively load sprite billboards from `assets/sprites/`. Add a "selected unit" ring indicator mesh under the active combatant.

---

### Lighting & Ambient Polish

**Why**: Single harsh directional light with no ambient gives flat, washed-out look in some zones.
**Files**: `crates/gui/src/camera.rs`
**Task**: Add a low-intensity ambient light to complement the directional sun. Add per-zone tint (e.g., blue-cold for Exterior zones, warm orange for Engineering). If Bevy bloom is available, add subtle bloom to emissive door/effect tiles. Consider fog/depth haze for atmosphere.

---

### Combat Animations & Hit Feedback

**Why**: Only movement is animated. Abilities fire and resolve with no visual feedback beyond text log.
**Files**: `crates/gui/src/character_visuals.rs`, `crates/gui/src/sync.rs`
**Task**: Add a brief damage flash (character mesh color → red → original over ~0.2s) when HP decreases. Add floating damage numbers that rise and fade. Add a screen shake system (`ScreenShake` resource) triggered by heavy attacks/explosions. Extend `CharacterMoveAnim` to support ease-in/ease-out.

---

### Particle & Ability Visual Effects

**Why**: Abilities currently have no visual distinction — melee and ranged look identical.
**Files**: New `crates/gui/src/effects.rs`, `crates/gui/src/character_visuals.rs`
**Task**: Implement a lightweight particle emitter (Bevy `Mesh2d` or sprite-based). Trigger effect on ability use: melee = impact burst at target, ranged = projectile arc, heal = rising green sparks, drain-AP = blue swirl. Death = dissolve (fade out mesh over 0.5s). Keep effects short-lived (< 1 second) to not block gameplay.

---

### Audio System

**Why**: Zero audio exists. Sound is the largest missing experiential layer.
**Files**: New `crates/gui/src/audio.rs`, `crates/gui/src/main.rs`
**Task**: Add `bevy_audio` (already in Bevy) plugin. Define `AudioEvent` enum (StartMusic, PlaySfx, StopMusic). Add `assets/audio/music/` and `assets/audio/sfx/` dirs. Implement: background music per AppState (menu theme, exploration ambient, battle track), SFX for: footstep on move, weapon hit, ability cast, death, UI click. Use looping background tracks and one-shot SFX.

---

### Intro Title Sequence & Ending Screen Polish

**Why**: The game begins directly at a static menu and ends on a plain text screen — no cinematic framing.
**Files**: `crates/gui/src/state.rs`, `crates/gui/src/ui/ending.rs`, `crates/gui/src/main.rs`
**Task**: Add `AppState::Intro` that plays before `MainMenu`: fade in "C A R B O N T H R O N E" title text with a 3–5 second hold, then fade to main menu. For the ending screen: add fade-in text reveal per line, add subtle looping ambient audio, replace plain "press any key" with a styled prompt. Optionally add a unique background image per ending.

---

## Advanced Systems

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
