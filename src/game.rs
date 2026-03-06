use bevy::prelude::*;
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::action_points::ActionPoints;
use crate::character::{Character, CharacterKind};
use crate::combat::{BattleStep, TurnEvent};
use crate::dialog::{DialogEngine, Trigger};
use crate::experience::Experience;
use crate::health::Health;
use crate::position::Position;
use crate::terrain::{BattleRng, generate_map};
use crate::zone::ZoneKind;

// ── Game phase ────────────────────────────────────────────────────────────────

pub enum GamePhase {
    Exploration(ExplorationState),
    Battle(ExplorationState),
}

// ── Exploration state ─────────────────────────────────────────────────────────

pub struct NpcData {
    pub pos: (i32, i32),
    pub name: &'static str,
    pub glyph: char,
}

pub struct ExplorationState {
    pub player_pos: (i32, i32),
    pub npcs: Vec<NpcData>,
    pub dialog: DialogEngine,
    pub zone_kind: ZoneKind,
    /// Lines in the active scene as (speaker, text).
    pub scene_lines: Vec<(String, String)>,
    /// Choice texts in the active scene (empty when no choices).
    pub scene_choices: Vec<String>,
    /// Index of the currently displayed line.
    pub line_index: usize,
    /// Index of the highlighted choice (only meaningful at choice screen).
    pub choice_index: usize,
    /// Whether dialog is currently displayed.
    pub in_dialog: bool,
}

impl ExplorationState {
    pub fn new() -> Self {
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
            zone_kind: ZoneKind::CommandDeck,
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
    pub fn fire_trigger(&mut self, trigger: Trigger) {
        if let Some(scene) = self.dialog.trigger(&trigger, self.zone_kind.location_id()) {
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
    pub fn at_choice_screen(&self) -> bool {
        self.in_dialog
            && self.line_index + 1 >= self.scene_lines.len()
            && !self.scene_choices.is_empty()
    }

    /// Advance one dialog line. Returns `true` when the dialog closes.
    pub fn advance_dialog(&mut self) -> bool {
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
    pub fn select_choice(&mut self) {
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

    /// Try to move the player by (dx, dy). Blocked by NPCs and map edges.
    pub fn try_move(&mut self, dx: i32, dy: i32) {
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
    pub fn adjacent_to_npc(&self) -> bool {
        let (px, py) = self.player_pos;
        self.npcs.iter().any(|n| {
            let (nx, ny) = n.pos;
            (px - nx).abs() + (py - ny).abs() == 1
        })
    }
}

impl Default for ExplorationState {
    fn default() -> Self {
        Self::new()
    }
}

// ── Game session ──────────────────────────────────────────────────────────────

/// Owns all mutable game state. Drive it by calling methods; render from the
/// public fields. No I/O or rendering happens here.
pub struct GameSession {
    pub phase: GamePhase,
    pub world: World,
    pub battle: Option<BattleStep>,
    pub last_event: Option<TurnEvent>,
}

impl GameSession {
    pub fn new() -> Self {
        Self {
            phase: GamePhase::Exploration(ExplorationState::new()),
            world: World::new(),
            battle: None,
            last_event: None,
        }
    }

    /// Transition from exploration into a fresh battle.
    pub fn transition_to_battle(&mut self) {
        let GamePhase::Exploration(_) = &self.phase else {
            return;
        };
        let GamePhase::Exploration(exploration) =
            std::mem::replace(&mut self.phase, GamePhase::Battle(ExplorationState::new()))
        else {
            unreachable!()
        };
        setup_battle(&mut self.world, exploration.zone_kind);
        self.battle = Some(BattleStep::new(&mut self.world));
        self.last_event = None;
        self.phase = GamePhase::Battle(exploration);
    }

    /// Advance the battle by one step. Returns a reference to the new event.
    pub fn step_battle(&mut self) -> &TurnEvent {
        let event = self.battle.as_mut().unwrap().step(&mut self.world);
        self.last_event = Some(event);
        self.last_event.as_ref().unwrap()
    }

    /// Transition from battle back to exploration.
    pub fn transition_to_exploration(&mut self) {
        let GamePhase::Battle(_) = &self.phase else {
            return;
        };
        let GamePhase::Battle(exploration) = std::mem::replace(
            &mut self.phase,
            GamePhase::Exploration(ExplorationState::new()),
        ) else {
            unreachable!()
        };
        self.world = World::new();
        self.battle = None;
        self.last_event = None;
        self.phase = GamePhase::Exploration(exploration);
    }

    /// True when a battle outcome has been decided.
    pub fn battle_over(&self) -> bool {
        self.last_event
            .as_ref()
            .and_then(|e| e.outcome.as_ref())
            .is_some()
    }
}

impl Default for GameSession {
    fn default() -> Self {
        Self::new()
    }
}

// ── World setup ───────────────────────────────────────────────────────────────

pub fn setup_battle(world: &mut World, zone_kind: ZoneKind) {
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
    let mut reserved: Vec<(i32, i32)> = player_positions.to_vec();
    reserved.extend_from_slice(enemy_positions);
    let map = generate_map(10, 10, zone_kind, &reserved, &mut rng);
    world.insert_resource(map);
    world.insert_resource(BattleRng(rng));
}
