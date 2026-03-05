use rand::SeedableRng;
use rand::rngs::StdRng;
use carbonthrone::zone::{CardinalDir, ZoneKind, ZoneType, zone_connections};

fn rng() -> StdRng {
    StdRng::seed_from_u64(42)
}

// ── Connection topology ───────────────────────────────────────────────────────

#[test]
fn connections_are_symmetric() {
    // For every A --dir--> B, B must connect back to A via the opposite direction.
    let all_zones = [
        ZoneKind::ResearchWing,
        ZoneKind::CommandDeck,
        ZoneKind::MilitaryAnnex,
        ZoneKind::SystemsCore,
        ZoneKind::MedicalBay,
        ZoneKind::DockingBay,
        ZoneKind::StationExterior,
        ZoneKind::RelayArray,
        ZoneKind::ExcavationSite,
    ];
    let dirs = [
        CardinalDir::North,
        CardinalDir::South,
        CardinalDir::East,
        CardinalDir::West,
    ];
    for &zone in &all_zones {
        let conns = zone_connections(zone);
        for &dir in &dirs {
            if let Some(neighbor) = conns.get(dir) {
                let neighbor_conns = zone_connections(neighbor);
                let back = neighbor_conns.get(dir.opposite());
                assert_eq!(
                    back,
                    Some(zone),
                    "{:?} --{:?}--> {:?}, but {:?} does not connect back via {:?}",
                    zone, dir, neighbor, neighbor, dir.opposite()
                );
            }
        }
    }
}

#[test]
fn research_wing_connects_north_to_command_deck() {
    let conns = zone_connections(ZoneKind::ResearchWing);
    assert_eq!(conns.north, Some(ZoneKind::CommandDeck));
}

#[test]
fn research_wing_connects_east_to_systems_core() {
    let conns = zone_connections(ZoneKind::ResearchWing);
    assert_eq!(conns.east, Some(ZoneKind::SystemsCore));
}

#[test]
fn docking_bay_has_four_connections() {
    let conns = zone_connections(ZoneKind::DockingBay);
    assert_eq!(conns.north, Some(ZoneKind::StationExterior));
    assert_eq!(conns.east, Some(ZoneKind::MilitaryAnnex));
    assert_eq!(conns.west, Some(ZoneKind::CommandDeck));
    assert_eq!(conns.south, Some(ZoneKind::SystemsCore));
}

#[test]
fn relay_array_only_connects_south() {
    let conns = zone_connections(ZoneKind::RelayArray);
    assert_eq!(conns.south, Some(ZoneKind::StationExterior));
    assert!(conns.north.is_none());
    assert!(conns.east.is_none());
    assert!(conns.west.is_none());
}

#[test]
fn excavation_site_only_connects_east() {
    let conns = zone_connections(ZoneKind::ExcavationSite);
    assert_eq!(conns.east, Some(ZoneKind::StationExterior));
    assert!(conns.north.is_none());
    assert!(conns.south.is_none());
    assert!(conns.west.is_none());
}

// ── Zone types ────────────────────────────────────────────────────────────────

#[test]
fn interior_zones_have_correct_type() {
    let interiors = [
        ZoneKind::ResearchWing,
        ZoneKind::CommandDeck,
        ZoneKind::MilitaryAnnex,
        ZoneKind::SystemsCore,
        ZoneKind::MedicalBay,
        ZoneKind::DockingBay,
    ];
    for zone in interiors {
        assert_eq!(zone.zone_type(), ZoneType::Interior, "{:?} should be Interior", zone);
    }
}

#[test]
fn exterior_zones_have_correct_type() {
    let exteriors = [
        ZoneKind::StationExterior,
        ZoneKind::RelayArray,
        ZoneKind::ExcavationSite,
    ];
    for zone in exteriors {
        assert_eq!(zone.zone_type(), ZoneType::Exterior, "{:?} should be Exterior", zone);
    }
}

// ── Zone encounter generation ─────────────────────────────────────────────────

#[test]
fn zone_enter_generates_encounter_with_enemies() {
    use carbonthrone::zone::Zone;
    let zone = Zone::enter(ZoneKind::DockingBay, 1, &mut rng());
    assert!(!zone.level.enemies.is_empty());
}

#[test]
fn zone_enter_enemies_come_from_zone_pool() {
    use carbonthrone::zone::Zone;
    let mut r = rng();
    for _ in 0..20 {
        let zone = Zone::enter(ZoneKind::ResearchWing, 1, &mut r);
        let pool = ZoneKind::ResearchWing.enemy_pool();
        for (enemy, _) in &zone.level.enemies {
            assert!(
                pool.contains(&enemy.kind),
                "ResearchWing spawned unexpected enemy kind: {:?}",
                enemy.kind
            );
        }
    }
}

#[test]
fn zone_kind_stored_on_zone() {
    use carbonthrone::zone::Zone;
    let zone = Zone::enter(ZoneKind::RelayArray, 3, &mut rng());
    assert_eq!(zone.kind, ZoneKind::RelayArray);
}

#[test]
fn zone_connections_stored_on_zone() {
    use carbonthrone::zone::Zone;
    let zone = Zone::enter(ZoneKind::RelayArray, 1, &mut rng());
    assert_eq!(zone.connections.south, Some(ZoneKind::StationExterior));
}

#[test]
fn enemy_level_matches_depth_in_zone() {
    use carbonthrone::zone::Zone;
    let depth = 4;
    let zone = Zone::enter(ZoneKind::MilitaryAnnex, depth, &mut rng());
    assert!(zone.level.enemies.iter().all(|(e, _)| e.level == depth));
}

#[test]
fn all_zones_can_be_entered() {
    use carbonthrone::zone::Zone;
    let all_zones = [
        ZoneKind::ResearchWing,
        ZoneKind::CommandDeck,
        ZoneKind::MilitaryAnnex,
        ZoneKind::SystemsCore,
        ZoneKind::MedicalBay,
        ZoneKind::DockingBay,
        ZoneKind::StationExterior,
        ZoneKind::RelayArray,
        ZoneKind::ExcavationSite,
    ];
    let mut r = rng();
    for kind in all_zones {
        let zone = Zone::enter(kind, 1, &mut r);
        assert!(!zone.level.enemies.is_empty(), "{:?} produced no enemies", kind);
    }
}
