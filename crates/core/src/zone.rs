use crate::character::{Character, CharacterKind};
use crate::position::Position;
use crate::terrain::{LevelMap, Tile, generate_map};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// One of the nine named zones in the Meridian station, as described in docs/world.md.
/// `Hallway` is an anonymous connecting corridor used during zone travel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZoneKind {
    // Interior zones
    ResearchWing,
    CommandDeck,
    MilitaryAnnex,
    SystemsCore,
    MedicalBay,
    DockingBay,
    // Exterior zones
    StationExterior,
    RelayArray,
    ExcavationSite,
    // Anonymous connecting corridor (used during travel between named zones)
    Hallway,
}

/// Whether a zone is inside the station or on the moon surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneType {
    Interior,
    Exterior,
}

/// Adjacent zone for each cardinal direction, as defined in docs/world.md.
///
/// ```text
///                          [Relay Array]
///                                |N
///           [Excavation Site]--W[Station Exterior]
///                                     |S
///              [Command Deck]--W[Docking Bay]--E[Military Annex]
///                    |S               |S               |S
///              [Research Wing]--E[Systems Core]--E[Medical Bay]
/// ```
#[derive(Debug, Clone, Default)]
pub struct ZoneConnections {
    pub north: Option<ZoneKind>,
    pub south: Option<ZoneKind>,
    pub east: Option<ZoneKind>,
    pub west: Option<ZoneKind>,
}

impl ZoneConnections {
    pub fn get(&self, dir: CardinalDir) -> Option<ZoneKind> {
        match dir {
            CardinalDir::North => self.north,
            CardinalDir::South => self.south,
            CardinalDir::East => self.east,
            CardinalDir::West => self.west,
        }
    }
}

/// Cardinal direction used for zone navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardinalDir {
    North,
    South,
    East,
    West,
}

impl CardinalDir {
    /// Returns the direction directly opposite to this one.
    pub fn opposite(self) -> Self {
        match self {
            CardinalDir::North => CardinalDir::South,
            CardinalDir::South => CardinalDir::North,
            CardinalDir::East => CardinalDir::West,
            CardinalDir::West => CardinalDir::East,
        }
    }
}

/// Probability of a combat encounter when entering a zone, scaled by loop number.
///
/// | Loop | Chance |
/// |------|--------|
/// |  1   | 40%    |
/// |  2   | 55%    |
/// |  3   | 70%    |
/// |  4   | 85%    |
/// |  5+  | 90%    |
///
/// Maximum clamped to 0.90.
pub fn encounter_chance(loop_number: u32) -> f64 {
    (0.25 + loop_number as f64 * 0.15).min(0.90)
}

/// Who, if anyone, has the initiative advantage at the start of an encounter.
#[derive(Debug, Clone, PartialEq)]
pub enum SurpriseState {
    /// Turn order is determined normally by speed stats.
    Normal,
    /// Enemies detected the party first — enemies act before the party this encounter.
    PartyAmbushed,
    /// The party detected the enemies first — party acts before enemies this encounter.
    EnemyAmbushed,
}

/// A zone in the Meridian station with a generated terrain map and optional encounter.
///
/// Call [`Zone::enter`] each time the player enters a zone to get a new random layout.
/// Combat happens before exploration: if `has_encounter()` is true, the party must
/// resolve that fight before NPCs phase-shift in. Use [`Zone::npcs_available`] to check.
/// Use [`Zone::generate_enemies`] to spawn enemies on demand when entering battle.
#[derive(Debug)]
pub struct Zone {
    pub kind: ZoneKind,
    pub connections: ZoneConnections,
    pub depth: u32,
    /// Current loop number (1–5). Determines encounter probability and enemy pool.
    pub loop_number: u32,
    /// Grid width (number of columns along the X axis).
    pub cols: u32,
    /// Grid height (number of rows along the Y axis).
    pub rows: u32,
    /// The terrain map for this zone.
    pub map: LevelMap,
    pub surprise: SurpriseState,
    /// Whether this zone has a combat encounter when entered.
    encounter: bool,
    /// Door tile positions mapped to the cardinal direction they lead toward.
    pub doors: HashMap<(i32, i32), CardinalDir>,
}

impl Zone {
    /// Enter a zone, rolling for a random combat encounter first.
    ///
    /// Encounter probability scales with `loop_number` via [`encounter_chance`]:
    /// higher loops mean more frequent and tougher fights. NPCs only phase-shift
    /// in after the encounter is resolved (or immediately when there is none).
    pub fn enter(kind: ZoneKind, depth: u32, loop_number: u32, rng: &mut impl Rng) -> Self {
        let with_encounter = rng.r#gen::<f64>() < encounter_chance(loop_number);
        Self::build(kind, depth, loop_number, with_encounter, None, rng)
    }

    /// Generate a zone with a guaranteed encounter, picking a random zone kind.
    /// Useful for testing and procedural content generation.
    pub fn generate(depth: u32, loop_number: u32, rng: &mut impl Rng) -> Self {
        let kind = random_zone_kind(rng);
        Self::build(kind, depth, loop_number, true, None, rng)
    }

    /// Enter an anonymous hallway zone used during travel between named zones.
    /// `travel_dir` is the direction toward the destination (exit door side).
    /// The opposite side gets a backtrack door leading back to the origin.
    pub fn enter_hallway(
        depth: u32,
        loop_number: u32,
        travel_dir: CardinalDir,
        rng: &mut impl Rng,
    ) -> Self {
        let with_encounter = rng.r#gen::<f64>() < encounter_chance(loop_number);
        Self::build(
            ZoneKind::Hallway,
            depth,
            loop_number,
            with_encounter,
            Some(travel_dir),
            rng,
        )
    }

    fn build(
        kind: ZoneKind,
        depth: u32,
        loop_number: u32,
        with_encounter: bool,
        hallway_dir: Option<CardinalDir>,
        rng: &mut impl Rng,
    ) -> Self {
        let connections = zone_connections(kind);
        let cols: u32 = rng.gen_range(8..=16);
        let rows: u32 = rng.gen_range(8..=16);

        // Compute door positions for each connected side (named zones) or exit (hallway).
        let mut doors: HashMap<(i32, i32), CardinalDir> = HashMap::new();
        if kind == ZoneKind::Hallway {
            let travel_dir = hallway_dir.unwrap_or(CardinalDir::East);
            // Exit door in the direction of travel.
            for pos in door_tiles(travel_dir, cols, rows) {
                doors.insert(pos, travel_dir);
            }
            // Backtrack door on the opposite side.
            let back_dir = travel_dir.opposite();
            for pos in door_tiles(back_dir, cols, rows) {
                doors.insert(pos, back_dir);
            }
        } else {
            for dir in [
                CardinalDir::North,
                CardinalDir::South,
                CardinalDir::East,
                CardinalDir::West,
            ] {
                if connections.get(dir).is_some() {
                    for pos in door_tiles(dir, cols, rows) {
                        doors.insert(pos, dir);
                    }
                }
            }
        }

        let all_door_positions: Vec<(i32, i32)> = doors.keys().copied().collect();
        let mut map = generate_map(cols, rows, kind, &[], &all_door_positions, rng);

        // Ensure door tiles are set in the map (generate_map already does this,
        // but we set them here too in case of any overlap).
        for &pos in doors.keys() {
            map.set(pos.0, pos.1, Tile::Door);
        }

        let surprise = match rng.gen_range(0..4u32) {
            0 => SurpriseState::PartyAmbushed,
            1 => SurpriseState::EnemyAmbushed,
            _ => SurpriseState::Normal,
        };

        Self {
            kind,
            connections,
            depth,
            loop_number,
            cols,
            rows,
            map,
            surprise,
            encounter: with_encounter,
            doors,
        }
    }

    /// True when this zone has a combat encounter.
    pub fn has_encounter(&self) -> bool {
        self.encounter
    }

    /// Generate enemies for this zone's encounter. Returns an empty vec if there is no encounter.
    /// Call this when entering battle rather than storing enemies on the zone.
    ///
    /// Enemy types are filtered by `loop_number` via [`ZoneKind::combat_enemy_pool`], so only
    /// faction members present and hostile at the current loop will appear. Aggression is
    /// adjusted per loop using [`crate::character::loop_aggression`].
    pub fn generate_enemies(&self, rng: &mut impl Rng) -> Vec<(Character, Position)> {
        use crate::character::loop_aggression;
        if !self.encounter {
            return Vec::new();
        }
        let enemy_level = self.depth.max(1);
        let enemy_pool = self.kind.combat_enemy_pool(self.loop_number);
        let enemy_count: usize = rng.gen_range(1..=4);
        let mut used: HashSet<(i32, i32)> = HashSet::new();
        let mut enemies = Vec::new();
        while enemies.len() < enemy_count {
            let x = rng.gen_range(0..self.cols as i32);
            let y = rng.gen_range(0..self.rows as i32);
            if self.map.get(x, y) == Tile::Open && used.insert((x, y)) {
                let ek = enemy_pool[rng.gen_range(0..enemy_pool.len())].clone();
                let mut character = Character::new_character(ek, enemy_level);
                character.aggression = loop_aggression(&character.kind, self.loop_number);
                enemies.push((character, Position::new(x, y)));
            }
        }
        enemies
    }

    /// Returns `true` when NPCs may phase-shift into the zone.
    ///
    /// NPCs are held back during an active combat encounter and become
    /// available only once the encounter is cleared (or when there was none).
    pub fn npcs_available(&self, encounter_cleared: bool) -> bool {
        !self.has_encounter() || encounter_cleared
    }
}

impl ZoneKind {
    pub fn zone_type(self) -> ZoneType {
        match self {
            ZoneKind::StationExterior | ZoneKind::RelayArray | ZoneKind::ExcavationSite => {
                ZoneType::Exterior
            }
            _ => ZoneType::Interior,
        }
    }

    pub fn location_id(self) -> &'static str {
        match self {
            ZoneKind::ResearchWing => "research_wing",
            ZoneKind::CommandDeck => "command_deck",
            ZoneKind::MilitaryAnnex => "military_annex",
            ZoneKind::SystemsCore => "systems_core",
            ZoneKind::MedicalBay => "medical_bay",
            ZoneKind::DockingBay => "docking_bay",
            ZoneKind::StationExterior => "station_exterior",
            ZoneKind::RelayArray => "relay_array",
            ZoneKind::ExcavationSite => "excavation_site",
            ZoneKind::Hallway => "hallway",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            ZoneKind::ResearchWing => "Research Wing",
            ZoneKind::CommandDeck => "Command Deck",
            ZoneKind::MilitaryAnnex => "Military Annex",
            ZoneKind::SystemsCore => "Systems Core",
            ZoneKind::MedicalBay => "Medical Bay",
            ZoneKind::DockingBay => "Docking Bay",
            ZoneKind::StationExterior => "Station Exterior",
            ZoneKind::RelayArray => "Relay Array",
            ZoneKind::ExcavationSite => "Excavation Site",
            ZoneKind::Hallway => "Connecting Corridor",
        }
    }

    /// The full set of enemy types that may ever spawn in this zone (loop-independent).
    ///
    /// Use [`ZoneKind::combat_enemy_pool`] instead when generating actual encounters,
    /// as it filters by loop number to match the narrative state of the station.
    pub fn enemy_pool(self) -> &'static [CharacterKind] {
        match self {
            ZoneKind::ResearchWing => &[
                CharacterKind::Zealot,
                CharacterKind::Purifier,
                CharacterKind::Preacher,
                CharacterKind::MaintenanceDrone,
                CharacterKind::Scavenger,
            ],
            ZoneKind::CommandDeck => &[
                CharacterKind::Zealot,
                CharacterKind::Purifier,
                CharacterKind::Preacher,
                CharacterKind::StationGuard,
                CharacterKind::SecurityUnit,
            ],
            ZoneKind::MilitaryAnnex => &[
                CharacterKind::GunForHire,
                CharacterKind::ShockTrooper,
                CharacterKind::Archon,
            ],
            ZoneKind::SystemsCore => &[
                CharacterKind::MaintenanceDrone,
                CharacterKind::SecurityUnit,
                CharacterKind::CombatFrame,
            ],
            ZoneKind::MedicalBay => &[
                CharacterKind::StationGuard,
                CharacterKind::VoidSpitter,
                CharacterKind::AbyssalBrute,
            ],
            ZoneKind::DockingBay => &[
                CharacterKind::Scavenger,
                CharacterKind::VoidRaider,
                CharacterKind::DrifterBoss,
            ],
            ZoneKind::StationExterior => &[
                CharacterKind::MoonCrawler,
                CharacterKind::VoidRaider,
                CharacterKind::AbyssalBrute,
            ],
            ZoneKind::RelayArray => &[CharacterKind::VoidRaider, CharacterKind::DrifterBoss],
            ZoneKind::ExcavationSite => &[
                CharacterKind::MoonCrawler,
                CharacterKind::VoidSpitter,
                CharacterKind::AbyssalBrute,
            ],
            ZoneKind::Hallway => &[
                CharacterKind::Scavenger,
                CharacterKind::MaintenanceDrone,
                CharacterKind::MoonCrawler,
                CharacterKind::VoidRaider,
                CharacterKind::Zealot,
                CharacterKind::Purifier,
            ],
        }
    }

    /// Loop-filtered set of enemies that actively fight when entering this zone.
    ///
    /// Based on world.md encounter tables and npcs.md faction behavior:
    /// - Constancy members appear from loop 1 in Research Wing (the breach point) and spread
    /// - Scavengers breach outer areas from loop 2+
    /// - Automata Security Units go online from loop 2+; Maintenance Drones hostile from loop 3+
    /// - Drifter Bosses establish themselves from loop 3+
    /// - Shock Troopers deployed by Doss from loop 3+
    /// - Abyssal Fauna surge from loop 2–3+; go Lethargic in loops 4–5
    /// - Station Personnel flip to hostile in loops 3–5
    pub fn combat_enemy_pool(self, loop_number: u32) -> Vec<CharacterKind> {
        match self {
            ZoneKind::ResearchWing => {
                // Constancy breach site from loop 1; Drones hostile from loop 3+; Scavengers loop 2+
                let mut pool = vec![CharacterKind::Zealot, CharacterKind::Purifier];
                pool.push(CharacterKind::MaintenanceDrone);
                if loop_number >= 2 {
                    pool.push(CharacterKind::Scavenger);
                    pool.push(CharacterKind::Preacher);
                }
                pool
            }
            ZoneKind::CommandDeck => {
                // Constancy spread here; Security Units loop 2+; Guards hostile loops 4-5
                let mut pool = vec![CharacterKind::Zealot];
                if loop_number >= 2 {
                    pool.push(CharacterKind::SecurityUnit);
                    pool.push(CharacterKind::Purifier);
                }
                if loop_number >= 3 {
                    pool.push(CharacterKind::Preacher);
                }
                if loop_number >= 4 {
                    pool.push(CharacterKind::StationGuard);
                }
                pool
            }
            ZoneKind::MilitaryAnnex => {
                // Gun-for-Hire always present; Shock Troopers from loop 3+; Archon loop 4+
                let mut pool = vec![CharacterKind::GunForHire];
                if loop_number >= 3 {
                    pool.push(CharacterKind::ShockTrooper);
                }
                if loop_number >= 4 {
                    pool.push(CharacterKind::Archon);
                }
                pool
            }
            ZoneKind::SystemsCore => {
                // Maintenance Drones present all loops; Security Units loop 2+; CombatFrame loop 5
                let mut pool = vec![CharacterKind::MaintenanceDrone];
                if loop_number >= 2 {
                    pool.push(CharacterKind::SecurityUnit);
                }
                if loop_number >= 5 {
                    pool.push(CharacterKind::CombatFrame);
                }
                pool
            }
            ZoneKind::MedicalBay => {
                // Guards hostile loop 3+; Void Spitters breach floor loop 4+; Brute loop 5 only
                let mut pool = vec![];
                if loop_number >= 3 {
                    pool.push(CharacterKind::StationGuard);
                }
                if loop_number >= 4 {
                    pool.push(CharacterKind::VoidSpitter);
                }
                if loop_number >= 5 {
                    pool.push(CharacterKind::AbyssalBrute);
                }
                // Fallback for early loops: Scavengers who raided the medical supplies
                if pool.is_empty() {
                    pool.push(CharacterKind::Scavenger);
                }
                pool
            }
            ZoneKind::DockingBay => {
                // Scavengers from loop 1; Raiders loop 2+; Boss loop 3+
                let mut pool = vec![CharacterKind::Scavenger];
                if loop_number >= 2 {
                    pool.push(CharacterKind::VoidRaider);
                }
                if loop_number >= 3 {
                    pool.push(CharacterKind::DrifterBoss);
                }
                pool
            }
            ZoneKind::StationExterior => {
                // Moon Crawlers from loop 1; Raiders loop 2+; Abyssal Brutes loop 3+
                let mut pool = vec![CharacterKind::MoonCrawler];
                if loop_number >= 2 {
                    pool.push(CharacterKind::VoidRaider);
                }
                if loop_number >= 3 {
                    pool.push(CharacterKind::AbyssalBrute);
                }
                pool
            }
            ZoneKind::RelayArray => {
                // Void Raiders camp here all loops; Boss establishes loop 2+
                let mut pool = vec![CharacterKind::VoidRaider];
                if loop_number >= 2 {
                    pool.push(CharacterKind::DrifterBoss);
                }
                pool
            }
            ZoneKind::ExcavationSite => {
                // Moon Crawlers all loops; Spitters and Brutes from loop 2+
                let mut pool = vec![CharacterKind::MoonCrawler];
                if loop_number >= 2 {
                    pool.push(CharacterKind::VoidSpitter);
                    pool.push(CharacterKind::AbyssalBrute);
                }
                pool
            }
            ZoneKind::Hallway => {
                // Connecting corridors host any faction passing through
                let mut pool = vec![CharacterKind::Scavenger, CharacterKind::MoonCrawler];
                if loop_number >= 2 {
                    pool.push(CharacterKind::VoidRaider);
                    pool.push(CharacterKind::Zealot);
                }
                if loop_number >= 3 {
                    pool.push(CharacterKind::MaintenanceDrone);
                    pool.push(CharacterKind::Purifier);
                }
                pool
            }
        }
    }
}

/// Returns the cardinal connections for the given zone as specified in docs/world.md.
pub fn zone_connections(kind: ZoneKind) -> ZoneConnections {
    match kind {
        ZoneKind::ResearchWing => ZoneConnections {
            north: Some(ZoneKind::CommandDeck),
            east: Some(ZoneKind::SystemsCore),
            ..Default::default()
        },
        ZoneKind::CommandDeck => ZoneConnections {
            south: Some(ZoneKind::ResearchWing),
            east: Some(ZoneKind::DockingBay),
            ..Default::default()
        },
        ZoneKind::MilitaryAnnex => ZoneConnections {
            west: Some(ZoneKind::DockingBay),
            south: Some(ZoneKind::MedicalBay),
            ..Default::default()
        },
        ZoneKind::SystemsCore => ZoneConnections {
            west: Some(ZoneKind::ResearchWing),
            east: Some(ZoneKind::MedicalBay),
            north: Some(ZoneKind::DockingBay),
            ..Default::default()
        },
        ZoneKind::MedicalBay => ZoneConnections {
            north: Some(ZoneKind::MilitaryAnnex),
            west: Some(ZoneKind::SystemsCore),
            ..Default::default()
        },
        ZoneKind::DockingBay => ZoneConnections {
            north: Some(ZoneKind::StationExterior),
            east: Some(ZoneKind::MilitaryAnnex),
            west: Some(ZoneKind::CommandDeck),
            south: Some(ZoneKind::SystemsCore),
        },
        ZoneKind::StationExterior => ZoneConnections {
            north: Some(ZoneKind::RelayArray),
            south: Some(ZoneKind::DockingBay),
            west: Some(ZoneKind::ExcavationSite),
            ..Default::default()
        },
        ZoneKind::RelayArray => ZoneConnections {
            south: Some(ZoneKind::StationExterior),
            ..Default::default()
        },
        ZoneKind::ExcavationSite => ZoneConnections {
            east: Some(ZoneKind::StationExterior),
            ..Default::default()
        },
        // Hallway connections are managed by TravelState, not statically defined.
        ZoneKind::Hallway => ZoneConnections::default(),
    }
}

/// Returns the 4 tile positions that form a door on the given side of a grid.
/// Tiles are centered on the side, clamped to valid bounds.
fn door_tiles(dir: CardinalDir, cols: u32, rows: u32) -> Vec<(i32, i32)> {
    match dir {
        CardinalDir::North => {
            let mid = cols as i32 / 2;
            (mid - 2..mid + 2)
                .map(|x| (x.clamp(0, cols as i32 - 1), 0))
                .collect()
        }
        CardinalDir::South => {
            let mid = cols as i32 / 2;
            let y = rows as i32 - 1;
            (mid - 2..mid + 2)
                .map(|x| (x.clamp(0, cols as i32 - 1), y))
                .collect()
        }
        CardinalDir::East => {
            let mid = rows as i32 / 2;
            let x = cols as i32 - 1;
            (mid - 2..mid + 2)
                .map(|y| (x, y.clamp(0, rows as i32 - 1)))
                .collect()
        }
        CardinalDir::West => {
            let mid = rows as i32 / 2;
            (mid - 2..mid + 2)
                .map(|y| (0, y.clamp(0, rows as i32 - 1)))
                .collect()
        }
    }
}

fn random_zone_kind(rng: &mut impl Rng) -> ZoneKind {
    // Hallway is excluded — it is only created via Zone::enter_hallway.
    match rng.gen_range(0..9u32) {
        0 => ZoneKind::ResearchWing,
        1 => ZoneKind::CommandDeck,
        2 => ZoneKind::MilitaryAnnex,
        3 => ZoneKind::SystemsCore,
        4 => ZoneKind::MedicalBay,
        5 => ZoneKind::DockingBay,
        6 => ZoneKind::StationExterior,
        7 => ZoneKind::RelayArray,
        _ => ZoneKind::ExcavationSite,
    }
}
