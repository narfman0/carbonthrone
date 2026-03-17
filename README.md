# Carbonthrone

A Rust turn-based tactical RPG set aboard a degrading space station caught in a temporal loop.

## Overview

You are a temporal researcher aboard **The Meridian** — a deep-future science station built into the surface of a moon orbiting a dying star. Your team has been experimenting on a captured **Temporal Anomaly**. Something goes wrong. The loop activates. The station resets.

But each reset corrupts the loop further. The station degrades, the timeline frays, and reality becomes less stable. The weapons and armor your team developed to study the anomaly are now the only tools that work in destabilized spacetime.

Assemble a small party, explore the station across up to five loops, and piece together who triggered the loop and why — before the timeline collapses entirely.

## Features

- **Turn-based tactical combat** on grid maps with cover, action points, and ability combos
- **Five-loop story structure** — the station changes each loop, NPCs degrade and slip, and new truths become accessible
- **Companion system** — choose a starting companion; each unlocks different story paths
- **Multiple endings** — determined by which companion truths you uncover and which version of events you believe
- **Two frontends** — a Bevy graphical GUI and a terminal CLI
- **Procedural zone maps** with scripted encounter placements

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)

### Build & Run

```bash
# Graphical GUI (recommended)
cargo run -p carbonthrone-gui

# Terminal / CLI
cargo run -p carbonthrone

# Release build (no dev inspector)
cargo build --release --no-default-features -p carbonthrone-gui
```

The GUI includes a developer inspector overlay (toggle with backtick) enabled by default. Use `--no-default-features` to strip it for release-like builds.

### Developer Commands

```bash
cargo build          # compile all crates
cargo test --jobs 2  # run all tests
cargo clippy         # lint
cargo fmt            # format
```

## Architecture

Carbonthrone is a **Cargo workspace** with three crates:

| Crate | Path | Role |
|---|---|---|
| `carbonthrone` | `crates/core/` | Core gameplay library — all logic, ECS components, systems, and data |
| `carbonthrone-cli` | `crates/cli/` | Terminal/CLI binary frontend |
| `carbonthrone-gui` | `crates/gui/` | Bevy graphical GUI binary frontend |

All gameplay logic lives in `crates/core/`. The engine uses **Bevy ECS** for all runtime state. Game design documents live in `docs/`; machine-readable data (YAML dialog scripts) lives in `crates/core/data/`.

## Documentation

Design documents are in `docs/`:

- [`docs/narrative.md`](docs/narrative.md) — story concept, loop structure, and endings
- [`docs/characters.md`](docs/characters.md) — player character and companion profiles
- [`docs/world.md`](docs/world.md) — zone map and layout
- [`docs/npcs.md`](docs/npcs.md) — enemy factions and loop behavior
- [`docs/weapons_and_abilities.md`](docs/weapons_and_abilities.md) — temporal ability system
- [`docs/armor_and_shields.md`](docs/armor_and_shields.md) — layered armor and shield mechanics
- [`docs/roadmap.md`](docs/roadmap.md) — planned features and implementation status
