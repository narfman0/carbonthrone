use carbonthrone::zone::{CardinalDir, ZoneKind, ZoneType, zone_connections};
use rand::SeedableRng;
use rand::rngs::StdRng;

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
                    zone,
                    dir,
                    neighbor,
                    neighbor,
                    dir.opposite()
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
        assert_eq!(
            zone.zone_type(),
            ZoneType::Interior,
            "{:?} should be Interior",
            zone
        );
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
        assert_eq!(
            zone.zone_type(),
            ZoneType::Exterior,
            "{:?} should be Exterior",
            zone
        );
    }
}

// ── Zone encounter generation ─────────────────────────────────────────────────

#[test]
fn zone_enter_can_produce_encounter() {
    use carbonthrone::zone::Zone;
    let mut r = rng();
    let found = (0..50).any(|_| Zone::enter(ZoneKind::DockingBay, 1, &mut r).has_encounter());
    assert!(found, "expected at least one encounter in 50 attempts");
}

#[test]
fn zone_enter_can_skip_encounter() {
    use carbonthrone::zone::Zone;
    let mut r = rng();
    let found = (0..50).any(|_| !Zone::enter(ZoneKind::DockingBay, 1, &mut r).has_encounter());
    assert!(
        found,
        "expected at least one zone without encounter in 50 attempts"
    );
}

#[test]
fn zone_enter_encounter_has_enemies() {
    use carbonthrone::zone::Zone;
    let mut r = rng();
    for _ in 0..50 {
        let zone = Zone::enter(ZoneKind::DockingBay, 1, &mut r);
        if zone.has_encounter() {
            assert!(!zone.generate_enemies(&mut r).is_empty(), "encounter had no enemies");
            return;
        }
    }
    panic!("could not find zone with encounter in 50 attempts");
}

#[test]
fn zone_enter_enemies_come_from_zone_pool() {
    use carbonthrone::zone::Zone;
    let mut r = rng();
    let pool = ZoneKind::ResearchWing.enemy_pool();
    for _ in 0..50 {
        let zone = Zone::enter(ZoneKind::ResearchWing, 1, &mut r);
        if zone.has_encounter() {
            for (enemy, _) in zone.generate_enemies(&mut r) {
                assert!(
                    pool.contains(&enemy.kind),
                    "ResearchWing spawned unexpected enemy kind: {:?}",
                    enemy.kind
                );
            }
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
    let mut r = rng();
    for _ in 0..50 {
        let zone = Zone::enter(ZoneKind::MilitaryAnnex, depth, &mut r);
        if zone.has_encounter() {
            assert!(zone.generate_enemies(&mut r).iter().all(|(e, _)| e.level == depth));
            return;
        }
    }
    panic!("could not find zone with encounter in 50 attempts");
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
        assert_eq!(zone.kind, kind, "{:?} zone was not created correctly", kind);
    }
}

// ── NPC phase-shift logic ─────────────────────────────────────────────────────

#[test]
fn npcs_available_when_no_encounter() {
    use carbonthrone::zone::Zone;
    let mut r = rng();
    for _ in 0..50 {
        let zone = Zone::enter(ZoneKind::DockingBay, 1, &mut r);
        if !zone.has_encounter() {
            assert!(
                zone.npcs_available(false),
                "NPCs should be available with no encounter"
            );
            return;
        }
    }
    panic!("could not find zone without encounter in 50 attempts");
}

#[test]
fn npcs_not_available_during_active_encounter() {
    use carbonthrone::zone::Zone;
    let mut r = rng();
    for _ in 0..50 {
        let zone = Zone::enter(ZoneKind::DockingBay, 1, &mut r);
        if zone.has_encounter() {
            assert!(
                !zone.npcs_available(false),
                "NPCs should be hidden during an active encounter"
            );
            return;
        }
    }
    panic!("could not find zone with encounter in 50 attempts");
}

#[test]
fn npcs_available_after_encounter_cleared() {
    use carbonthrone::zone::Zone;
    let mut r = rng();
    for _ in 0..50 {
        let zone = Zone::enter(ZoneKind::DockingBay, 1, &mut r);
        if zone.has_encounter() {
            assert!(
                zone.npcs_available(true),
                "NPCs should phase-shift in after encounter is cleared"
            );
            return;
        }
    }
    panic!("could not find zone with encounter in 50 attempts");
}
