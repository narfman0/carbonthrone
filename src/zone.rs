use crate::character::{Character, CharacterKind};
use crate::position::Position;
use crate::terrain::{LevelMap, Tile, generate_map};
use rand::Rng;
use std::collections::{HashMap, HashSet};

/// One of the nine named zones in the Meridian station, as described in docs/world.md.
/// `Hallway` is an anonymous connecting corridor used during zone travel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// Probability that a combat encounter is generated when entering a zone.
/// At 0.75, three out of four zone entries trigger a fight.
pub const ENCOUNTER_CHANCE: f64 = 0.75;

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
    /// There is an [`ENCOUNTER_CHANCE`] probability that enemies are generated
    /// using zone-appropriate terrain and enemy types. NPCs only phase-shift in
    /// after the encounter is resolved (or immediately when there is none).
    pub fn enter(kind: ZoneKind, depth: u32, rng: &mut impl Rng) -> Self {
        let with_encounter = rng.r#gen::<f64>() < ENCOUNTER_CHANCE;
        Self::build(kind, depth, with_encounter, None, rng)
    }

    /// Generate a zone with a guaranteed encounter, picking a random zone kind.
    /// Useful for testing and procedural content generation.
    pub fn generate(depth: u32, rng: &mut impl Rng) -> Self {
        let kind = random_zone_kind(rng);
        Self::build(kind, depth, true, None, rng)
    }

    /// Enter an anonymous hallway zone used during travel between named zones.
    /// `travel_dir` is the direction toward the destination (exit door side).
    /// The opposite side gets a backtrack door leading back to the origin.
    pub fn enter_hallway(depth: u32, travel_dir: CardinalDir, rng: &mut impl Rng) -> Self {
        let with_encounter = rng.r#gen::<f64>() < ENCOUNTER_CHANCE;
        Self::build(
            ZoneKind::Hallway,
            depth,
            with_encounter,
            Some(travel_dir),
            rng,
        )
    }

    fn build(
        kind: ZoneKind,
        depth: u32,
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
    pub fn generate_enemies(&self, rng: &mut impl Rng) -> Vec<(Character, Position)> {
        if !self.encounter {
            return Vec::new();
        }
        let enemy_level = self.depth.max(1);
        let enemy_pool = self.kind.enemy_pool();
        let enemy_count: usize = rng.gen_range(1..=4);
        let mut used: HashSet<(i32, i32)> = HashSet::new();
        let mut enemies = Vec::new();
        while enemies.len() < enemy_count {
            let x = rng.gen_range(0..self.cols as i32);
            let y = rng.gen_range(0..self.rows as i32);
            if self.map.get(x, y) == Tile::Open && used.insert((x, y)) {
                let ek = enemy_pool[rng.gen_range(0..enemy_pool.len())].clone();
                enemies.push((
                    Character::new_character(ek, enemy_level),
                    Position::new(x, y),
                ));
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

    /// The set of enemy types that can spawn in encounters within this zone.
    pub fn enemy_pool(self) -> &'static [CharacterKind] {
        match self {
            ZoneKind::ResearchWing => &[CharacterKind::MaintenanceDrone, CharacterKind::Scavenger],
            ZoneKind::CommandDeck => &[CharacterKind::StationGuard, CharacterKind::SecurityUnit],
            ZoneKind::MilitaryAnnex => &[CharacterKind::GunForHire, CharacterKind::ShockTrooper],
            ZoneKind::SystemsCore => {
                &[CharacterKind::MaintenanceDrone, CharacterKind::SecurityUnit]
            }
            ZoneKind::MedicalBay => &[CharacterKind::VoidSpitter, CharacterKind::StationGuard],
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
            ],
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
