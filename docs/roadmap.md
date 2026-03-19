# Carbonthrone — Roadmap Plan

## Context

Core combat/exploration loops are functional. Story endings, companion dialog, scripted encounters, and NPC aggression are implemented. Intro/outro sequences, screen shake, and HP polish are done. The Temporal Flux system and all five advanced temporal abilities (Displacement, Acceleration, Entropic Rounds, Echo Strike, Temporal Recall) are implemented. Remaining work falls into two categories: Polish & Presentation (visuals, audio, animations, UI) and Advanced Systems (armor layering, shields, equipment).

---

## ⚠️ High Priority

### Enemy AI Overhaul

**Why**: The current AI wastes AP on meaningless back-and-forth movement, making combat trivially easy and strategically uninteresting. Enemies should feel like credible tactical threats — pursuing, flanking, and using abilities intelligently rather than shuffling in place until their turn ends.

**Files**: `crates/core/src/combat.rs`, `crates/core/src/turn.rs`, `crates/core/src/player_input.rs`

**Task**: Replace the current movement heuristic with a proper goal-oriented AI loop:

1. **Pursue & close distance**: If an enemy has a melee ability and the nearest player is out of reach, spend AP moving toward them along a valid path (no aimless oscillation). Track the last position and avoid re-visiting it on the same turn.
2. **Use abilities purposefully**: After closing to attack range, spend remaining AP on the highest-value ability available — prefer damaging abilities over Pass. If flux is high, prefer lower-flux abilities.
3. **Ranged enemies hold distance**: Ranged attackers should try to maintain optimal attack range (2–4 tiles), retreating if a player closes to melee range.
4. **Opportunity costing**: If AP is insufficient to both move *and* attack, the AI should evaluate whether moving is worthwhile. If attacking from current position is possible, attack first.
5. **Ability variety**: Enemies with utility abilities (DrainAP, Displacement) should use them opportunistically — e.g., drain a low-AP player who just moved, or displace a player out of cover.
6. **Pass as a last resort only**: The AI should only call Pass when it genuinely has no valid moves or abilities left — never as a filler action.

**Acceptance criteria**: In a standard combat encounter, no enemy should spend more than one AP on movement if they can attack instead, and no enemy should move to the same tile they just vacated on the same turn.

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
