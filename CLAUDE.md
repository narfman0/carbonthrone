# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Carbonthrone is a Rust RPG strategy game. The player assembles a party of up to 5 characters to battle enemies. The current phase is gameplay logic with unit tests; UI/graphics come later.

## Commands

```bash
cargo build          # compile
cargo test --jobs 2  # run all tests
cargo test <name>    # run a single test by name (substring match)
cargo run            # run the game
cargo clippy         # lint
cargo fmt            # format (run on any modified .rs files after changes)
```

## Architecture

All gameplay logic lives in `src/` as library modules, declared in `main.rs`:

- **`stats.rs`** — `Stats` struct (max_hp, attack, defense, speed, magic) with per-class base values and per-level growth. `CharacterClass` is imported here to drive class-specific values.
- **`character.rs`** — `Character` (name, class, level, stats, current_hp, xp) with damage/heal/xp methods and automatic level-up.
- **`party.rs`** — `Party` holds up to `MAX_PARTY_SIZE` (5) `Character`s; enforces the cap; tracks wipe state.
- **`combat.rs`** — Pure functions for damage math (`calc_damage`, `calc_magic_damage`) and speed-based turn ordering (`turn_order`).
- **`enemy.rs`** — `EnemyKind` enum (Goblin, Skeleton, Orc, Troll, DarkMage, Dragon), `Enemy` struct with level-scaled stats and xp reward. `Enemy::new(kind, level)` applies base stats plus per-level growth.

After modifying any `.rs` files, run `cargo fmt` on them before finishing.

Tests live in `tests/` as integration tests (one file per module). `src/lib.rs` owns the module tree and makes them importable; `src/main.rs` is the binary entry point.

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
