# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Carbonthrone is a Rust RPG strategy game. The player assembles a party of up to 5 characters to battle enemies. The current phase is gameplay logic with unit tests; UI/graphics come later.

## Commands

```bash
cargo build          # compile
cargo test           # run all tests
cargo test <name>    # run a single test by name (substring match)
cargo run            # run the game
cargo clippy         # lint
```

## Architecture

All gameplay logic lives in `src/` as library modules, declared in `main.rs`:

- **`stats.rs`** — `Stats` struct (max_hp, attack, defense, speed, magic) with per-class base values and per-level growth. `CharacterClass` is imported here to drive class-specific values.
- **`character.rs`** — `Character` (name, class, level, stats, current_hp, xp) with damage/heal/xp methods and automatic level-up. Implements `Combatant`.
- **`party.rs`** — `Party` holds up to `MAX_PARTY_SIZE` (5) `Character`s; enforces the cap; tracks wipe state.
- **`combat.rs`** — Pure functions for damage math (`calc_damage`, `calc_magic_damage`) and speed-based turn ordering (`turn_order`).
- **`combatant.rs`** — `Combatant` trait: common interface for anything in combat (name, hp, stats accessors, `take_damage`, `is_alive`). Implemented by both `Character` and `Enemy`.
- **`enemy.rs`** — `EnemyKind` enum (Goblin, Skeleton, Orc, Troll, DarkMage, Dragon), `Enemy` struct with level-scaled stats and xp reward. `Enemy::new(kind, level)` applies base stats plus per-level growth. Implements `Combatant`.

Tests live in `tests/` as integration tests (one file per module). `src/lib.rs` owns the module tree and makes them importable; `src/main.rs` is the binary entry point.

## Design Documents

Game design vision lives in `docs/`. Consult these when implementing or designing new systems:

- **`docs/narrative.md`** — Story concept, central conceit, and branching endings; references characters.md and loops.md for details
- **`docs/armor_and_shields.md`** — Layered armor system (ablative/reactive/thermal lining), directional shields, frequency tuning, shared squad shield bubbles, bleed/overcharge mechanics
- **`docs/weapons_and_abilities.md`** — Temporal weapon abilities (Displacement, Rewind, Stasis, Acceleration, Entropic Rounds, Echo Strike) and the Temporal Flux resource system
- **`docs/characters.md`** — Player character and companion profiles: classes, hidden arcs, companion dialog effects (Researcher, Dr. Orin, Doss, Kaleo)
- **`docs/loops.md`** — Loop-by-loop station state, opening scene, NPC behavioral tells, discovery opportunities, and party composition
