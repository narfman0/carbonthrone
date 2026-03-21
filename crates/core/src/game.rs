use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::action_points::{ActionPoints, ap_for_speed};
use crate::character::{Aggression, Character, CharacterKind};
use crate::combat::{BattleStep, TurnEvent};
use crate::dialog::DialogFlags;
use crate::experience::Experience;
use crate::health::Health;
use crate::npc_population::{derive_companion, npc_interact_slug};
use crate::position::Position;
use crate::save::{BattleSnapshot, CombatantRole, CombatantSnapshot, SaveData};
use crate::save_restore::restore_battle;
use crate::scripted_encounter::{
    PartyCompanion, ScriptedAlly, ScriptedFirstAction, scripted_encounter_for,
};
use crate::terrain::{BattleRng, LevelMap};
use crate::travel::TravelState;
use crate::travel::arrival_chance;
use crate::travel_system::{follow_companions, place_companions_near, spawn_pos_near_door};
use crate::world_setup::{setup_battle, setup_exploration};
use crate::zone::{CardinalDir, Zone, ZoneKind};

// Re-export for external callers (tests) that import via `carbonthrone::game::zone_npcs`.
pub use crate::npc_population::zone_npcs;

// ── Game phase ────────────────────────────────────────────────────────────────

/// Which story ending the player reached in loop 5.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EndingKind {
    /// Anomaly contained; timeline unchanged. War happens as-is.
    Default,
    /// Player helped Orin destroy the program. Orin doesn't survive.
    SableDestroyed,
    /// Player stopped Orin. Program persists; timeline preserved.
    SableStopped,
    /// Player sided with Kaleo. Anomaly redirected; loop folds harmlessly.
    KaleoCompromise,
    /// Player sided with Doss. Timeline accepted as-is.
    DossPreserved,
}

/// Result of a [`GameSession::move_player`] call.
#[derive(Debug, Clone, PartialEq)]
pub enum MoveResult {
    /// The player did not move (blocked by an obstacle or not in exploration phase).
    Blocked,
    /// The player moved to an open tile with no door.
    Moved,
    /// The player stepped onto a named-zone door and entered a hallway.
    EnteredHallway,
    /// The player exited a hallway and entered another hallway (arrival roll failed).
    ContinuedHallway,
    /// The player exited a hallway and arrived at the destination zone.
    ArrivedAtDestination,
    /// The player stepped onto the hallway backtrack door and returned to the origin zone.
    Backtracked,
}

pub enum GamePhase {
    Exploration(ExplorationState),
    Battle(ExplorationState),
    /// Placeholder used only during phase transitions; never observed externally.
    Transitioning,
    /// A story ending has been reached.
    Ended(EndingKind),
}

// ── Exploration state ─────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct NpcData {
    pub pos: (i32, i32),
    pub name: &'static str,
    pub glyph: char,
    pub kind: CharacterKind,
    pub aggression: Aggression,
}

pub struct ExplorationState {
    /// Entity for the player-controlled Researcher in the ECS world.
    pub player_entity: Entity,
    /// One entity per companion in `party[1..]`; spawned persistently at session start.
    pub companion_entities: Vec<Entity>,
    pub npcs: Vec<NpcData>,
    /// Dialog flag state for game-logic queries (NPC spawning, ending resolution, saves).
    /// Variable state inside active dialog is owned by the GUI's `DialogueRunner`.
    pub flags: DialogFlags,
    /// Which companion is currently travelling with the player ("orin", "doss", "kaleo").
    pub active_companion: Option<String>,
    pub zone: Zone,
    pub party: Vec<Character>,
    /// Whether a Yarn dialog node is currently being displayed.
    /// Set to `true` by `set_pending_dialog_node`; cleared by `end_dialog` or `dismiss_dialog`.
    pub in_dialog: bool,
    /// Yarn node name the GUI should start next frame, if any.
    /// Cleared by the GUI after calling `DialogueRunner::start_node`.
    pub pending_dialog_node: Option<String>,
    /// Set while the player is traveling between named zones via hallways.
    pub travel: Option<TravelState>,
    /// A combat encounter is waiting to start (set on zone entry when the zone
    /// rolled an encounter). Cleared when `maybe_start_battle` fires.
    pub pending_battle: bool,
    /// IDs of scripted encounters the player has already fought.
    pub fought_scripted_encounters: HashSet<String>,
    /// Last line shown per dialog node (key → (speaker, text)).
    /// Persisted so the GUI can re-display it when an NPC has nothing new to say.
    pub last_dialog_lines: HashMap<String, (String, String)>,
}

impl ExplorationState {
    /// Set the pending dialog node for the given trigger kind at the current zone.
    /// Marks `in_dialog = true` so battles and movement are blocked until the
    /// GUI advances the dialog and calls `end_dialog`.
    pub fn set_pending_dialog_node(&mut self, trigger: &str, loop_number: u32) {
        let location = self.zone.kind.location_id();
        let node = format!("loop{loop_number}_{location}_on_{trigger}");
        self.pending_dialog_node = Some(node);
        self.in_dialog = true;
    }

    /// Fire an interact trigger for the given NPC at the current location.
    ///
    /// Companion NPCs (Orin, Doss, Kaleo) route to the zone-level interact node
    /// (`loop{N}_{location}_on_interact`).  Non-companion NPCs (Salvage Operative,
    /// Gun-for-Hire, Station Guard) route to a per-NPC node so multi-NPC zones like
    /// Medical Bay can handle each character independently
    /// (`loop{N}_{location}_{slug}_on_interact`).
    pub fn fire_interact(&mut self, loop_number: u32, npc_kind: &CharacterKind) {
        let location = self.zone.kind.location_id();
        let node = match npc_interact_slug(npc_kind) {
            Some(slug) => format!("loop{loop_number}_{location}_{slug}_on_interact"),
            None => format!("loop{loop_number}_{location}_on_interact"),
        };
        self.pending_dialog_node = Some(node);
        self.in_dialog = true;
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
            follow_companions(
                world,
                self.player_entity,
                &self.companion_entities,
                &self.zone,
            );
            return self.zone.doors.get(&(nx, ny)).copied();
        }
        None
    }

    /// True when the player is adjacent (Manhattan distance 1) to any NPC.
    pub fn adjacent_to_npc(&self, world: &World) -> bool {
        self.adjacent_npc(world).is_some()
    }

    /// Return a reference to the first NPC adjacent (Manhattan distance 1) to the player,
    /// or `None` if no NPC is adjacent.
    pub fn adjacent_npc<'a>(&'a self, world: &World) -> Option<&'a NpcData> {
        let pos = world
            .get::<Position>(self.player_entity)
            .expect("player has Position");
        let (px, py) = (pos.x, pos.y);
        self.npcs.iter().find(|n| {
            let (nx, ny) = n.pos;
            (px - nx).abs() + (py - ny).abs() == 1
        })
    }
}

// ── Helper functions ─────────────────────────────────────────────────────────

/// Determine if a story ending has been reached based on flags set by Yarn dialog.
/// Returns `None` if the game should continue normally.
fn resolve_ending(flags: &DialogFlags, loop_number: u32) -> Option<EndingKind> {
    if loop_number < 5 {
        return None;
    }
    if flags.is_flag_set("ending_doss") {
        return Some(EndingKind::DossPreserved);
    }
    if flags.is_flag_set("ending_kaleo") {
        return Some(EndingKind::KaleoCompromise);
    }
    if flags.is_flag_set("orin_helped") {
        return Some(EndingKind::SableDestroyed);
    }
    if flags.is_flag_set("orin_stopped") && !flags.is_flag_set("convergence_triggered") {
        return Some(EndingKind::SableStopped);
    }
    None
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
        let (player_entity, companion_entities) = setup_exploration(&mut world, &party);

        let flags = DialogFlags::new();
        let loop_number = 1u32;

        let mut rng = StdRng::seed_from_u64(rand::random::<u64>());
        let zone = Zone::enter(ZoneKind::ResearchWing, 1, loop_number, &mut rng);
        let mut npcs = zone_npcs(zone.kind, zone.cols, zone.rows, loop_number, flags.flags());
        for npc in &mut npcs {
            npc.pos = zone.map.nearest_open_tile(npc.pos.0, npc.pos.1);
        }

        let mut exploration = ExplorationState {
            player_entity,
            companion_entities,
            npcs,
            flags,
            active_companion: None,
            zone,
            party,
            in_dialog: false,
            pending_dialog_node: None,
            travel: None,
            pending_battle: false,
            fought_scripted_encounters: HashSet::new(),
            last_dialog_lines: HashMap::new(),
        };
        exploration.set_pending_dialog_node("enter", loop_number);

        let mut session = Self {
            phase: GamePhase::Exploration(exploration),
            world,
            battle: None,
            last_event: None,
            loop_number: 1,
        };
        session.apply_pending_battle();
        // Note: maybe_start_battle is blocked until in_dialog is cleared by
        // the GUI after the opening Yarn dialog completes.
        session
    }

    /// Transition from exploration into a fresh battle.
    ///
    /// If a [`ScriptedEncounter`] is registered for the current zone and loop
    /// number, it overrides random enemy generation: enemies spawn at fixed
    /// positions and temporary allies are spawned alongside the player.
    pub fn transition_to_battle(&mut self) {
        let GamePhase::Exploration(_) = &self.phase else {
            return;
        };
        let GamePhase::Exploration(exploration) =
            std::mem::replace(&mut self.phase, GamePhase::Transitioning)
        else {
            unreachable!()
        };
        let script = scripted_encounter_for(exploration.zone.kind, self.loop_number);
        setup_battle(&mut self.world, &exploration.zone, script.as_ref());
        self.battle = Some(BattleStep::new(&mut self.world));
        self.last_event = None;
        self.phase = GamePhase::Battle(exploration);
    }

    /// Set `pending_battle`, forcing it true for scripted encounters not yet fought.
    fn apply_pending_battle(&mut self) {
        let GamePhase::Exploration(e) = &mut self.phase else {
            return;
        };
        e.pending_battle = e.zone.has_encounter();
        if !e.pending_battle {
            if let Some(enc) = scripted_encounter_for(e.zone.kind, self.loop_number) {
                if !e.fought_scripted_encounters.contains(enc.id) {
                    e.pending_battle = true;
                }
            }
        }
    }

    /// If a battle is pending and no dialog is currently active, start it now.
    fn maybe_start_battle(&mut self) {
        let should = match &self.phase {
            GamePhase::Exploration(e) => e.pending_battle && !e.in_dialog,
            _ => false,
        };
        if should {
            if let GamePhase::Exploration(e) = &mut self.phase {
                e.pending_battle = false;
            }
            self.transition_to_battle();
        }
    }

    /// Called by the GUI when a Yarn dialog node completes.
    ///
    /// `new_flags` are the boolean variable names (without `$`) that were set
    /// to `true` during the dialog — read from `DialogueRunner::variable_storage()`.
    /// Flags are merged into `ExplorationState::flags`, the active companion is
    /// re-derived, companions are recruited, and ending resolution is attempted.
    /// If a battle was pending, it starts automatically.
    pub fn end_dialog(&mut self, new_flags: Vec<String>) {
        {
            let GamePhase::Exploration(e) = &mut self.phase else {
                return;
            };
            e.flags.import_flags(new_flags);
            e.active_companion = derive_companion(&e.flags);
            e.in_dialog = false;
            e.pending_dialog_node = None;
        }
        self.sync_and_recruit_companions();
        let ending = if let GamePhase::Exploration(e) = &self.phase {
            resolve_ending(&e.flags, self.loop_number)
        } else {
            None
        };
        if let Some(ending) = ending {
            self.phase = GamePhase::Ended(ending);
            return;
        }
        self.maybe_start_battle();
    }

    /// Dismiss pending dialog without processing it (used by the CLI and other
    /// non-Yarn frontends to unblock game state).
    pub fn dismiss_dialog(&mut self) {
        {
            let GamePhase::Exploration(e) = &mut self.phase else {
                return;
            };
            e.in_dialog = false;
            e.pending_dialog_node = None;
        }
        self.maybe_start_battle();
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
        // Despawn temporary scripted allies (player-kind characters added just
        // for this encounter — identified by the ScriptedAlly marker).
        let scripted_allies: Vec<Entity> = {
            let mut q = self.world.query::<(Entity, &ScriptedAlly)>();
            q.iter(&self.world).map(|(e, _)| e).collect()
        };
        for e in scripted_allies {
            self.world.despawn(e);
        }
        self.world.remove_resource::<LevelMap>();
        self.world.remove_resource::<BattleRng>();
        self.battle = None;
        self.last_event = None;
        // Sync HP from ECS back to party vec so save data is accurate.
        if let Some(h) = self.world.get::<Health>(exploration.player_entity) {
            exploration.party[0].current_hp = h.current.max(1);
        }
        for (i, &entity) in exploration.companion_entities.iter().enumerate() {
            if let Some(h) = self.world.get::<Health>(entity) {
                if let Some(m) = exploration.party.get_mut(i + 1) {
                    m.current_hp = h.current.max(1);
                }
            }
        }
        if let Some(enc) = scripted_encounter_for(exploration.zone.kind, self.loop_number) {
            exploration
                .fought_scripted_encounters
                .insert(enc.id.to_string());
        }
        exploration.set_pending_dialog_node("combat_end", self.loop_number);
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
        let loop_number = self.loop_number;
        let origin = exploration.zone.kind;
        let player_entity = exploration.player_entity;
        exploration.travel = Some(TravelState::new(origin, destination, travel_dir));
        exploration.zone = Zone::enter_hallway(depth, loop_number, travel_dir, rng);
        exploration.npcs.clear();
        // Spawn 1 tile inward from the backtrack (entry) door.
        let spawn = spawn_pos_near_door(&exploration.zone, travel_dir.opposite());
        *self
            .world
            .get_mut::<Position>(player_entity)
            .expect("player has Position") = Position::new(spawn.0, spawn.1);
        place_companions_near(
            &mut self.world,
            &exploration.companion_entities,
            spawn,
            &exploration.zone.map,
        );
    }

    /// Attempt to exit the current hallway. Rolls against [`arrival_chance`] for
    /// the current loop number.
    ///
    /// Returns `true` if the party arrived at the destination, `false` if they
    /// entered another hallway.
    pub fn exit_hallway(&mut self, rng: &mut impl rand::Rng) -> bool {
        // Extract scalars in a short immutable-borrow block so the borrow is
        // dropped before we need split-field borrows below.
        let (destination, travel_dir, depth, player_entity) = {
            let GamePhase::Exploration(e) = &self.phase else {
                return false;
            };
            if e.travel.is_none() {
                return false;
            }
            let t = e.travel.as_ref().unwrap();
            (t.destination, t.travel_dir, e.zone.depth, e.player_entity)
        };
        let loop_number = self.loop_number;

        if rng.r#gen::<f64>() < arrival_chance(loop_number) {
            // All zone-setup and dialog happen inside a scoped block so the
            // borrow on self.phase is released before we call maybe_start_battle.
            let (spawn, companion_entities, zone_map) = {
                let GamePhase::Exploration(e) = &mut self.phase else {
                    unreachable!()
                };
                e.zone = Zone::enter(destination, depth, loop_number, rng);
                e.travel = None;
                e.npcs = zone_npcs(
                    e.zone.kind,
                    e.zone.cols,
                    e.zone.rows,
                    loop_number,
                    e.flags.flags(),
                );
                // Snap NPC positions to the nearest open tile.
                for npc in &mut e.npcs {
                    npc.pos = e.zone.map.nearest_open_tile(npc.pos.0, npc.pos.1);
                }
                // Spawn 1 tile inward from the entry door (faces back toward origin).
                let spawn = spawn_pos_near_door(&e.zone, travel_dir.opposite());
                e.set_pending_dialog_node("enter", loop_number);
                (spawn, e.companion_entities.clone(), e.zone.map.clone())
            }; // self.phase borrow released
            *self
                .world
                .get_mut::<Position>(player_entity)
                .expect("player has Position") = Position::new(spawn.0, spawn.1);
            place_companions_near(&mut self.world, &companion_entities, spawn, &zone_map);
            self.sync_and_recruit_companions();
            self.apply_pending_battle();
            self.maybe_start_battle();
            true
        } else {
            let (spawn, companion_entities, zone_map) = {
                let GamePhase::Exploration(e) = &mut self.phase else {
                    unreachable!()
                };
                e.travel.as_mut().unwrap().hallways_traversed += 1;
                e.zone = Zone::enter_hallway(depth, loop_number, travel_dir, rng);
                e.npcs.clear();
                // Spawn 1 tile inward from the backtrack door.
                let spawn = spawn_pos_near_door(&e.zone, travel_dir.opposite());
                (spawn, e.companion_entities.clone(), e.zone.map.clone())
            }; // self.phase borrow released
            *self
                .world
                .get_mut::<Position>(player_entity)
                .expect("player has Position") = Position::new(spawn.0, spawn.1);
            place_companions_near(&mut self.world, &companion_entities, spawn, &zone_map);
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
    pub fn move_player(&mut self, dx: i32, dy: i32, rng: &mut impl rand::Rng) -> MoveResult {
        let GamePhase::Exploration(exploration) = &mut self.phase else {
            return MoveResult::Blocked;
        };
        let door_dir = exploration.try_move(&mut self.world, dx, dy);
        let Some(dir) = door_dir else {
            return MoveResult::Moved;
        };

        // Player stepped on a door — trigger travel.
        let GamePhase::Exploration(exploration) = &self.phase else {
            return MoveResult::Moved;
        };
        let is_hallway = exploration.zone.kind == ZoneKind::Hallway;
        let travel_dir = exploration.travel.as_ref().map(|t| t.travel_dir);
        let destination = exploration.zone.connections.get(dir);

        if is_hallway {
            if Some(dir) == travel_dir {
                let arrived = self.exit_hallway(rng);
                if arrived {
                    MoveResult::ArrivedAtDestination
                } else {
                    MoveResult::ContinuedHallway
                }
            } else {
                self.backtrack_to_origin(rng);
                MoveResult::Backtracked
            }
        } else if destination.is_some() {
            self.initiate_travel(destination.unwrap(), rng);
            MoveResult::EnteredHallway
        } else {
            MoveResult::Moved
        }
    }

    /// Cancel travel and return to the origin zone, spawning near the door
    /// that faces the destination (so the player can re-enter the hallway).
    pub fn backtrack_to_origin(&mut self, rng: &mut impl rand::Rng) {
        let (origin, travel_dir, depth, player_entity) = {
            let GamePhase::Exploration(e) = &self.phase else {
                return;
            };
            if e.travel.is_none() {
                return;
            }
            let t = e.travel.as_ref().unwrap();
            (t.origin, t.travel_dir, e.zone.depth, e.player_entity)
        };
        let loop_number = self.loop_number;
        let (spawn, companion_entities, zone_map) = {
            let GamePhase::Exploration(e) = &mut self.phase else {
                unreachable!()
            };
            e.zone = Zone::enter(origin, depth, loop_number, rng);
            e.travel = None;
            e.npcs = zone_npcs(
                e.zone.kind,
                e.zone.cols,
                e.zone.rows,
                loop_number,
                e.flags.flags(),
            );
            // Snap NPC positions to the nearest open tile.
            for npc in &mut e.npcs {
                npc.pos = e.zone.map.nearest_open_tile(npc.pos.0, npc.pos.1);
            }
            // Spawn 1 tile inward from the door that leads toward the destination.
            let spawn = spawn_pos_near_door(&e.zone, travel_dir);
            e.set_pending_dialog_node("enter", loop_number);
            (spawn, e.companion_entities.clone(), e.zone.map.clone())
        }; // self.phase borrow released
        *self
            .world
            .get_mut::<Position>(player_entity)
            .expect("player has Position") = Position::new(spawn.0, spawn.1);
        place_companions_near(&mut self.world, &companion_entities, spawn, &zone_map);
        self.sync_and_recruit_companions();
        self.apply_pending_battle();
        self.maybe_start_battle();
    }

    /// Move `actor` to `next` during animated combat movement.
    /// Shared for both player and enemy animated paths.
    pub fn step_combat_tile(&mut self, actor: Entity, next: (i32, i32)) {
        if let Some(mut pos) = self.world.get_mut::<Position>(actor) {
            *pos = Position::new(next.0, next.1);
        }
    }

    /// Finalize a completed animated move path for any combatant: charge AP and end the turn if
    /// exhausted. Branches on the current battle turn to apply the correct queue-advance logic.
    pub fn finalize_character_move(&mut self, actor: Entity, ap_cost: i32, final_pos: Position) {
        use crate::combat::{Turn, TurnEvent};
        use crate::turn::TurnAction;
        let turn = self.battle.as_ref().map(|b| b.turn).unwrap_or(Turn::Player);
        let outcome = self
            .battle
            .as_mut()
            .and_then(|b| b.charge_character_move_ap(&mut self.world, ap_cost).1);
        self.last_event = Some(TurnEvent {
            actor: Some(actor),
            turn,
            actions: vec![TurnAction::Move { to: final_pos }],
            outcome,
        });
    }

    /// True when a battle outcome has been decided.
    pub fn battle_over(&self) -> bool {
        self.last_event
            .as_ref()
            .and_then(|e| e.outcome.as_ref())
            .is_some()
    }

    /// Restart the current loop after player defeat: clean up battle state and
    /// re-enter the loop start without advancing loop_number.
    pub fn restart_current_loop(&mut self) {
        if matches!(self.phase, GamePhase::Ended(_)) {
            return;
        }
        // Strip out battle state so enter_loop_start (which requires Exploration
        // phase) can run.
        if let GamePhase::Battle(_) = &self.phase {
            let GamePhase::Battle(exploration) =
                std::mem::replace(&mut self.phase, GamePhase::Transitioning)
            else {
                unreachable!()
            };
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
            let scripted_allies: Vec<Entity> = {
                let mut q = self.world.query::<(Entity, &ScriptedAlly)>();
                q.iter(&self.world).map(|(e, _)| e).collect()
            };
            for e in scripted_allies {
                self.world.despawn(e);
            }
            self.world.remove_resource::<LevelMap>();
            self.world.remove_resource::<BattleRng>();
            self.battle = None;
            self.last_event = None;
            self.phase = GamePhase::Exploration(exploration);
        }
        let mut rng = StdRng::from_entropy();
        self.enter_loop_start(&mut rng);
    }

    /// Jump to a specific loop (1–5): set loop_number, restore party HP, and
    /// restart the player in ResearchWing with the appropriate opening scene.
    pub fn goto_loop(&mut self, target: u32) {
        if matches!(self.phase, GamePhase::Ended(_)) {
            return;
        }
        self.loop_number = target.clamp(1, 5);
        let mut rng = StdRng::from_entropy();
        self.enter_loop_start(&mut rng);
    }

    /// Advance to the next loop: increment loop_number, restore party HP, and
    /// restart the player in ResearchWing with the appropriate opening scene.
    pub fn reset_loop(&mut self, rng: &mut impl rand::Rng) {
        if matches!(self.phase, GamePhase::Ended(_)) {
            return;
        }
        self.loop_number = (self.loop_number + 1).min(5);
        self.enter_loop_start(rng);
    }

    /// Shared body for `reset_loop` / `goto_loop`: restores HP, enters
    /// ResearchWing, repositions the party, and fires opening dialog.
    /// Assumes `self.loop_number` is already set to the target value.
    fn enter_loop_start(&mut self, rng: &mut impl rand::Rng) {
        let loop_number = self.loop_number;

        // Restore party HP to max.
        let mut q = self.world.query::<&mut crate::health::Health>();
        for mut h in q.iter_mut(&mut self.world) {
            h.current = h.max;
        }

        let (player_entity, companion_entities, spawn, zone_map) = {
            let GamePhase::Exploration(e) = &mut self.phase else {
                return;
            };
            // Flags are preserved across loops; Yarn handles per-loop scenes.
            e.zone = Zone::enter(ZoneKind::ResearchWing, 1, loop_number, rng);
            e.travel = None;
            e.npcs = zone_npcs(
                e.zone.kind,
                e.zone.cols,
                e.zone.rows,
                loop_number,
                e.flags.flags(),
            );
            // Snap NPC positions to the nearest open tile.
            for npc in &mut e.npcs {
                npc.pos = e.zone.map.nearest_open_tile(npc.pos.0, npc.pos.1);
            }
            e.set_pending_dialog_node("enter", loop_number);
            let spawn = e.zone.map.nearest_open_tile(1, 1);
            (
                e.player_entity,
                e.companion_entities.clone(),
                spawn,
                e.zone.map.clone(),
            )
        }; // self.phase borrow released
        *self
            .world
            .get_mut::<Position>(player_entity)
            .expect("player has Position") = Position::new(spawn.0, spawn.1);
        place_companions_near(&mut self.world, &companion_entities, spawn, &zone_map);
        self.sync_and_recruit_companions();
        self.apply_pending_battle();
        self.maybe_start_battle();
    }

    /// Capture the minimal state needed to reconstruct this session later.
    pub fn to_save_data(&mut self) -> SaveData {
        // Extract the exploration state regardless of whether we are in
        // Exploration or Battle phase. If Battle, also build a snapshot.
        // Extract the exploration state. For Battle, also capture the entity
        // identifiers we need for the snapshot (copy them out so we can release
        // the borrow on self.phase before calling build_battle_snapshot).
        let battle_info: Option<(Entity, Vec<Entity>, ZoneKind, u32, u32)> = match &self.phase {
            GamePhase::Battle(e) => Some((
                e.player_entity,
                e.companion_entities.clone(),
                e.zone.kind,
                e.zone.cols,
                e.zone.rows,
            )),
            _ => None,
        };
        let battle_snapshot = battle_info
            .as_ref()
            .map(|(p, c, zk, zc, zr)| self.build_battle_snapshot(*p, c, *zk, *zc, *zr));

        let (exploration, battle_snapshot) = match &self.phase {
            GamePhase::Exploration(e) => (e, None),
            GamePhase::Battle(e) => (e, battle_snapshot),
            GamePhase::Ended(k) => {
                return SaveData {
                    loop_number: self.loop_number,
                    flags: vec![],
                    active_companion: None,
                    current_zone: ZoneKind::ResearchWing,
                    party_kinds: vec![CharacterKind::Researcher],
                    party_hp: vec![],
                    party_levels: vec![],
                    fought_scripted_encounters: vec![],
                    ending: Some(k.clone()),
                    battle_snapshot: None,
                    last_dialog_lines: HashMap::new(),
                };
            }
            GamePhase::Transitioning => {
                return SaveData {
                    loop_number: self.loop_number,
                    flags: vec![],
                    active_companion: None,
                    current_zone: ZoneKind::ResearchWing,
                    party_kinds: vec![CharacterKind::Researcher],
                    party_hp: vec![],
                    party_levels: vec![],
                    fought_scripted_encounters: vec![],
                    ending: None,
                    battle_snapshot: None,
                    last_dialog_lines: HashMap::new(),
                };
            }
        };

        let flags = exploration.flags.export_flags();
        let active_companion = exploration.active_companion.clone();
        let current_zone = exploration.zone.kind;
        let party_kinds = exploration.party.iter().map(|c| c.kind.clone()).collect();
        let party_hp: Vec<i32> = exploration
            .party
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let entity = if i == 0 {
                    exploration.player_entity
                } else {
                    exploration
                        .companion_entities
                        .get(i - 1)
                        .copied()
                        .unwrap_or(Entity::PLACEHOLDER)
                };
                self.world
                    .get::<Health>(entity)
                    .map(|h| h.current.max(1))
                    .unwrap_or(1)
            })
            .collect();
        let party_levels: Vec<u32> = exploration
            .party
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let entity = if i == 0 {
                    exploration.player_entity
                } else {
                    exploration
                        .companion_entities
                        .get(i - 1)
                        .copied()
                        .unwrap_or(Entity::PLACEHOLDER)
                };
                self.world
                    .get::<Experience>(entity)
                    .map(|e| e.level)
                    .unwrap_or(1)
            })
            .collect();
        let fought_scripted_encounters = {
            let mut v: Vec<String> = exploration
                .fought_scripted_encounters
                .iter()
                .cloned()
                .collect();
            v.sort();
            v
        };
        let last_dialog_lines = exploration.last_dialog_lines.clone();
        SaveData {
            loop_number: self.loop_number,
            flags,
            active_companion,
            current_zone,
            party_kinds,
            party_hp,
            party_levels,
            fought_scripted_encounters,
            ending: None,
            battle_snapshot,
            last_dialog_lines,
        }
    }

    /// Build a full battle snapshot from the current battle state.
    /// `player` and `companions` are passed in by the caller after they
    /// have released the borrow on `self.phase`.
    fn build_battle_snapshot(
        &mut self,
        player: Entity,
        companions: &[Entity],
        fallback_zone: ZoneKind,
        fallback_cols: u32,
        fallback_rows: u32,
    ) -> BattleSnapshot {
        let mut combatants: Vec<CombatantSnapshot> = Vec::new();
        let mut entity_to_idx: std::collections::HashMap<Entity, usize> =
            std::collections::HashMap::new();

        // Player entity.
        if let (Some(ch), Some(h), Some(ap), Some(pos)) = (
            self.world.get::<crate::character::Character>(player),
            self.world.get::<Health>(player),
            self.world.get::<ActionPoints>(player),
            self.world.get::<Position>(player),
        ) {
            entity_to_idx.insert(player, combatants.len());
            combatants.push(CombatantSnapshot {
                role: CombatantRole::Player,
                kind: ch.kind.clone(),
                level: ch.level,
                current_hp: h.current,
                max_hp: h.max,
                current_ap: ap.current,
                max_ap: ap.max,
                x: pos.x,
                y: pos.y,
                aggression: ch.aggression.clone(),
                scripted_first_action_name: None,
                scripted_first_action_executed: false,
            });
        }

        // Companion entities.
        for &comp in companions {
            if let (Some(ch), Some(h), Some(ap), Some(pos)) = (
                self.world.get::<crate::character::Character>(comp),
                self.world.get::<Health>(comp),
                self.world.get::<ActionPoints>(comp),
                self.world.get::<Position>(comp),
            ) {
                entity_to_idx.insert(comp, combatants.len());
                combatants.push(CombatantSnapshot {
                    role: CombatantRole::Companion,
                    kind: ch.kind.clone(),
                    level: ch.level,
                    current_hp: h.current,
                    max_hp: h.max,
                    current_ap: ap.current,
                    max_ap: ap.max,
                    x: pos.x,
                    y: pos.y,
                    aggression: ch.aggression.clone(),
                    scripted_first_action_name: None,
                    scripted_first_action_executed: false,
                });
            }
        }

        // Enemies and scripted allies — all entities with Character that are
        // not the player or a companion.
        let party_entities: std::collections::HashSet<Entity> = std::iter::once(player)
            .chain(companions.iter().copied())
            .collect();
        let others: Vec<Entity> = {
            let mut q = self.world.query::<(Entity, &crate::character::Character)>();
            q.iter(&self.world)
                .filter(|(e, _)| !party_entities.contains(e))
                .map(|(e, _)| e)
                .collect()
        };

        for entity in others {
            if let (Some(ch), Some(h), Some(ap), Some(pos)) = (
                self.world.get::<crate::character::Character>(entity),
                self.world.get::<Health>(entity),
                self.world.get::<ActionPoints>(entity),
                self.world.get::<Position>(entity),
            ) {
                let is_ally = self.world.get::<ScriptedAlly>(entity).is_some();
                let (fa_name, fa_executed) =
                    if let Some(fa) = self.world.get::<ScriptedFirstAction>(entity) {
                        (Some(fa.ability_name.to_string()), fa.executed)
                    } else {
                        (None, false)
                    };
                entity_to_idx.insert(entity, combatants.len());
                combatants.push(CombatantSnapshot {
                    role: if is_ally {
                        CombatantRole::ScriptedAlly
                    } else {
                        CombatantRole::Enemy
                    },
                    kind: ch.kind.clone(),
                    level: ch.level,
                    current_hp: h.current,
                    max_hp: h.max,
                    current_ap: ap.current,
                    max_ap: ap.max,
                    x: pos.x,
                    y: pos.y,
                    aggression: ch.aggression.clone(),
                    scripted_first_action_name: fa_name,
                    scripted_first_action_executed: fa_executed,
                });
            }
        }

        // Actor queue as stable indices into the combatants vec.
        let actor_queue: Vec<usize> = self
            .battle
            .as_ref()
            .map(|b| {
                b.player_queue()
                    .iter()
                    .filter_map(|e| entity_to_idx.get(e).copied())
                    .collect()
            })
            .unwrap_or_default();

        // Map tiles.
        let (map_cols, map_rows, zone_kind, map_tiles) =
            if let Some(lm) = self.world.get_resource::<LevelMap>() {
                (lm.cols, lm.rows, lm.zone_kind, lm.non_default_tiles())
            } else {
                (fallback_cols, fallback_rows, fallback_zone, vec![])
            };

        let (round, turn) = self
            .battle
            .as_ref()
            .map(|b| (b.round, b.turn))
            .unwrap_or((1, crate::combat::Turn::Player));

        let flux = self
            .world
            .get_resource::<crate::temporal_flux::TemporalFlux>()
            .map(|f| f.flux)
            .unwrap_or(0);

        BattleSnapshot {
            round,
            turn,
            actor_queue,
            combatants,
            map_cols,
            map_rows,
            zone_kind,
            map_tiles,
            flux,
        }
    }

    /// Reconstruct a game session from previously saved data.
    pub fn from_save_data(data: SaveData, rng: &mut impl rand::Rng) -> Self {
        let party: Vec<Character> = data
            .party_kinds
            .iter()
            .zip(data.party_hp.iter().chain(std::iter::repeat(&i32::MAX)))
            .zip(data.party_levels.iter().chain(std::iter::repeat(&1u32)))
            .map(|((kind, &hp), &level)| {
                let mut ch = Character::new_character(kind.clone(), level);
                if hp != i32::MAX {
                    ch.current_hp = hp;
                }
                ch
            })
            .collect();
        let party = if party.is_empty() {
            vec![Character::new_character(CharacterKind::Researcher, 1)]
        } else {
            party
        };

        let mut world = World::new();
        let (player_entity, companion_entities) = setup_exploration(&mut world, &party);

        // Restore saved experience levels on party entities.
        let all_entities: Vec<Entity> = std::iter::once(player_entity)
            .chain(companion_entities.iter().copied())
            .collect();
        for (entity, &level) in all_entities
            .iter()
            .zip(data.party_levels.iter().chain(std::iter::repeat(&1u32)))
        {
            if let Some(mut xp) = world.get_mut::<Experience>(*entity) {
                xp.level = level;
            }
        }

        let loop_number = data.loop_number;
        let mut flags = DialogFlags::new();
        flags.import_flags(data.flags);
        let active_companion = data.active_companion.or_else(|| derive_companion(&flags));

        let zone = Zone::enter(data.current_zone, 1, loop_number, rng);
        let mut npcs = zone_npcs(zone.kind, zone.cols, zone.rows, loop_number, flags.flags());
        for npc in &mut npcs {
            npc.pos = zone.map.nearest_open_tile(npc.pos.0, npc.pos.1);
        }

        let mut exploration = ExplorationState {
            player_entity,
            companion_entities,
            npcs,
            flags,
            active_companion,
            zone,
            party,
            in_dialog: false,
            pending_dialog_node: None,
            travel: None,
            pending_battle: false,
            fought_scripted_encounters: data.fought_scripted_encounters.into_iter().collect(),
            last_dialog_lines: data.last_dialog_lines,
        };
        exploration.set_pending_dialog_node("enter", loop_number);

        let mut session = Self {
            phase: GamePhase::Exploration(exploration),
            world,
            battle: None,
            last_event: None,
            loop_number,
        };
        if let Some(snapshot) = data.battle_snapshot {
            // Restore exact battle state — skip procedural battle generation.
            restore_battle(&mut session, &snapshot);
        } else {
            session.apply_pending_battle();
            session.maybe_start_battle();
        }
        if let Some(ending) = data.ending {
            session.phase = GamePhase::Ended(ending);
        }
        session
    }
}

impl Default for GameSession {
    fn default() -> Self {
        Self::new()
    }
}

// ── Companion management ──────────────────────────────────────────────────────

impl GameSession {
    /// Spawn any newly recruited companions that are flagged but not yet in the party.
    pub fn sync_and_recruit_companions(&mut self) {
        // Step 1: collect which companion kinds need to be spawned.
        let (to_spawn, researcher_level, researcher_pos, zone_map, companion_count) = {
            let GamePhase::Exploration(e) = &self.phase else {
                return;
            };
            let researcher_pos = self
                .world
                .get::<Position>(e.player_entity)
                .copied()
                .unwrap_or(Position::new(1, 1));
            let researcher_level = e.party.first().map(|c| c.level).unwrap_or(1);
            let recruits = [
                ("companion_orin", CharacterKind::Orin),
                ("companion_doss", CharacterKind::Doss),
                ("kaleo_recruited", CharacterKind::Kaleo),
            ];
            let to_spawn: Vec<CharacterKind> = recruits
                .iter()
                .filter(|(flag, kind)| {
                    e.flags.is_flag_set(flag) && !e.party.iter().any(|c| &c.kind == kind)
                })
                .map(|(_, kind)| kind.clone())
                .collect();
            (
                to_spawn,
                researcher_level,
                researcher_pos,
                e.zone.map.clone(),
                e.companion_entities.len(),
            )
        };

        // Step 3: spawn each new companion entity and add to party.
        for (i, kind) in to_spawn.into_iter().enumerate() {
            let companion = Character::new_character(kind, researcher_level);
            let hp = companion.current_hp;
            let stats = companion.stats.clone();
            let ap_max = ap_for_speed(stats.speed);
            let offset = (companion_count + i) as i32 + 1;
            let cols = zone_map.cols as i32;
            let rows = zone_map.rows as i32;
            let cx = (researcher_pos.x + offset).clamp(0, cols - 1);
            let cy = researcher_pos.y.clamp(0, rows - 1);
            let (px, py) = zone_map.nearest_open_tile(cx, cy);
            let pos = Position::new(px, py);
            let entity = self
                .world
                .spawn((
                    companion.clone(),
                    stats,
                    Health::new(hp),
                    ActionPoints::new(ap_max),
                    Experience::new(),
                    pos,
                    PartyCompanion,
                ))
                .id();
            let GamePhase::Exploration(e) = &mut self.phase else {
                return;
            };
            e.party.push(companion);
            e.companion_entities.push(entity);
        }
    }
}
