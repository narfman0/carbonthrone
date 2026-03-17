# Carbonthrone — Roadmap Plan

## Context

Core combat/exploration loops are functional. Story endings, companion dialog, scripted encounters, and NPC aggression are implemented. Intro/outro sequences, screen shake, and HP polish are done. bevy_yarnspinner hot-reload support (`file_watcher`/`asset_processor` dev features) is enabled. Remaining work falls into three categories: Depth & Progression (temporal mechanics, loop-aware zone state), Polish & Presentation (visuals, audio, animations, UI), and Advanced Systems (armor layering, shields, equipment).

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

### Character Portraits in Dialog

**Why**: The dialog overlay shows speaker names and text but no visual identity — all speakers look the same.
**Files**: `crates/gui/src/ui/dialog.rs`, `assets/portraits/`
**Task**: Load a portrait image per `CharacterKind` from `assets/portraits/` (e.g., `sable.png`, `researcher.png`). In the dialog UI panel, display the active speaker's portrait to the left of the dialog text box. Swap portrait on speaker change. A silhouette fallback is acceptable for characters without a dedicated portrait asset.

---

### Environment Textures & Materials

**Why**: All tiles are solid-color Cuboids — functional for gameplay but visually primitive.
**Files**: `crates/gui/src/tile_mesh.rs`
**Task**: Replace `StandardMaterial` solid colors with textured materials (or at minimum, add slight roughness/emissive variation). Load tile textures from `assets/textures/`. Distinguish tile types visually: grid lines for open floor, concrete/rubble texture for obstacles, glowing cyan strip for doors. Keep geometry simple (Cuboids stay), upgrade materials only.

---

### Animated Characters

**Why**: Characters are static meshes with no idle or action animations, making the world feel lifeless.
**Files**: `crates/gui/src/character_visuals.rs`
**Task**: Add a skeletal or transform-based animation system for character entities. Implement idle animations (subtle bob/sway), walk cycle during `CharacterMoveAnim`, death, and attack/ability cast poses triggered on action execution. Can use Bevy's built-in animation graph (`AnimationPlayer`) if sprite sheets or GLTF assets are available, or procedural transform animation as a fallback.

---

### Lighting & Ambient Polish

**Why**: Single harsh directional light with no ambient gives flat, washed-out look in some zones.
**Files**: `crates/gui/src/camera.rs`
**Task**: Add a low-intensity ambient light to complement the directional sun. Add per-zone tint (e.g., blue-cold for Exterior zones, warm orange for Engineering). If Bevy bloom is available, add subtle bloom to emissive door/effect tiles. Consider fog/depth haze for atmosphere.

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

### Animated Characters

**Why**: Characters are static meshes with no idle or action animations, making the world feel lifeless.
**Files**: `crates/gui/src/character_visuals.rs`
**Task**: Add a skeletal or transform-based animation system for character entities. Implement idle animations (subtle bob/sway), walk cycle during `CharacterMoveAnim`, death, and attack/ability cast poses triggered on action execution. Can use Bevy's built-in animation graph (`AnimationPlayer`) if sprite sheets or GLTF assets are available, or procedural transform animation as a fallback.

---

### Animated Characters

**Why**: Characters are static meshes with no idle or action animations, making the world feel lifeless.
**Files**: `crates/gui/src/character_visuals.rs`
**Task**: Add a skeletal or transform-based animation system for character entities. Implement idle animations (subtle bob/sway), walk cycle during `CharacterMoveAnim`, death, and attack/ability cast poses triggered on action execution. Can use Bevy's built-in animation graph (`AnimationPlayer`) if sprite sheets or GLTF assets are available, or procedural transform animation as a fallback.

---

### Animated Characters

**Why**: Characters are static meshes with no idle or action animations, making the world feel lifeless.
**Files**: `crates/gui/src/character_visuals.rs`
**Task**: Add a skeletal or transform-based animation system for character entities. Implement idle animations (subtle bob/sway), walk cycle during `CharacterMoveAnim`, death, and attack/ability cast poses triggered on action execution. Can use Bevy's built-in animation graph (`AnimationPlayer`) if sprite sheets or GLTF assets are available, or procedural transform animation as a fallback.

---
