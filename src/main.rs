use std::collections::HashMap;
use std::io::{self, Write};

use bevy::prelude::*;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{self, ClearType},
};

use carbonthrone::character::{Aggression, Character};
use carbonthrone::combat::{BattleOutcome, BattleStep, Turn, TurnAction, TurnEvent};
use carbonthrone::game::{ExplorationState, GamePhase, GameSession};
use carbonthrone::health::Health;
use carbonthrone::position::Position;
use carbonthrone::stats::Stats;
use carbonthrone::terrain::{CoverLevel, LevelMap};

fn main() {
    let mut session = GameSession::new();

    let mut stdout = io::stdout();
    terminal::enable_raw_mode().expect("enable raw mode");

    loop {
        execute!(
            stdout,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )
        .unwrap();

        match &session.phase {
            GamePhase::Exploration(state) => {
                let frame = render_exploration(state);
                write!(stdout, "{}", frame).unwrap();
            }
            GamePhase::Battle(_) => {
                let frame = render(
                    &mut session.world,
                    session.battle.as_ref().unwrap(),
                    session.last_event.as_ref(),
                );
                write!(stdout, "{}", frame).unwrap();
            }
        }
        stdout.flush().unwrap();

        // ── Input ────────────────────────────────────────────────────────────
        loop {
            let Ok(ev) = event::read() else { continue };
            let Event::Key(k) = ev else { continue };
            if k.kind != KeyEventKind::Press {
                continue;
            }

            // Global quit
            match k.code {
                KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                    terminal::disable_raw_mode().unwrap();
                    return;
                }
                KeyCode::Char('q') | KeyCode::Esc => {
                    terminal::disable_raw_mode().unwrap();
                    return;
                }
                _ => {}
            }

            match &mut session.phase {
                // ── Exploration input ─────────────────────────────────────
                GamePhase::Exploration(state) => {
                    if state.in_dialog {
                        match k.code {
                            KeyCode::Char(' ') | KeyCode::Enter => {
                                if state.at_choice_screen() {
                                    state.select_choice();
                                } else {
                                    state.advance_dialog();
                                }
                                break;
                            }
                            KeyCode::Up => {
                                if state.at_choice_screen() && state.choice_index > 0 {
                                    state.choice_index -= 1;
                                }
                                break;
                            }
                            KeyCode::Down => {
                                if state.at_choice_screen()
                                    && state.choice_index + 1 < state.scene_choices.len()
                                {
                                    state.choice_index += 1;
                                }
                                break;
                            }
                            _ => {}
                        }
                    } else {
                        match k.code {
                            KeyCode::Up | KeyCode::Char('w') => {
                                state.try_move(0, -1);
                                break;
                            }
                            KeyCode::Down | KeyCode::Char('s') => {
                                state.try_move(0, 1);
                                break;
                            }
                            KeyCode::Left | KeyCode::Char('a') => {
                                state.try_move(-1, 0);
                                break;
                            }
                            KeyCode::Right | KeyCode::Char('d') => {
                                state.try_move(1, 0);
                                break;
                            }
                            KeyCode::Char('e') => {
                                use carbonthrone::dialog::Trigger;
                                if state.adjacent_to_npc() {
                                    state.fire_trigger(Trigger::OnInteract);
                                }
                                break;
                            }
                            KeyCode::Char('b') => {
                                session.transition_to_battle();
                                break;
                            }
                            _ => {}
                        }
                    }
                }

                // ── Battle input ──────────────────────────────────────────
                GamePhase::Battle(_) => {
                    let battle_over = session.battle_over();
                    match k.code {
                        KeyCode::Char(' ') if !battle_over => {
                            session.step_battle();
                            break;
                        }
                        _ if battle_over => {
                            session.transition_to_exploration();
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

// ── Exploration rendering ─────────────────────────────────────────────────────

const WIDTH: usize = 58;

fn render_exploration(state: &ExplorationState) -> String {
    let mut out = String::new();
    let bar = "=".repeat(WIDTH);

    out += &format!("{}\r\n", bar);
    out += &format!("{:^width$}\r\n", "C A R B O N T H R O N E", width = WIDTH);
    out += &format!("{}\r\n", bar);
    out += "\r\n";

    out += &format!("  Zone: {}\r\n", state.zone.kind.display_name());
    out += "\r\n";

    let grid_cols = state.zone.cols as i32;
    let grid_rows = state.zone.rows as i32;
    let (px, py) = (state.pos.x, state.pos.y);

    for y in 0..grid_rows {
        out += "  ";
        for x in 0..grid_cols {
            let ch = if (x, y) == (px, py) {
                '@'
            } else if let Some(npc) = state.npcs.iter().find(|n| n.pos == (x, y)) {
                npc.glyph
            } else {
                state.zone.map.display_glyph(x, y)
            };
            out += &format!("{} ", ch);
        }
        out += "\r\n";
    }
    out += "\r\n";

    // NPC legend
    for npc in &state.npcs {
        out += &format!("  {} = {}\r\n", npc.glyph, npc.name);
    }
    out += "\r\n";

    // Dialog area
    out += &format!("  {}\r\n", "-".repeat(WIDTH - 2));
    if state.in_dialog && !state.scene_lines.is_empty() {
        let (speaker, text) = &state.scene_lines[state.line_index];
        out += &format!("  [{speaker}]: {text}\r\n");
        out += "\r\n";

        if state.at_choice_screen() {
            for (i, choice) in state.scene_choices.iter().enumerate() {
                let cursor = if i == state.choice_index { ">" } else { " " };
                out += &format!("  {cursor} {choice}\r\n");
            }
            out += "\r\n";
        }
    } else {
        out += "  (explore the zone)\r\n";
    }

    // Controls footer
    out += "\r\n";
    out += &format!("{}\r\n", bar);
    let controls_str = if state.in_dialog {
        if state.at_choice_screen() {
            "[UP/DOWN] choose  [ENTER] confirm"
        } else {
            "[SPACE] continue"
        }
    } else if state.adjacent_to_npc() {
        "[WASD/Arrows] move  [E] talk  [B] battle  [Q] quit"
    } else {
        "[WASD/Arrows] move  [B] battle  [Q] quit"
    };
    out += &format!("{controls_str}\r\n");
    out += &format!("{}\r\n", bar);

    out
}

// ── Battle rendering ──────────────────────────────────────────────────────────

fn render(world: &mut World, battle: &BattleStep, last: Option<&TurnEvent>) -> String {
    let mut out = String::new();
    let bar = "=".repeat(WIDTH);

    out += &format!("{}\r\n", bar);
    out += &format!("{:^width$}\r\n", "C A R B O N T H R O N E", width = WIDTH);
    out += &format!("{}\r\n", bar);
    out += "\r\n";

    let side_label = match battle.turn {
        Turn::Player => "Player Side",
        Turn::Enemy => "Enemy Side",
    };
    if let Some(next) = battle.next_actor() {
        let name = entity_name(world, next);
        out += &format!(
            "  Round {}  |  {}  |  Next: {}\r\n",
            battle.round, side_label, name
        );
    } else {
        out += &format!("  Round {}  |  {}\r\n", battle.round, side_label);
    }
    out += "\r\n";

    let players = player_entities(world);
    let enemies = enemy_entities(world);
    let rows = players.len().max(enemies.len());

    out += &format!("  {:<27}  {}\r\n", "PLAYERS", "ENEMIES");
    out += &format!("  {}\r\n", "-".repeat(WIDTH - 2));
    for i in 0..rows {
        let left = players
            .get(i)
            .map(|&(e, cur, max)| combatant_line(world, e, cur, max))
            .unwrap_or_default();
        let right = enemies
            .get(i)
            .map(|&(e, cur, max)| combatant_line(world, e, cur, max))
            .unwrap_or_default();
        out += &format!("  {:<27}  {}\r\n", left, right);
    }
    out += "\r\n";

    out += &format!("  {}\r\n", map_string(world));
    out += "\r\n";

    if let Some(map) = world.get_resource::<LevelMap>() {
        out += &format!("  Zone: {}\r\n", map.zone_kind.display_name());
    }
    out += "  . open  # obstacle  c partial-cover  C full-cover\r\n";
    out += "\r\n";

    out += &format!("  {}\r\n", "-".repeat(WIDTH - 2));
    if let Some(event) = last {
        match event.actor {
            Some(actor) => {
                let name = entity_name(world, actor);
                let side_str = match event.turn {
                    Turn::Player => "Player",
                    Turn::Enemy => "Enemy",
                };
                out += &format!("  -- {}'s turn ({}) --\r\n", name, side_str);
                if event.actions.is_empty() {
                    out += "  > (no actions)\r\n";
                }
                for action in &event.actions {
                    match action {
                        TurnAction::Attack {
                            target,
                            damage,
                            hit,
                            cover,
                        } => {
                            let tname = entity_name(world, *target);
                            let cover_str = match cover {
                                CoverLevel::None => "",
                                CoverLevel::Partial => " [partial cover]",
                                CoverLevel::Full => " [full cover]",
                            };
                            if *hit {
                                out += &format!(
                                    "  > {} attacks {} for {} dmg{}\r\n",
                                    name, tname, damage, cover_str
                                );
                            } else {
                                out += &format!(
                                    "  > {} attacks {} -- MISS{}\r\n",
                                    name, tname, cover_str
                                );
                            }
                        }
                        TurnAction::Move { to } => {
                            out += &format!("  > {} moves to ({}, {})\r\n", name, to.x, to.y);
                        }
                        TurnAction::UseAbility {
                            ability_name,
                            target,
                            value,
                            hit,
                        } => {
                            let target_str = match target {
                                Some(t) => format!(" on {}", entity_name(world, *t)),
                                None => String::new(),
                            };
                            if *hit {
                                out += &format!(
                                    "  > {} uses {}{} for {}\r\n",
                                    name, ability_name, target_str, value
                                );
                            } else {
                                out += &format!(
                                    "  > {} uses {}{} -- MISS\r\n",
                                    name, ability_name, target_str
                                );
                            }
                        }
                    }
                }
            }
            None => out += "  -- (side change) --\r\n",
        }

        if let Some(ref outcome) = event.outcome {
            out += "\r\n";
            let msg = match outcome {
                BattleOutcome::PlayerVictory => "*** VICTORY! The party prevails! ***",
                BattleOutcome::PlayerDefeated => "*** DEFEAT! The party has fallen. ***",
                BattleOutcome::Draw => "*** DRAW! The battle ends in stalemate. ***",
            };
            out += &format!("  {}\r\n", msg);
        }
    } else {
        out += "  (press SPACE to begin)\r\n";
    }

    out += "\r\n";
    out += &format!("{}\r\n", bar);
    if last.and_then(|e| e.outcome.as_ref()).is_some() {
        out += &format!("  {:<width$}\r\n", "[any key] quit", width = WIDTH - 2);
    } else {
        out += &format!("  {:<27}  {}\r\n", "[SPACE] next turn", "[Q / ESC] quit");
    }
    out += &format!("{}\r\n", bar);

    out
}

fn player_entities(world: &mut World) -> Vec<(Entity, i32, i32)> {
    let mut q = world.query::<(Entity, &Character, &Health, &Stats)>();
    let mut v: Vec<(Entity, i32, i32, i32)> = q
        .iter(world)
        .filter(|(_, c, _, _)| c.kind.is_player())
        .map(|(e, _, h, stats)| (e, h.current, h.max, stats.speed))
        .collect();
    v.sort_by(|a, b| b.3.cmp(&a.3));
    v.into_iter()
        .map(|(e, cur, max, _)| (e, cur, max))
        .collect()
}

fn enemy_entities(world: &mut World) -> Vec<(Entity, i32, i32)> {
    let mut q = world.query::<(Entity, &Character, &Health, &Stats)>();
    let mut v: Vec<(Entity, i32, i32, i32)> = q
        .iter(world)
        .filter(|(_, c, _, _)| c.aggression != Aggression::Friendly)
        .filter(|(_, c, _, _)| !c.kind.is_player())
        .map(|(e, _, h, stats)| (e, h.current, h.max, stats.speed))
        .collect();
    v.sort_by(|a, b| b.3.cmp(&a.3));
    v.into_iter()
        .map(|(e, cur, max, _)| (e, cur, max))
        .collect()
}

fn combatant_line(world: &World, entity: Entity, current: i32, max: i32) -> String {
    let name = entity_name(world, entity);
    let bar = hp_bar(current, max);
    let dead = if current <= 0 { " (dead)" } else { "" };
    format!("{:<8} {} {:>3}/{:<3}{}", name, bar, current, max, dead)
}

fn hp_bar(current: i32, max: i32) -> String {
    const W: usize = 8;
    if max <= 0 {
        return format!("[{}]", "-".repeat(W));
    }
    let filled = ((current.max(0) as usize) * W / max as usize).min(W);
    format!("[{}{}]", "#".repeat(filled), "-".repeat(W - filled))
}

fn entity_name(world: &World, entity: Entity) -> String {
    if let Some(c) = world.get::<Character>(entity) {
        return c.name.clone();
    }
    format!("#{}", entity.index())
}

fn map_string(world: &mut World) -> String {
    let terrain = world.get_resource::<LevelMap>().map(|map| {
        let max_x = map.cols as i32 - 1;
        let max_y = map.rows as i32 - 1;
        let cells: HashMap<(i32, i32), char> = (0..=max_y)
            .flat_map(|y| (0..=max_x).map(move |x| ((x, y), map.display_glyph(x, y))))
            .collect();
        (max_x, max_y, cells)
    });

    let (mut max_x, mut max_y, mut cells) = terrain.unwrap_or_else(|| (9, 1, HashMap::new()));

    let mut qc = world.query::<(&Position, &Character, &Health)>();
    let char_glyphs: Vec<(i32, i32, char)> = qc
        .iter(world)
        .map(|(pos, c, h)| {
            let glyph = if !h.is_alive() {
                'x'
            } else if c.kind.is_player() {
                'P'
            } else {
                'E'
            };
            (pos.x, pos.y, glyph)
        })
        .collect();
    for (x, y, ch) in char_glyphs {
        max_x = max_x.max(x);
        max_y = max_y.max(y);
        cells.insert((x, y), ch);
    }

    let mut rows = Vec::new();
    for y in 0..=max_y {
        let row: String = (0..=max_x)
            .map(|x| cells.get(&(x, y)).copied().unwrap_or('.'))
            .map(|c| format!("{} ", c))
            .collect::<String>()
            .trim_end()
            .to_string();
        rows.push(row);
    }
    rows.join("\r\n  ")
}
