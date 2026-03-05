use crate::character::CharacterKind;
use crate::level::Level;
use crate::terrain::Biome;
use rand::Rng;

/// One of the nine named zones in the Meridian station, as described in docs/world.md.
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

/// A zone in the Meridian station with a freshly generated encounter layout.
///
/// Call [`Zone::enter`] each time the player enters a zone to get a new random layout.
/// Combat happens before exploration: if `encounter` is `Some`, the party must resolve
/// that fight before NPCs phase-shift in. Use [`Zone::npcs_available`] to check
/// whether NPCs should appear.
#[derive(Debug)]
pub struct Zone {
    pub kind: ZoneKind,
    pub connections: ZoneConnections,
    /// `Some` when a combat encounter was rolled on entry; `None` for a clear zone.
    pub encounter: Option<Level>,
}

impl Zone {
    /// Enter a zone, rolling for a random combat encounter first.
    ///
    /// There is an [`ENCOUNTER_CHANCE`] probability that enemies are generated
    /// using zone-appropriate biome and enemy types. NPCs only phase-shift in
    /// after the encounter is resolved (or immediately when there is none).
    pub fn enter(kind: ZoneKind, depth: u32, rng: &mut impl Rng) -> Self {
        let connections = zone_connections(kind);
        let encounter = if rng.r#gen::<f64>() < ENCOUNTER_CHANCE {
            let biome = kind.default_biome();
            let enemy_pool = kind.enemy_pool();
            Some(Level::generate_for_zone(depth, biome, enemy_pool, rng))
        } else {
            None
        };
        Self {
            kind,
            connections,
            encounter,
        }
    }

    /// Returns `true` when NPCs may phase-shift into the zone.
    ///
    /// NPCs are held back during an active combat encounter and become
    /// available only once the encounter is cleared (or when there was none).
    pub fn npcs_available(&self, encounter_cleared: bool) -> bool {
        self.encounter.is_none() || encounter_cleared
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
        }
    }

    /// The default terrain biome for encounters in this zone.
    pub fn default_biome(self) -> Biome {
        match self {
            ZoneKind::ResearchWing => Biome::BioLab,
            ZoneKind::CommandDeck => Biome::VoidStation,
            ZoneKind::MilitaryAnnex => Biome::VoidStation,
            ZoneKind::SystemsCore => Biome::VoidStation,
            ZoneKind::MedicalBay => Biome::BioLab,
            ZoneKind::DockingBay => Biome::NeonDistrict,
            ZoneKind::StationExterior => Biome::AsteroidColony,
            ZoneKind::RelayArray => Biome::AsteroidColony,
            ZoneKind::ExcavationSite => Biome::AsteroidColony,
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
    }
}
