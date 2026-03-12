use std::collections::HashSet;

use bevy::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::action_points::{ap_for_speed, ActionPoints};
use crate::character::{Character, CharacterKind};
use crate::combat::{BattleStep, TurnEvent};
use crate::dialog::{DialogEngine, Trigger};
use crate::experience::Experience;
use crate::health::Health;
use crate::position::Position;
use crate::save::SaveData;
use crate::scripted_encounter::{
    scripted_encounter_for, PartyCompanion, ScriptedAlly, ScriptedEncounter, ScriptedFirstAction,
};
use crate::terrain::{BattleRng, LevelMap};
use crate::travel::arrival_chance;
use crate::travel::TravelState;
use crate::zone::{CardinalDir, Zone, ZoneKind};

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

pub enum GamePhase {
    Exploration(ExplorationState),
    Battle(ExplorationState),
    /// Placeholder used only during phase transitions; never observed externally.
    Transitioning,
    /// A story ending has been reached.
    Ended(EndingKind),
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
    /// One entity per companion in `party[1..]`; spawned persistently at session start.
    pub companion_entities: Vec<Entity>,
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
    /// A combat encounter is waiting to start (set on zone entry when the zone
    /// rolled an encounter). Cleared when `maybe_start_battle` fires.
    pub pending_battle: bool,
    /// IDs of scripted encounters the player has already fought.
    pub fought_scripted_encounters: HashSet<String>,
}

impl ExplorationState {
    /// Fire a trigger at the current location and load the resulting scene, if any.
    pub fn fire_trigger(&mut self, trigger: Trigger) {
        let location = self.zone.kind.location_id();
        // Collect lines inside a scoped block so the borrow on self.dialog is
        // released before we call current_available_choice_texts().
        let triggered = {
            if let Some(scene) = self.dialog.trigger(&trigger, location) {
                self.scene_lines = scene
                    .lines
                    .iter()
                    .map(|l| (l.speaker.clone(), l.text.clone()))
                    .collect();
                self.line_index = 0;
                self.choice_index = 0;
                self.in_dialog = !self.scene_lines.is_empty();
                true
            } else {
                false
            }
        }; // borrow on self.dialog released here

        if triggered {
            // Now filter choices with the post-activation flag state.
            self.scene_choices = self.dialog.current_available_choice_texts();
        } else if trigger == Trigger::OnInteract {
            if let Some(scene) = self.dialog.last_completed_interact_scene(location) {
                if let Some(last_line) = scene.lines.last() {
                    self.scene_lines = vec![(last_line.speaker.clone(), last_line.text.clone())];
                    self.scene_choices = vec![];
                    self.line_index = 0;
                    self.choice_index = 0;
                    self.in_dialog = true;
                }
            }
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
            if let Some(scene_id) = self.dialog.current_scene_id().map(str::to_string) {
                self.dialog.mark_scene_complete(&scene_id);
            }
            self.in_dialog = false;
            true
        }
    }

    /// Confirm the highlighted choice.
    pub fn select_choice(&mut self) {
        // Collect lines in a scoped block so the borrow on self.dialog is
        // released before we call current_available_choice_texts().
        // DialogEngine::select_choice commits any sets_flag before returning,
        // so the post-commit flag state is used when filtering below.
        let scene_found = {
            if let Some(scene) = self.dialog.select_choice(self.choice_index) {
                self.scene_lines = scene
                    .lines
                    .iter()
                    .map(|l| (l.speaker.clone(), l.text.clone()))
                    .collect();
                self.line_index = 0;
                self.choice_index = 0;
                self.in_dialog = !self.scene_lines.is_empty();
                true
            } else {
                false
            }
        }; // borrow on self.dialog released here

        if scene_found {
            // Filter with the post-commit flag state (includes any flag set by
            // the choice we just resolved).
            self.scene_choices = self.dialog.current_available_choice_texts();
        } else {
            if let Some(scene_id) = self.dialog.current_scene_id().map(str::to_string) {
                self.dialog.mark_scene_complete(&scene_id);
            }
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
            follow_companions(world, self.player_entity, &self.companion_entities, &self.zone);
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

// ── Helper functions ─────────────────────────────────────────────────────────

/// Determine if a story ending has been reached based on completed dialog
/// scenes and flags. Returns `None` if the game should continue normally.
fn resolve_ending(dialog: &crate::dialog::DialogEngine, loop_number: u32) -> Option<EndingKind> {
    if loop_number < 5 {
        return None;
    }
    if dialog.is_scene_complete("loop5_ending_doss") {
        return Some(EndingKind::DossPreserved);
    }
    if dialog.is_scene_complete("loop5_ending_kaleo") {
        return Some(EndingKind::KaleoCompromise);
    }
    if dialog.is_scene_complete("loop5_orin_player_agrees") {
        return Some(EndingKind::SableDestroyed);
    }
    if dialog.is_flag_set("orin_stopped") && !dialog.is_flag_set("convergence_triggered") {
        return Some(EndingKind::SableStopped);
    }
    None
}

/// Return the embedded YAML source for the given loop number (1–5).
pub fn loop_yaml(loop_number: u32) -> &'static str {
    match loop_number {
        1 => include_str!("../data/loops/loop1.yaml"),
        2 => include_str!("../data/loops/loop2.yaml"),
        3 => include_str!("../data/loops/loop3.yaml"),
        4 => include_str!("../data/loops/loop4.yaml"),
        _ => include_str!("../data/loops/loop5.yaml"),
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
        let (player_entity, companion_entities) = setup_exploration(&mut world, &party);

        let mut dialog = DialogEngine::new();
        let loop_number = 1u32;
        dialog
            .load_script(loop_yaml(loop_number))
            .expect("load loop yaml");

        let mut rng = StdRng::seed_from_u64(rand::random::<u64>());
        let zone = Zone::enter(ZoneKind::ResearchWing, 1, loop_number, &mut rng);
        let npcs = zone_npcs(zone.kind, zone.cols, zone.rows, loop_number, dialog.flags());

        let mut exploration = ExplorationState {
            player_entity,
            companion_entities,
            npcs,
            dialog,
            zone,
            party,
            scene_lines: Vec::new(),
            scene_choices: Vec::new(),
            line_index: 0,
            choice_index: 0,
            in_dialog: false,
            travel: None,
            pending_battle: false,
            fought_scripted_encounters: HashSet::new(),
        };
        exploration.fire_trigger(Trigger::OnEnter);

        let mut session = Self {
            phase: GamePhase::Exploration(exploration),
            world,
            battle: None,
            last_event: None,
            loop_number: 1,
        };
        session.apply_pending_battle();
        session.maybe_start_battle();
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

    /// Advance one line of active dialog. Returns `true` when the dialog
    /// window closes. If a battle was pending and the dialog just closed,
    /// the battle starts automatically.
    pub fn advance_dialog(&mut self) -> bool {
        let closed = {
            let GamePhase::Exploration(e) = &mut self.phase else {
                return false;
            };
            e.advance_dialog()
        };
        if closed {
            self.maybe_start_battle();
        }
        closed
    }

    /// Confirm the highlighted choice. If this choice closes the dialog and
    /// a battle was pending, the battle starts automatically.
    pub fn select_choice(&mut self) {
        let dialog_closed = {
            let GamePhase::Exploration(e) = &mut self.phase else {
                return;
            };
            let was_in = e.in_dialog;
            e.select_choice();
            was_in && !e.in_dialog
        };
        self.sync_and_recruit_companions();
        if dialog_closed {
            let ending = if let GamePhase::Exploration(e) = &self.phase {
                resolve_ending(&e.dialog, self.loop_number)
            } else {
                None
            };
            if let Some(ending) = ending {
                self.phase = GamePhase::Ended(ending);
                return;
            }
            self.maybe_start_battle();
        }
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
        exploration.fire_trigger(Trigger::OnCombatEnd);
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
            exploration.zone.cols,
            exploration.zone.rows,
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
            let (spawn, companion_entities, zone_cols, zone_rows) = {
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
                    e.dialog.flags(),
                );
                // Spawn 1 tile inward from the entry door (faces back toward origin).
                let spawn = spawn_pos_near_door(&e.zone, travel_dir.opposite());
                e.fire_trigger(Trigger::OnEnter);
                (spawn, e.companion_entities.clone(), e.zone.cols, e.zone.rows)
            }; // self.phase borrow released
            *self
                .world
                .get_mut::<Position>(player_entity)
                .expect("player has Position") = Position::new(spawn.0, spawn.1);
            place_companions_near(&mut self.world, &companion_entities, spawn, zone_cols, zone_rows);
            self.sync_and_recruit_companions();
            self.apply_pending_battle();
            self.maybe_start_battle();
            true
        } else {
            let (spawn, companion_entities, zone_cols, zone_rows) = {
                let GamePhase::Exploration(e) = &mut self.phase else {
                    unreachable!()
                };
                e.travel.as_mut().unwrap().hallways_traversed += 1;
                e.zone = Zone::enter_hallway(depth, loop_number, travel_dir, rng);
                e.npcs.clear();
                // Spawn 1 tile inward from the backtrack door.
                let spawn = spawn_pos_near_door(&e.zone, travel_dir.opposite());
                (spawn, e.companion_entities.clone(), e.zone.cols, e.zone.rows)
            }; // self.phase borrow released
            *self
                .world
                .get_mut::<Position>(player_entity)
                .expect("player has Position") = Position::new(spawn.0, spawn.1);
            place_companions_near(&mut self.world, &companion_entities, spawn, zone_cols, zone_rows);
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
        let (spawn, companion_entities, zone_cols, zone_rows) = {
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
                e.dialog.flags(),
            );
            // Spawn 1 tile inward from the door that leads toward the destination.
            let spawn = spawn_pos_near_door(&e.zone, travel_dir);
            e.fire_trigger(Trigger::OnEnter);
            (spawn, e.companion_entities.clone(), e.zone.cols, e.zone.rows)
        }; // self.phase borrow released
        *self
            .world
            .get_mut::<Position>(player_entity)
            .expect("player has Position") = Position::new(spawn.0, spawn.1);
        place_companions_near(&mut self.world, &companion_entities, spawn, zone_cols, zone_rows);
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

    /// Advance to the next loop: increment loop_number, restore party HP, and
    /// restart the player in ResearchWing with the appropriate opening scene.
    pub fn reset_loop(&mut self, rng: &mut impl rand::Rng) {
        if matches!(self.phase, GamePhase::Ended(_)) {
            return;
        }
        self.loop_number = (self.loop_number + 1).min(5);
        let loop_number = self.loop_number;

        // Restore party HP to max.
        let mut q = self.world.query::<&mut crate::health::Health>();
        for mut h in q.iter_mut(&mut self.world) {
            h.current = h.max;
        }

        let (player_entity, companion_entities, zone_cols, zone_rows) = {
            let GamePhase::Exploration(e) = &mut self.phase else {
                return;
            };
            // Reload scenes for the new loop; flags are preserved.
            e.dialog.clear_scenes();
            e.dialog
                .load_script(loop_yaml(loop_number))
                .expect("load loop yaml");
            e.zone = Zone::enter(ZoneKind::ResearchWing, 1, loop_number, rng);
            e.travel = None;
            e.npcs = zone_npcs(
                e.zone.kind,
                e.zone.cols,
                e.zone.rows,
                loop_number,
                e.dialog.flags(),
            );
            e.fire_trigger(Trigger::OnEnter);
            (e.player_entity, e.companion_entities.clone(), e.zone.cols, e.zone.rows)
        }; // self.phase borrow released
        *self
            .world
            .get_mut::<Position>(player_entity)
            .expect("player has Position") = Position::new(1, 1);
        place_companions_near(&mut self.world, &companion_entities, (1, 1), zone_cols, zone_rows);
        self.sync_and_recruit_companions();
        self.apply_pending_battle();
        self.maybe_start_battle();
    }

    /// Capture the minimal state needed to reconstruct this session later.
    pub fn to_save_data(&self) -> SaveData {
        let ending = if let GamePhase::Ended(k) = &self.phase {
            Some(k.clone())
        } else {
            None
        };
        let GamePhase::Exploration(exploration) = &self.phase else {
            // Fall back to defaults if called mid-battle or after ending.
            return SaveData {
                loop_number: self.loop_number,
                flags: vec![],
                active_companion: None,
                current_zone: ZoneKind::ResearchWing,
                party_kinds: vec![CharacterKind::Researcher],
                party_hp: vec![],
                completed_scenes: vec![],
                fought_scripted_encounters: vec![],
                ending,
            };
        };
        let flags = exploration.dialog.export_flags();
        let active_companion = exploration.dialog.active_companion().map(str::to_string);
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
        let completed_scenes = exploration.dialog.export_completed_scenes();
        let fought_scripted_encounters = {
            let mut v: Vec<String> = exploration
                .fought_scripted_encounters
                .iter()
                .cloned()
                .collect();
            v.sort();
            v
        };
        SaveData {
            loop_number: self.loop_number,
            flags,
            active_companion,
            current_zone,
            party_kinds,
            party_hp,
            completed_scenes,
            fought_scripted_encounters,
            ending,
        }
    }

    /// Reconstruct a game session from previously saved data.
    pub fn from_save_data(data: SaveData, rng: &mut impl rand::Rng) -> Self {
        let party: Vec<Character> = data
            .party_kinds
            .iter()
            .zip(data.party_hp.iter().chain(std::iter::repeat(&i32::MAX)))
            .map(|(kind, &hp)| {
                let mut ch = Character::new_character(kind.clone(), 1);
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

        let mut dialog = DialogEngine::new();
        let loop_number = data.loop_number;
        dialog
            .load_script(loop_yaml(loop_number))
            .expect("load loop yaml");
        dialog.import_flags(data.flags);
        dialog.import_completed_scenes(data.completed_scenes);
        if let Some(companion) = data.active_companion {
            dialog.set_companion(companion);
        }

        let zone = Zone::enter(data.current_zone, 1, loop_number, rng);
        let npcs = zone_npcs(zone.kind, zone.cols, zone.rows, loop_number, dialog.flags());

        let mut exploration = ExplorationState {
            player_entity,
            companion_entities,
            npcs,
            dialog,
            zone,
            party,
            scene_lines: Vec::new(),
            scene_choices: Vec::new(),
            line_index: 0,
            choice_index: 0,
            in_dialog: false,
            travel: None,
            pending_battle: false,
            fought_scripted_encounters: data.fought_scripted_encounters.into_iter().collect(),
        };
        exploration.fire_trigger(Trigger::OnEnter);

        let mut session = Self {
            phase: GamePhase::Exploration(exploration),
            world,
            battle: None,
            last_event: None,
            loop_number,
        };
        session.apply_pending_battle();
        session.maybe_start_battle();
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
    /// Sync the dialog engine's active companion from flags, then spawn any
    /// newly recruited companions that are flagged but not yet in the party.
    pub fn sync_and_recruit_companions(&mut self) {
        // Step 1: update dialog active_companion pointer.
        {
            let GamePhase::Exploration(e) = &mut self.phase else {
                return;
            };
            sync_companion(&mut e.dialog);
        }

        // Step 2: collect which companion kinds need to be spawned.
        let (to_spawn, researcher_level, researcher_pos, zone_cols, zone_rows, companion_count) = {
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
                    e.dialog.is_flag_set(flag) && !e.party.iter().any(|c| &c.kind == kind)
                })
                .map(|(_, kind)| kind.clone())
                .collect();
            (
                to_spawn,
                researcher_level,
                researcher_pos,
                e.zone.cols,
                e.zone.rows,
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
            let pos = Position::new(
                (researcher_pos.x + offset).clamp(0, zone_cols as i32 - 1),
                researcher_pos.y.clamp(0, zone_rows as i32 - 1),
            );
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

// ── World setup ───────────────────────────────────────────────────────────────

/// Spawns the party into the exploration world. Returns the Researcher's entity
/// and a vec of companion entities (one per `party[1..]`).
pub fn setup_exploration(world: &mut World, party: &[Character]) -> (Entity, Vec<Entity>) {
    let ch = &party[0];
    let ap_max = ap_for_speed(ch.stats.speed);
    let player = world
        .spawn((
            ch.clone(),
            ch.stats.clone(),
            Health::new(ch.current_hp),
            ActionPoints::new(ap_max),
            Experience::new(),
            Position::new(0, 2),
        ))
        .id();

    let mut companions = vec![];
    for (i, companion) in party[1..].iter().enumerate() {
        let stats = companion.stats.clone();
        let ap_max = ap_for_speed(stats.speed);
        let pos = Position::new(i as i32 + 1, 2);
        let e = world
            .spawn((
                companion.clone(),
                stats,
                Health::new(companion.current_hp),
                ActionPoints::new(ap_max),
                Experience::new(),
                pos,
                PartyCompanion,
            ))
            .id();
        companions.push(e);
    }
    (player, companions)
}

/// Adds enemies and battle resources to the world. Party is already present from
/// `setup_exploration`.
///
/// When `script` is `Some`, enemy positions are taken from the scripted
/// encounter definition instead of being generated randomly.  Scripted allies
/// are spawned with the [`ScriptedAlly`] marker component so they are
/// despawned at the end of combat.
pub fn setup_battle(world: &mut World, zone: &Zone, script: Option<&ScriptedEncounter>) {
    let mut rng = StdRng::seed_from_u64(rand::random::<u64>());

    // Spawn enemies — either from the script or generated randomly.
    if let Some(s) = script {
        for placement in &s.enemies {
            let (character, pos) = placement.to_character_and_pos(zone.cols, zone.rows);
            let stats = character.stats.clone();
            let hp = character.current_hp;
            let ap_max = ap_for_speed(stats.speed);
            let mut entity_cmd = world.spawn((
                character,
                stats,
                Health::new(hp),
                ActionPoints::new(ap_max),
                pos,
            ));
            if let Some(ability_name) = placement.first_ability {
                entity_cmd.insert(ScriptedFirstAction {
                    ability_name,
                    executed: false,
                });
            }
        }
    } else {
        for (character, pos) in zone.generate_enemies(&mut rng) {
            let stats = character.stats.clone();
            let hp = character.current_hp;
            let ap_max = ap_for_speed(stats.speed);
            world.spawn((
                character,
                stats,
                Health::new(hp),
                ActionPoints::new(ap_max),
                pos,
            ));
        }
    }

    // Spawn scripted allies (temporary, fight on the player's side).
    if let Some(s) = script {
        for placement in &s.allies {
            let (character, pos) = placement.to_character_and_pos(zone.cols, zone.rows);
            let stats = character.stats.clone();
            let hp = character.current_hp;
            let ap_max = ap_for_speed(stats.speed);
            let mut entity_cmd = world.spawn((
                character,
                stats,
                Health::new(hp),
                ActionPoints::new(ap_max),
                pos,
                ScriptedAlly,
            ));
            if let Some(ability_name) = placement.first_ability {
                entity_cmd.insert(ScriptedFirstAction {
                    ability_name,
                    executed: false,
                });
            }
        }
    }

    let battle_rng = StdRng::seed_from_u64(rand::random::<u64>());
    world.insert_resource(zone.map.clone());
    world.insert_resource(BattleRng(battle_rng));
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Move each companion one step toward the player if Chebyshev distance > 3.
fn follow_companions(
    world: &mut World,
    player_entity: Entity,
    companion_entities: &[Entity],
    zone: &Zone,
) {
    let player_pos = match world.get::<Position>(player_entity) {
        Some(p) => *p,
        None => return,
    };
    for &entity in companion_entities {
        let comp_pos = match world.get::<Position>(entity) {
            Some(p) => *p,
            None => continue,
        };
        let dx = player_pos.x - comp_pos.x;
        let dy = player_pos.y - comp_pos.y;
        let chebyshev = dx.abs().max(dy.abs());
        if chebyshev <= 3 {
            continue;
        }
        let sx = dx.signum();
        let sy = dy.signum();
        for (cx, cy) in [(sx, sy), (sx, 0), (0, sy)] {
            if cx == 0 && cy == 0 {
                continue;
            }
            let nx = (comp_pos.x + cx).clamp(0, zone.cols as i32 - 1);
            let ny = (comp_pos.y + cy).clamp(0, zone.rows as i32 - 1);
            if zone.map.get(nx, ny).is_passable() {
                if let Some(mut pos) = world.get_mut::<Position>(entity) {
                    *pos = Position::new(nx, ny);
                }
                break;
            }
        }
    }
}

/// Teleport each companion to a position adjacent to `spawn` within zone bounds.
fn place_companions_near(
    world: &mut World,
    companion_entities: &[Entity],
    spawn: (i32, i32),
    cols: u32,
    rows: u32,
) {
    for (i, &entity) in companion_entities.iter().enumerate() {
        let ox = i as i32 + 1;
        let nx = (spawn.0 + ox).clamp(0, cols as i32 - 1);
        let ny = spawn.1.clamp(0, rows as i32 - 1);
        if let Some(mut pos) = world.get_mut::<Position>(entity) {
            *pos = Position::new(nx, ny);
        }
    }
}

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

/// Return the NPCs that should populate `kind` given the current `loop_number` and flag state.
pub fn zone_npcs(
    kind: ZoneKind,
    cols: u32,
    rows: u32,
    loop_number: u32,
    flags: &std::collections::HashSet<String>,
) -> Vec<NpcData> {
    let cx = (cols as i32 / 2).max(1);
    let cy = (rows as i32 / 3).max(1);
    match kind {
        ZoneKind::CommandDeck if !flags.contains("companion_orin") => vec![NpcData {
            pos: (cx, cy),
            name: "Orin",
            glyph: 'O',
        }],
        ZoneKind::MilitaryAnnex if !flags.contains("companion_doss") => vec![NpcData {
            pos: (cx, cy),
            name: "Doss",
            glyph: 'D',
        }],
        ZoneKind::SystemsCore if loop_number >= 2 => vec![NpcData {
            pos: (cx, cy),
            name: "Kaleo",
            glyph: 'K',
        }],
        ZoneKind::DockingBay if loop_number <= 3 => vec![NpcData {
            pos: (cx, cy),
            name: "Gun-for-Hire",
            glyph: 'H',
        }],
        _ => vec![],
    }
}

/// Sync the dialog engine's active companion from the flag state.
/// Should be called whenever the player arrives in a new zone.
pub fn sync_companion(dialog: &mut DialogEngine) {
    if dialog.is_flag_set("companion_orin") {
        dialog.set_companion("orin");
    } else if dialog.is_flag_set("companion_doss") {
        dialog.set_companion("doss");
    } else if dialog.is_flag_set("kaleo_recruited") {
        dialog.set_companion("kaleo");
    }
}
