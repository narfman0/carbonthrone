# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Carbonthrone is a Rust RPG strategy game. The player assembles a party of up to 5 characters to battle enemies. The current phase is gameplay logic with unit tests; UI/graphics come later.

## Commands

```bash
cargo build          # compile
cargo test --jobs 2  # run all tests
cargo test --jobs 2 <name>    # run a single test by name (substring match)
cargo run            # run the game
cargo clippy         # lint
cargo fmt            # format (run on any modified .rs files after changes)
```

## Architecture

All gameplay logic lives in `src/` as library modules. The engine uses **Bevy ECS** (`World`, `Entity`, `Component`, `Resource`) for all runtime state. `src/lib.rs` owns the module tree; `src/main.rs` is the binary entry point. Tests live in `tests/` as integration tests (one file per module).

### Core entity components (Bevy `Component`)

- **`stats.rs`** — `Stats` (max_hp, attack, defense, speed, magic) with per-`CharacterKind` base values and `level_up()` growth.
- **`character.rs`** — `Character` (kind, level, current_hp, aggression) + `CharacterKind` enum covering all 4 player characters and 16 NPC/enemy types grouped by faction (The Constancy, Drifters, Automata, Abyssal Fauna, Station Personnel). `Aggression` controls whether a `CharacterKind` fights or is friendly. `Character::new_character(kind, level)` builds a fully-statted entity.
- **`health.rs`** — `Health` component (current/max HP) with `take_damage`, `heal`, `is_alive`.
- **`action_points.rs`** — `ActionPoints` component (current/max AP); `refresh_ap_system` Bevy system restores AP to max at turn start.
- **`experience.rs`** — `Experience` component; `level_up_system` Bevy system applies pending level-ups and syncs `Stats`/`Health`.
- **`position.rs`** — `Position` component (integer x/y grid coords).

### Combat

- **`combat.rs`** — Pure math (`calc_damage`, `calc_hit_chance`, `roll_hit`, `turn_order`), `BattleStep` state machine that drives full combat rounds, and `TurnEvent` log entries. References `turn.rs` and `player_input.rs` for action execution.
- **`turn.rs`** — `Action` enum (Move, UseAbility, Pass) and `apply_action` function that executes one action on the Bevy `World`. Enforces AP cost, melee range (Chebyshev ≤ 1), and obstacle blocking.
- **`player_input.rs`** — `PlayerActionChoice` (richer wrapper with hit%, damage preview, cover info) and `available_player_actions` which enumerates all valid moves for a player combatant given current AP, terrain, and enemy positions.
- **`ability.rs`** — `Ability` struct, `AbilityKind` (Melee/Ranged/Utility), `AbilityEffect` (BonusDamage, ArmorPiercing, ArmorPiercingStrike, Heal, DrainAP, GrantAP). Per-character ability tables via `character_abilities(kind)` and `available_abilities(kind, level)`. `CharacterAbilities` is a Bevy component pairing an entity to its ability set.

### World & traversal

- **`terrain.rs`** — `Tile` (Open/Obstacle/Door), `CoverLevel` (None/Partial/Full), `LevelMap` (Bevy `Resource`; 2-D grid with precomputed directional cover), `BattleRng` resource, and `generate_map` procedural generator.
- **`zone.rs`** — `ZoneKind` (9 named zones + Hallway), `Zone` (generated terrain map + door positions + encounter roll), `CardinalDir`, `ZoneConnections`, `SurpriseState`. `Zone::enter` and `Zone::enter_hallway` produce randomised layouts. `ZoneKind::combat_enemy_pool(loop_number)` returns the loop-filtered enemy roster.
- **`travel.rs`** — `TravelState` (origin, destination, travel_dir, hallways_traversed) and `arrival_chance(loop_number)` — probability of reaching a destination when exiting a hallway (decreases each loop).

### Game session & persistence

- **`game.rs`** — `GameSession` (top-level owner of `World`, `GamePhase`, `BattleStep`) and `ExplorationState` (player entity, NPC list, dialog engine, current zone, travel state, scene display state). Drives phase transitions (`transition_to_battle`, `transition_to_exploration`), `move_player`, `initiate_travel`, `exit_hallway`, `backtrack_to_origin`, `reset_loop`, and save/load. `zone_npcs` and `sync_companion` are helpers here.
- **`dialog.rs`** — `DialogEngine` (YAML-driven scene state machine), `Scene`, `DialogLine`, `Choice`, `Trigger` (OnEnter/OnCombatEnd/OnInteract/OnChoice). Loads per-loop YAML from `data/loops/`; evaluates companion + flag requirements; exposes flag import/export for saves.
- **`save.rs`** — `SaveData` struct (loop_number, flags, active_companion, current_zone, party_kinds, party_hp); `save_game`/`load_game` serialize to `save.yaml`.

## Design Documents

Game design vision lives in `docs/`; machine-readable data (YAML) lives in `data/`. Consult these when implementing or designing new systems:

- **`docs/narrative.md`** — Story concept, central conceit, and branching endings; references characters.md and loops/ for details
- **`docs/armor_and_shields.md`** — Layered armor system (ablative/reactive/thermal lining), directional shields, frequency tuning, shared squad shield bubbles, bleed/overcharge mechanics
- **`docs/weapons_and_abilities.md`** — Temporal weapon abilities (Displacement, Rewind, Stasis, Acceleration, Entropic Rounds, Echo Strike) and the Temporal Flux resource system
- **`docs/characters.md`** — Player character and companion profiles: classes, hidden arcs, companion dialog effects (Researcher, Dr. Orin, Doss, Kaleo)
- **`docs/loops/`** — One file per loop (`loop1.md`–`loop5.md`); station state, opening scene, NPC behavioral tells, discovery opportunities. `index.md` has the overview and party composition table.
- **`data/loops/`** — Companion YAML scripts (`loop1.yaml`–`loop5.yaml`); machine-readable dialog scenes with flags, triggers, branching choices, and companion conditions.
- **`docs/world.md`** — Zone map and layout: 9 zones (6 interior, 3 exterior), room counts, tile sizes, cardinal connections, encounters, and NPCs per zone.
- **`docs/npcs.md`** — Enemy factions (Drifters, Automata, Abyssal Fauna, Station Personnel), variants, aggression states, and loop behavior.
