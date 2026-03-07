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
use crate::terrain::{BattleRng, LevelMap};
use crate::travel::TravelState;
use crate::travel::arrival_chance;
use crate::zone::{CardinalDir, Zone, ZoneKind};

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
    /// Set while the player is traveling between named zones via hallways.
    pub travel: Option<TravelState>,
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
    /// Returns the `CardinalDir` of a door tile if the player moved onto one.
    pub fn try_move(&mut self, world: &mut World, dx: i32, dy: i32) -> Option<CardinalDir> {
        if self.in_dialog {
            return None;
        }
        let current = *world
            .get::<Position>(self.player_entity)
            .expect("player has Position");
        let nx = (current.x + dx).clamp(0, self.zone.cols as i32 - 1);
        let ny = (current.y + dy).clamp(0, self.zone.rows as i32 - 1);
        if self.zone.map.get(nx, ny).is_passable() && !self.npcs.iter().any(|n| n.pos == (nx, ny)) {
            *world
                .get_mut::<Position>(self.player_entity)
                .expect("player has Position") = Position::new(nx, ny);
            return self.zone.doors.get(&(nx, ny)).copied();
        }
        None
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
    /// Current loop number (1–5). Affects travel arrival probability.
    pub loop_number: u32,
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
            travel: None,
        };
        exploration.fire_trigger(Trigger::OnEnter);

        Self {
            phase: GamePhase::Exploration(exploration),
            world,
            battle: None,
            last_event: None,
            loop_number: 1,
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
        let GamePhase::Battle(exploration) =
            std::mem::replace(&mut self.phase, GamePhase::Transitioning)
        else {
            unreachable!()
        };
        // Despawn enemies; party entities persist with their current state.
        let enemies: Vec<Entity> = {
            let mut q = self.world.query::<(Entity, &Character)>();
            q.iter(&self.world)
                .filter(|(_, c)| !c.kind.is_player())
                .map(|(e, _)| e)
                .collect()
        };
        for e in enemies {
            self.world.despawn(e);
        }
        self.world.remove_resource::<LevelMap>();
        self.world.remove_resource::<BattleRng>();
        self.battle = None;
        self.last_event = None;
        self.phase = GamePhase::Exploration(exploration);
    }

    /// Begin traveling toward `destination`. Replaces the current zone with an
    /// anonymous hallway. Only callable during exploration when not already traveling.
    pub fn initiate_travel(&mut self, destination: ZoneKind, rng: &mut impl rand::Rng) {
        let GamePhase::Exploration(exploration) = &mut self.phase else {
            return;
        };
        if exploration.travel.is_some() {
            return;
        }
        // Find which direction in the current zone leads to the destination.
        let travel_dir = [
            CardinalDir::North,
            CardinalDir::South,
            CardinalDir::East,
            CardinalDir::West,
        ]
        .iter()
        .copied()
        .find(|&d| exploration.zone.connections.get(d) == Some(destination))
        .unwrap_or(CardinalDir::South);

        let depth = exploration.zone.depth;
        let origin = exploration.zone.kind;
        let player_entity = exploration.player_entity;
        exploration.travel = Some(TravelState::new(origin, destination, travel_dir));
        exploration.zone = Zone::enter_hallway(depth, travel_dir, rng);
        exploration.npcs.clear();
        // Spawn 1 tile inward from the backtrack (entry) door.
        let spawn = spawn_pos_near_door(&exploration.zone, travel_dir.opposite());
        *self
            .world
            .get_mut::<Position>(player_entity)
            .expect("player has Position") = Position::new(spawn.0, spawn.1);
    }

    /// Attempt to exit the current hallway. Rolls against [`arrival_chance`] for
    /// the current loop number.
    ///
    /// Returns `true` if the party arrived at the destination, `false` if they
    /// entered another hallway.
    pub fn exit_hallway(&mut self, rng: &mut impl rand::Rng) -> bool {
        let GamePhase::Exploration(exploration) = &mut self.phase else {
            return false;
        };
        if exploration.travel.is_none() {
            return false;
        }
        let destination = exploration.travel.as_ref().unwrap().destination;
        let travel_dir = exploration.travel.as_ref().unwrap().travel_dir;
        let depth = exploration.zone.depth;
        let player_entity = exploration.player_entity;
        let loop_number = self.loop_number;

        if rng.r#gen::<f64>() < arrival_chance(loop_number) {
            exploration.zone = Zone::enter(destination, depth, rng);
            exploration.travel = None;
            exploration.npcs.clear();
            // Spawn 1 tile inward from the entry door (faces back toward origin).
            let spawn = spawn_pos_near_door(&exploration.zone, travel_dir.opposite());
            *self
                .world
                .get_mut::<Position>(player_entity)
                .expect("player has Position") = Position::new(spawn.0, spawn.1);
            exploration.fire_trigger(Trigger::OnEnter);
            true
        } else {
            exploration.travel.as_mut().unwrap().hallways_traversed += 1;
            exploration.zone = Zone::enter_hallway(depth, travel_dir, rng);
            exploration.npcs.clear();
            // Spawn 1 tile inward from the backtrack door.
            let spawn = spawn_pos_near_door(&exploration.zone, travel_dir.opposite());
            *self
                .world
                .get_mut::<Position>(player_entity)
                .expect("player has Position") = Position::new(spawn.0, spawn.1);
            false
        }
    }

    /// Move the player by (dx, dy) during exploration.
    ///
    /// Delegates to [`ExplorationState::try_move`]. If the player lands on a
    /// door tile, travel is initiated automatically:
    /// - Named zone door → [`Self::initiate_travel`] toward the connected zone.
    /// - Hallway exit door (travel direction) → [`Self::exit_hallway`].
    /// - Hallway backtrack door (opposite direction) → [`Self::backtrack_to_origin`].
    pub fn move_player(&mut self, dx: i32, dy: i32, rng: &mut impl rand::Rng) {
        let GamePhase::Exploration(exploration) = &mut self.phase else {
            return;
        };
        let door_dir = exploration.try_move(&mut self.world, dx, dy);
        let Some(dir) = door_dir else { return };

        // Player stepped on a door — trigger travel.
        let GamePhase::Exploration(exploration) = &self.phase else {
            return;
        };
        let is_hallway = exploration.zone.kind == ZoneKind::Hallway;
        let travel_dir = exploration.travel.as_ref().map(|t| t.travel_dir);
        let destination = exploration.zone.connections.get(dir);

        if is_hallway {
            if Some(dir) == travel_dir {
                self.exit_hallway(rng);
            } else {
                self.backtrack_to_origin(rng);
            }
        } else if let Some(dest) = destination {
            self.initiate_travel(dest, rng);
        }
    }

    /// Cancel travel and return to the origin zone, spawning near the door
    /// that faces the destination (so the player can re-enter the hallway).
    pub fn backtrack_to_origin(&mut self, rng: &mut impl rand::Rng) {
        let GamePhase::Exploration(exploration) = &mut self.phase else {
            return;
        };
        if exploration.travel.is_none() {
            return;
        }
        let origin = exploration.travel.as_ref().unwrap().origin;
        let travel_dir = exploration.travel.as_ref().unwrap().travel_dir;
        let depth = exploration.zone.depth;
        let player_entity = exploration.player_entity;
        exploration.zone = Zone::enter(origin, depth, rng);
        exploration.travel = None;
        exploration.npcs.clear();
        // Spawn 1 tile inward from the door that leads toward the destination.
        let spawn = spawn_pos_near_door(&exploration.zone, travel_dir);
        *self
            .world
            .get_mut::<Position>(player_entity)
            .expect("player has Position") = Position::new(spawn.0, spawn.1);
        exploration.fire_trigger(Trigger::OnEnter);
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
    let mut rng = StdRng::seed_from_u64(rand::random::<u64>());
    for (character, pos) in zone.generate_enemies(&mut rng) {
        let stats = character.stats.clone();
        let hp = character.current_hp;
        world.spawn((character, stats, Health::new(hp), ActionPoints::new(4), pos));
    }

    let battle_rng = StdRng::seed_from_u64(rand::random::<u64>());
    world.insert_resource(zone.map.clone());
    world.insert_resource(BattleRng(battle_rng));
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns the position 1 tile inward from the door on `door_dir` side of `zone`.
/// Used to place the player just inside a zone after transitioning.
fn spawn_pos_near_door(zone: &Zone, door_dir: CardinalDir) -> (i32, i32) {
    let door_pos = zone
        .doors
        .iter()
        .find(|entry| *entry.1 == door_dir)
        .map(|entry| *entry.0);

    let Some((x, y)) = door_pos else {
        return (1, 1);
    };

    let nx = match door_dir {
        CardinalDir::East => (x - 1).max(0),
        CardinalDir::West => (x + 1).min(zone.cols as i32 - 1),
        _ => x,
    };
    let ny = match door_dir {
        CardinalDir::North => (y + 1).min(zone.rows as i32 - 1),
        CardinalDir::South => (y - 1).max(0),
        _ => y,
    };
    (nx, ny)
}
