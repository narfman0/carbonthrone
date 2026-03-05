use std::collections::HashMap;
use std::io::{self, Write};

use bevy::prelude::*;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{self, ClearType},
};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use carbonthrone::action_points::ActionPoints;
use carbonthrone::character::{Aggression, Character, CharacterKind};
use carbonthrone::combat::{BattleOutcome, BattleStep, Turn, TurnAction, TurnEvent};
use carbonthrone::dialog::{DialogEngine, Trigger};
use carbonthrone::experience::Experience;
use carbonthrone::health::Health;
use carbonthrone::position::Position;
use carbonthrone::stats::Stats;
use carbonthrone::terrain::{BattleRng, CoverLevel, LevelMap, generate_map};
use carbonthrone::zone::ZoneKind;

// ── Game phase ────────────────────────────────────────────────────────────────

enum GamePhase {
    Exploration(ExplorationState),
    Battle,
}

// ── Exploration state ─────────────────────────────────────────────────────────

struct NpcData {
    pos: (i32, i32),
    name: &'static str,
    glyph: char,
}

struct ExplorationState {
    player_pos: (i32, i32),
    npcs: Vec<NpcData>,
    dialog: DialogEngine,
    location: String,
    /// Lines in the active scene as (speaker, text).
    scene_lines: Vec<(String, String)>,
    /// Choice texts in the active scene (empty when no choices).
    scene_choices: Vec<String>,
    /// Index of the currently displayed line.
    line_index: usize,
    /// Index of the highlighted choice (only meaningful at choice screen).
    choice_index: usize,
    /// Whether dialog is currently displayed.
    in_dialog: bool,
}

impl ExplorationState {
    fn new() -> Self {
        let mut dialog = DialogEngine::new();
        let yaml = include_str!("../data/loops/loop1.yaml");
        dialog.load_script(yaml).expect("load loop1.yaml");
        dialog.set_companion("orin");
        dialog.set_flag("companion_orin");

        let mut state = Self {
            player_pos: (0, 2),
            npcs: vec![NpcData {
                pos: (5, 2),
                name: "Orin",
                glyph: 'N',
            }],
            dialog,
            location: "command_deck".to_string(),
            scene_lines: Vec::new(),
            scene_choices: Vec::new(),
            line_index: 0,
            choice_index: 0,
            in_dialog: false,
        };
        state.fire_trigger(Trigger::OnEnter);
        state
    }

    /// Fire a trigger at the current location and load the resulting scene, if any.
    fn fire_trigger(&mut self, trigger: Trigger) {
        let loc = self.location.clone();
        if let Some(scene) = self.dialog.trigger(&trigger, &loc) {
            self.scene_lines = scene
                .lines
                .iter()
                .map(|l| (l.speaker.clone(), l.text.clone()))
                .collect();
            self.scene_choices = scene
                .choices
                .as_ref()
                .map(|cs| cs.iter().map(|c| c.text.clone()).collect())
                .unwrap_or_default();
            self.line_index = 0;
            self.choice_index = 0;
            self.in_dialog = !self.scene_lines.is_empty();
        }
    }

    /// True when the player is at the last line and choices are visible.
    fn at_choice_screen(&self) -> bool {
        self.in_dialog
            && self.line_index + 1 >= self.scene_lines.len()
            && !self.scene_choices.is_empty()
    }

    /// Advance one dialog line.  Returns `true` when the dialog closes.
    fn advance_dialog(&mut self) -> bool {
        if self.line_index + 1 < self.scene_lines.len() {
            self.line_index += 1;
            false
        } else if !self.scene_choices.is_empty() {
            // Stay at the choice screen — handled by select_choice().
            false
        } else {
            self.in_dialog = false;
            true
        }
    }

    /// Confirm the highlighted choice.
    fn select_choice(&mut self) {
        if let Some(scene) = self.dialog.select_choice(self.choice_index) {
            self.scene_lines = scene
                .lines
                .iter()
                .map(|l| (l.speaker.clone(), l.text.clone()))
                .collect();
            self.scene_choices = scene
                .choices
                .as_ref()
                .map(|cs| cs.iter().map(|c| c.text.clone()).collect())
                .unwrap_or_default();
            self.line_index = 0;
            self.choice_index = 0;
            self.in_dialog = !self.scene_lines.is_empty();
        } else {
            self.in_dialog = false;
        }
    }

    /// Try to move the player by (dx, dy).  Blocked by NPCs and map edges.
    fn try_move(&mut self, dx: i32, dy: i32) {
        if self.in_dialog {
            return;
        }
        let nx = (self.player_pos.0 + dx).clamp(0, 9);
        let ny = (self.player_pos.1 + dy).clamp(0, 9);
        if !self.npcs.iter().any(|n| n.pos == (nx, ny)) {
            self.player_pos = (nx, ny);
        }
    }

    /// True when the player is adjacent (Manhattan distance 1) to any NPC.
    fn adjacent_to_npc(&self) -> bool {
        let (px, py) = self.player_pos;
        self.npcs.iter().any(|n| {
            let (nx, ny) = n.pos;
            (px - nx).abs() + (py - ny).abs() == 1
        })
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let mut phase = GamePhase::Exploration(ExplorationState::new());
    let mut world = World::new();
    let mut battle: Option<BattleStep> = None;
    let mut last_event: Option<TurnEvent> = None;

    let mut stdout = io::stdout();
    terminal::enable_raw_mode().expect("enable raw mode");

    loop {
        execute!(
            stdout,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )
        .unwrap();

        match &phase {
            GamePhase::Exploration(state) => {
                let frame = render_exploration(state);
                write!(stdout, "{}", frame).unwrap();
            }
            GamePhase::Battle => {
                let b = battle.as_mut().unwrap();
                let frame = render(&mut world, b, last_event.as_ref());
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

            match &mut phase {
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
                                if state.adjacent_to_npc() {
                                    state.fire_trigger(Trigger::OnInteract);
                                }
                                break;
                            }
                            KeyCode::Char('b') => {
                                // Transition to battle
                                setup_battle(&mut world);
                                battle = Some(BattleStep::new(&mut world));
                                last_event = None;
                                phase = GamePhase::Battle;
                                break;
                            }
                            _ => {}
                        }
                    }
                }

                // ── Battle input ──────────────────────────────────────────
                GamePhase::Battle => {
                    let b = battle.as_mut().unwrap();
                    let battle_over = last_event
                        .as_ref()
                        .and_then(|e| e.outcome.as_ref())
                        .is_some();

                    match k.code {
                        KeyCode::Char(' ') if !battle_over => {
                            last_event = Some(b.step(&mut world));
                            break;
                        }
                        _ if battle_over => {
                            terminal::disable_raw_mode().unwrap();
                            return;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

// ── World setup ───────────────────────────────────────────────────────────────

fn setup_battle(world: &mut World) {
    let player_positions: &[(i32, i32)] = &[(0, 0), (0, 1)];
    let enemy_positions: &[(i32, i32)] = &[(9, 0), (9, 1)];

    for (i, pc) in [CharacterKind::Doss, CharacterKind::Researcher]
        .into_iter()
        .enumerate()
    {
        let ch = Character::new_character(pc, 1);
        let stats = ch.stats.clone();
        let hp = ch.current_hp;
        let (px, py) = player_positions[i];
        world.spawn((
            ch,
            stats,
            Health::new(hp),
            ActionPoints::new(4),
            Experience::new(),
            Position::new(px, py, 0),
        ));
    }

    for (i, (kind, level)) in [
        (CharacterKind::Scavenger, 1u32),
        (CharacterKind::DrifterBoss, 2u32),
    ]
    .into_iter()
    .enumerate()
    {
        let ch = Character::new_character(kind, level);
        let stats = ch.stats.clone();
        let hp = ch.current_hp;
        let (ex, ey) = enemy_positions[i];
        world.spawn((
            ch,
            stats,
            Health::new(hp),
            ActionPoints::new(4),
            Position::new(ex, ey, 0),
        ));
    }

    let mut rng = StdRng::seed_from_u64(rand::random::<u64>());
    let zone_kinds = [
        ZoneKind::CommandDeck,
        ZoneKind::DockingBay,
        ZoneKind::ResearchWing,
        ZoneKind::ExcavationSite,
    ];
    let zone_kind = zone_kinds[rng.gen_range(0..4)];
    let mut reserved: Vec<(i32, i32)> = player_positions.to_vec();
    reserved.extend_from_slice(enemy_positions);
    let map = generate_map(10, 10, zone_kind, &reserved, &mut rng);
    world.insert_resource(map);
    world.insert_resource(BattleRng(rng));
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

    out += &format!("  Zone: Command Deck\r\n");
    out += "\r\n";

    // Zone grid (10 x 6)
    let grid_cols = 10i32;
    let grid_rows = 6i32;
    let (px, py) = state.player_pos;

    for y in 0..grid_rows {
        out += "  ";
        for x in 0..grid_cols {
            let ch = if (x, y) == (px, py) {
                '@'
            } else if let Some(npc) = state.npcs.iter().find(|n| n.pos == (x, y)) {
                npc.glyph
            } else {
                '.'
            };
            out += &format!("{} ", ch);
        }
        out += "\r\n";
    }
    out += "\r\n";

    // NPC legend
    for npc in &state.npcs {
        let hint = if {
            let (nx, ny) = npc.pos;
            (px - nx).abs() + (py - ny).abs() == 1
        } {
            "  [E] to talk"
        } else {
            ""
        };
        out += &format!("  {} = {}{}\r\n", npc.glyph, npc.name, hint);
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
            out += "  [UP/DOWN] choose  [ENTER] confirm\r\n";
        } else {
            let remaining = state.scene_lines.len().saturating_sub(state.line_index + 1);
            if remaining > 0 {
                out += &format!("  ({remaining} line(s) remaining — SPACE to continue)\r\n");
            } else {
                out += "  [SPACE] close\r\n";
            }
        }
    } else {
        out += "  (explore the zone — move with WASD / arrow keys)\r\n";
    }

    // Controls footer
    out += "\r\n";
    out += &format!("{}\r\n", bar);
    out += &format!(
        "  {:<27}  {}\r\n",
        "[WASD/Arrows] move  [E] talk", "[B] battle  [Q] quit"
    );
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
