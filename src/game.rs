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
use crate::terrain::{BattleRng, Tile};
use crate::zone::{Zone, ZoneKind};

// ── Game phase ────────────────────────────────────────────────────────────────

pub enum GamePhase {
    Exploration(ExplorationState),
    Battle(ExplorationState),
    /// Placeholder used only during phase transitions; never observed externally.
    Transitioning,
}

// ── Exploration state ─────────────────────────────────────────────────────────

pub struct NpcData {
    pub pos: (i32, i32),
    pub name: &'static str,
    pub glyph: char,
}

pub struct ExplorationState {
    /// Entity for the player-controlled Researcher in the ECS world.
    pub player_entity: Entity,
    pub npcs: Vec<NpcData>,
    pub dialog: DialogEngine,
    pub zone: Zone,
    pub party: Vec<Character>,
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

    /// Fire a trigger at the current location and load the resulting scene, if any.
    pub fn fire_trigger(&mut self, trigger: Trigger) {
        if let Some(scene) = self.dialog.trigger(&trigger, self.zone.kind.location_id()) {
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
    pub fn try_move(&mut self, world: &mut World, dx: i32, dy: i32) {
        if self.in_dialog {
            return;
        }
        let current = *world
            .get::<Position>(self.player_entity)
            .expect("player has Position");
        let nx = (current.x + dx).clamp(0, self.zone.cols as i32 - 1);
        let ny = (current.y + dy).clamp(0, self.zone.rows as i32 - 1);
        if self.zone.map.get(nx, ny) == Tile::Open && !self.npcs.iter().any(|n| n.pos == (nx, ny)) {
            *world
                .get_mut::<Position>(self.player_entity)
                .expect("player has Position") = Position::new(nx, ny);
        }
    }

    /// True when the player is adjacent (Manhattan distance 1) to any NPC.
    pub fn adjacent_to_npc(&self, world: &World) -> bool {
        let pos = world
            .get::<Position>(self.player_entity)
            .expect("player has Position");
        let (px, py) = (pos.x, pos.y);
        self.npcs.iter().any(|n| {
            let (nx, ny) = n.pos;
            (px - nx).abs() + (py - ny).abs() == 1
        })
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
        let mut world = World::new();
        let party = vec![Character::new_character(CharacterKind::Researcher, 1)];
        let player_entity = setup_exploration(&mut world, &party);

        let mut dialog = DialogEngine::new();
        let yaml = include_str!("../data/loops/loop1.yaml");
        dialog.load_script(yaml).expect("load loop1.yaml");

        let mut rng = StdRng::seed_from_u64(rand::random::<u64>());
        let zone = Zone::enter(ZoneKind::CommandDeck, 1, &mut rng);

        let mut exploration = ExplorationState {
            player_entity,
            npcs: vec![NpcData {
                pos: (5, 2),
                name: "Orin",
                glyph: 'N',
            }],
            dialog,
            zone,
            party,
            scene_lines: Vec::new(),
            scene_choices: Vec::new(),
            line_index: 0,
            choice_index: 0,
            in_dialog: false,
        };
        exploration.fire_trigger(Trigger::OnEnter);

        Self {
            phase: GamePhase::Exploration(exploration),
            world,
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
            std::mem::replace(&mut self.phase, GamePhase::Transitioning)
        else {
            unreachable!()
        };
        setup_battle(&mut self.world, &exploration.zone);
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
        let GamePhase::Battle(mut exploration) =
            std::mem::replace(&mut self.phase, GamePhase::Transitioning)
        else {
            unreachable!()
        };
        self.world = World::new();
        let player_entity = setup_exploration(&mut self.world, &exploration.party);
        exploration.player_entity = player_entity;
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

/// Spawns the party into the exploration world. Returns the first party member's entity
/// (the player-controlled Researcher).
pub fn setup_exploration(world: &mut World, party: &[Character]) -> Entity {
    let ch = &party[0];
    world
        .spawn((
            ch.clone(),
            ch.stats.clone(),
            Health::new(ch.current_hp),
            ActionPoints::new(4),
            Experience::new(),
            Position::new(0, 2),
        ))
        .id()
}

/// Adds enemies and battle resources to the world. Party is already present from
/// `setup_exploration`.
pub fn setup_battle(world: &mut World, zone: &Zone) {
    for (character, pos) in &zone.enemies {
        let stats = character.stats.clone();
        let hp = character.current_hp;
        world.spawn((
            character.clone(),
            stats,
            Health::new(hp),
            ActionPoints::new(4),
            Position::new(pos.x, pos.y),
        ));
    }

    let rng = StdRng::seed_from_u64(rand::random::<u64>());
    world.insert_resource(zone.map.clone());
    world.insert_resource(BattleRng(rng));
}
