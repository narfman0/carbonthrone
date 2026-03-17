use carbonthrone::character::{Aggression, CharacterKind};
use carbonthrone::game::zone_npcs;
use carbonthrone::zone::{
    CardinalDir, Zone, ZoneKind, ZoneType, encounter_chance, zone_connections,
};
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

// ── Encounter chance scaling ──────────────────────────────────────────────────

#[test]
fn encounter_chance_increases_with_loop() {
    assert!(encounter_chance(1) < encounter_chance(2));
    assert!(encounter_chance(2) < encounter_chance(3));
    assert!(encounter_chance(3) < encounter_chance(4));
    assert!(encounter_chance(4) < encounter_chance(5));
}

#[test]
fn encounter_chance_loop1_is_40_percent() {
    assert!((encounter_chance(1) - 0.40).abs() < 1e-9);
}

#[test]
fn encounter_chance_loop5_is_capped() {
    assert!(encounter_chance(5) <= 0.90);
    assert!(encounter_chance(100) <= 0.90);
}

// ── Zone encounter generation ─────────────────────────────────────────────────

#[test]
fn zone_enter_can_produce_encounter() {
    let mut r = rng();
    let found = (0..50).any(|_| Zone::enter(ZoneKind::DockingBay, 1, 1, &mut r).has_encounter());
    assert!(found, "expected at least one encounter in 50 attempts");
}

#[test]
fn zone_enter_can_skip_encounter() {
    let mut r = rng();
    let found = (0..50).any(|_| !Zone::enter(ZoneKind::DockingBay, 1, 1, &mut r).has_encounter());
    assert!(
        found,
        "expected at least one zone without encounter in 50 attempts"
    );
}

#[test]
fn zone_enter_encounter_has_enemies() {
    let mut r = rng();
    for _ in 0..50 {
        let zone = Zone::enter(ZoneKind::DockingBay, 1, 1, &mut r);
        if zone.has_encounter() {
            assert!(
                !zone.generate_enemies(&mut r).is_empty(),
                "encounter had no enemies"
            );
            return;
        }
    }
    panic!("could not find zone with encounter in 50 attempts");
}

#[test]
fn zone_enter_enemies_come_from_zone_pool() {
    let mut r = rng();
    let pool = ZoneKind::ResearchWing.enemy_pool();
    for _ in 0..50 {
        let zone = Zone::enter(ZoneKind::ResearchWing, 1, 1, &mut r);
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
    let zone = Zone::enter(ZoneKind::RelayArray, 3, 1, &mut rng());
    assert_eq!(zone.kind, ZoneKind::RelayArray);
}

#[test]
fn zone_connections_stored_on_zone() {
    let zone = Zone::enter(ZoneKind::RelayArray, 1, 1, &mut rng());
    assert_eq!(zone.connections.south, Some(ZoneKind::StationExterior));
}

#[test]
fn enemy_level_matches_depth_in_zone() {
    let depth = 4;
    let mut r = rng();
    for _ in 0..50 {
        let zone = Zone::enter(ZoneKind::MilitaryAnnex, depth, 1, &mut r);
        if zone.has_encounter() {
            assert!(
                zone.generate_enemies(&mut r)
                    .iter()
                    .all(|(e, _)| e.level == depth)
            );
            return;
        }
    }
    panic!("could not find zone with encounter in 50 attempts");
}

#[test]
fn all_zones_can_be_entered() {
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
        let zone = Zone::enter(kind, 1, 1, &mut r);
        assert_eq!(zone.kind, kind, "{:?} zone was not created correctly", kind);
    }
}

// ── NPC phase-shift logic ─────────────────────────────────────────────────────

#[test]
fn npcs_available_when_no_encounter() {
    let mut r = rng();
    for _ in 0..50 {
        let zone = Zone::enter(ZoneKind::DockingBay, 1, 1, &mut r);
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
    let mut r = rng();
    for _ in 0..50 {
        let zone = Zone::enter(ZoneKind::DockingBay, 1, 1, &mut r);
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
    let mut r = rng();
    for _ in 0..50 {
        let zone = Zone::enter(ZoneKind::DockingBay, 1, 1, &mut r);
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

// ── Loop-aware enemy pool ─────────────────────────────────────────────────────

#[test]
fn docking_bay_loop1_has_only_scavengers() {
    let pool = ZoneKind::DockingBay.combat_enemy_pool(1);
    assert!(pool.contains(&CharacterKind::Scavenger));
    assert!(
        !pool.contains(&CharacterKind::VoidRaider),
        "Raiders arrive loop 2+"
    );
    assert!(
        !pool.contains(&CharacterKind::DrifterBoss),
        "Boss arrives loop 3+"
    );
}

#[test]
fn docking_bay_loop2_adds_void_raiders() {
    let pool = ZoneKind::DockingBay.combat_enemy_pool(2);
    assert!(pool.contains(&CharacterKind::Scavenger));
    assert!(pool.contains(&CharacterKind::VoidRaider));
    assert!(!pool.contains(&CharacterKind::DrifterBoss));
}

#[test]
fn docking_bay_loop3_adds_drifter_boss() {
    let pool = ZoneKind::DockingBay.combat_enemy_pool(3);
    assert!(pool.contains(&CharacterKind::DrifterBoss));
}

#[test]
fn excavation_site_loop1_only_moon_crawlers() {
    let pool = ZoneKind::ExcavationSite.combat_enemy_pool(1);
    assert!(pool.contains(&CharacterKind::MoonCrawler));
    assert!(!pool.contains(&CharacterKind::VoidSpitter));
    assert!(!pool.contains(&CharacterKind::AbyssalBrute));
}

#[test]
fn excavation_site_loop2_adds_spitters_and_brutes() {
    let pool = ZoneKind::ExcavationSite.combat_enemy_pool(2);
    assert!(pool.contains(&CharacterKind::VoidSpitter));
    assert!(pool.contains(&CharacterKind::AbyssalBrute));
}

#[test]
fn medical_bay_loop1_fallback_scavengers() {
    let pool = ZoneKind::MedicalBay.combat_enemy_pool(1);
    assert!(
        !pool.is_empty(),
        "medical bay loop 1 pool must not be empty"
    );
    assert!(pool.contains(&CharacterKind::Scavenger));
}

#[test]
fn medical_bay_loop4_adds_void_spitters() {
    let pool = ZoneKind::MedicalBay.combat_enemy_pool(4);
    assert!(pool.contains(&CharacterKind::VoidSpitter));
}

#[test]
fn medical_bay_loop5_adds_abyssal_brute() {
    let pool = ZoneKind::MedicalBay.combat_enemy_pool(5);
    assert!(pool.contains(&CharacterKind::AbyssalBrute));
}

#[test]
fn military_annex_loop1_has_gun_for_hire() {
    let pool = ZoneKind::MilitaryAnnex.combat_enemy_pool(1);
    assert!(pool.contains(&CharacterKind::GunForHire));
    assert!(!pool.contains(&CharacterKind::ShockTrooper));
}

#[test]
fn military_annex_loop3_adds_shock_troopers() {
    let pool = ZoneKind::MilitaryAnnex.combat_enemy_pool(3);
    assert!(pool.contains(&CharacterKind::ShockTrooper));
}

#[test]
fn research_wing_loop1_has_constancy() {
    let pool = ZoneKind::ResearchWing.combat_enemy_pool(1);
    assert!(pool.contains(&CharacterKind::Zealot));
    assert!(pool.contains(&CharacterKind::Purifier));
    assert!(
        !pool.contains(&CharacterKind::Preacher),
        "Preachers arrive loop 2+"
    );
}

#[test]
fn research_wing_loop2_adds_scavengers_and_preachers() {
    let pool = ZoneKind::ResearchWing.combat_enemy_pool(2);
    assert!(pool.contains(&CharacterKind::Scavenger));
    assert!(pool.contains(&CharacterKind::Preacher));
}

#[test]
fn all_combat_pools_are_nonempty_for_all_loops() {
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
        ZoneKind::Hallway,
    ];
    for loop_number in 1..=5 {
        for zone in all_zones {
            let pool = zone.combat_enemy_pool(loop_number);
            assert!(
                !pool.is_empty(),
                "{:?} loop {} has empty combat pool",
                zone,
                loop_number
            );
        }
    }
}

// ── Loop-aware aggression ─────────────────────────────────────────────────────

#[test]
fn loop_aggression_maintenance_drone_neutral_early_hostile_late() {
    use carbonthrone::character::{Aggression, loop_aggression};
    assert_eq!(
        loop_aggression(&CharacterKind::MaintenanceDrone, 1),
        Aggression::Neutral
    );
    assert_eq!(
        loop_aggression(&CharacterKind::MaintenanceDrone, 2),
        Aggression::Neutral
    );
    assert_eq!(
        loop_aggression(&CharacterKind::MaintenanceDrone, 3),
        Aggression::Aggressive
    );
}

#[test]
fn loop_aggression_station_guard_flips_over_loops() {
    use carbonthrone::character::{Aggression, loop_aggression};
    assert_eq!(
        loop_aggression(&CharacterKind::StationGuard, 1),
        Aggression::Friendly
    );
    assert_eq!(
        loop_aggression(&CharacterKind::StationGuard, 3),
        Aggression::Neutral
    );
    assert_eq!(
        loop_aggression(&CharacterKind::StationGuard, 4),
        Aggression::Aggressive
    );
}

#[test]
fn loop_aggression_abyssal_fauna_lethargic_in_late_loops() {
    use carbonthrone::character::{Aggression, loop_aggression};
    assert_eq!(
        loop_aggression(&CharacterKind::MoonCrawler, 3),
        Aggression::Aggressive
    );
    assert_eq!(
        loop_aggression(&CharacterKind::MoonCrawler, 4),
        Aggression::Lethargic
    );
    assert_eq!(
        loop_aggression(&CharacterKind::AbyssalBrute, 5),
        Aggression::Lethargic
    );
}

#[test]
fn generated_enemies_have_loop_appropriate_aggression() {
    use carbonthrone::character::Aggression;
    let mut r = rng();
    // In loop 3, Maintenance Drones in SystemsCore should be Aggressive
    for _ in 0..100 {
        let zone = Zone::enter(ZoneKind::SystemsCore, 3, 3, &mut r);
        if zone.has_encounter() {
            for (enemy, _) in zone.generate_enemies(&mut r) {
                if enemy.kind == CharacterKind::MaintenanceDrone {
                    assert_eq!(
                        enemy.aggression,
                        Aggression::Aggressive,
                        "Drones should be Aggressive in loop 3"
                    );
                }
            }
            return;
        }
    }
}

// ── zone_npcs: per-zone NPC placement ────────────────────────────────────────

fn empty_flags() -> std::collections::HashSet<String> {
    std::collections::HashSet::new()
}

fn flags_with(items: &[&str]) -> std::collections::HashSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

#[test]
fn research_wing_has_salvage_op_loops_1_and_2() {
    for loop_n in 1..=2 {
        let npcs = zone_npcs(ZoneKind::ResearchWing, 20, 20, loop_n, &empty_flags());
        assert!(
            npcs.iter().any(|n| n.kind == CharacterKind::SalvageOperative),
            "ResearchWing loop {loop_n} should have a Salvage Operative"
        );
    }
}

#[test]
fn research_wing_has_no_npcs_loop_3_plus() {
    for loop_n in 3..=5 {
        let npcs = zone_npcs(ZoneKind::ResearchWing, 20, 20, loop_n, &empty_flags());
        assert!(
            npcs.is_empty(),
            "ResearchWing loop {loop_n} should have no NPCs (operative has left)"
        );
    }
}

#[test]
fn command_deck_has_orin_unless_companion() {
    let npcs = zone_npcs(ZoneKind::CommandDeck, 20, 20, 1, &empty_flags());
    assert!(
        npcs.iter().any(|n| n.kind == CharacterKind::Orin),
        "CommandDeck should have Orin when not a companion"
    );
}

#[test]
fn command_deck_no_orin_when_companion_flag() {
    let flags = flags_with(&["companion_orin"]);
    let npcs = zone_npcs(ZoneKind::CommandDeck, 20, 20, 1, &flags);
    assert!(
        !npcs.iter().any(|n| n.kind == CharacterKind::Orin),
        "CommandDeck should not have Orin when companion_orin flag is set"
    );
}

#[test]
fn military_annex_has_doss_unless_companion() {
    let npcs = zone_npcs(ZoneKind::MilitaryAnnex, 20, 20, 1, &empty_flags());
    assert!(
        npcs.iter().any(|n| n.kind == CharacterKind::Doss),
        "MilitaryAnnex should have Doss when not a companion"
    );
}

#[test]
fn military_annex_no_doss_when_companion_flag() {
    let flags = flags_with(&["companion_doss"]);
    let npcs = zone_npcs(ZoneKind::MilitaryAnnex, 20, 20, 1, &flags);
    assert!(
        !npcs.iter().any(|n| n.kind == CharacterKind::Doss),
        "MilitaryAnnex should not have Doss when companion_doss flag is set"
    );
}

#[test]
fn systems_core_no_kaleo_loop_1() {
    let npcs = zone_npcs(ZoneKind::SystemsCore, 20, 20, 1, &empty_flags());
    assert!(
        !npcs.iter().any(|n| n.kind == CharacterKind::Kaleo),
        "SystemsCore loop 1 should not have Kaleo (not yet active)"
    );
}

#[test]
fn systems_core_has_kaleo_loop_2_plus() {
    for loop_n in 2..=5 {
        let npcs = zone_npcs(ZoneKind::SystemsCore, 20, 20, loop_n, &empty_flags());
        assert!(
            npcs.iter().any(|n| n.kind == CharacterKind::Kaleo),
            "SystemsCore loop {loop_n} should have Kaleo"
        );
    }
}

#[test]
fn systems_core_no_kaleo_when_recruited() {
    let flags = flags_with(&["kaleo_recruited"]);
    let npcs = zone_npcs(ZoneKind::SystemsCore, 20, 20, 3, &flags);
    assert!(
        !npcs.iter().any(|n| n.kind == CharacterKind::Kaleo),
        "SystemsCore should not have Kaleo when kaleo_recruited flag is set"
    );
}

#[test]
fn medical_bay_has_salvage_op_and_guard_loops_1_2() {
    for loop_n in 1..=2 {
        let npcs = zone_npcs(ZoneKind::MedicalBay, 20, 20, loop_n, &empty_flags());
        assert!(
            npcs.iter().any(|n| n.kind == CharacterKind::SalvageOperative),
            "MedicalBay loop {loop_n} should have a Salvage Operative"
        );
        assert!(
            npcs.iter().any(|n| n.kind == CharacterKind::StationGuard),
            "MedicalBay loop {loop_n} should have a Station Guard"
        );
    }
}

#[test]
fn medical_bay_guard_gone_loop_3_plus() {
    for loop_n in 3..=5 {
        let npcs = zone_npcs(ZoneKind::MedicalBay, 20, 20, loop_n, &empty_flags());
        assert!(
            !npcs.iter().any(|n| n.kind == CharacterKind::StationGuard),
            "MedicalBay loop {loop_n} should have no Station Guard (gone or hostile)"
        );
    }
}

#[test]
fn docking_bay_has_gun_for_hire_loops_1_to_4() {
    for loop_n in 1..=4 {
        let npcs = zone_npcs(ZoneKind::DockingBay, 20, 20, loop_n, &empty_flags());
        assert!(
            npcs.iter().any(|n| n.kind == CharacterKind::GunForHire),
            "DockingBay loop {loop_n} should have Gun-for-Hire"
        );
    }
}

#[test]
fn docking_bay_no_gun_for_hire_loop_5() {
    let npcs = zone_npcs(ZoneKind::DockingBay, 20, 20, 5, &empty_flags());
    assert!(
        !npcs.iter().any(|n| n.kind == CharacterKind::GunForHire),
        "DockingBay loop 5 should have no Gun-for-Hire (flipped by Doss)"
    );
}

#[test]
fn station_exterior_salvage_op_only_loop_3() {
    let npcs_l3 = zone_npcs(ZoneKind::StationExterior, 20, 20, 3, &empty_flags());
    assert!(
        npcs_l3.iter().any(|n| n.kind == CharacterKind::SalvageOperative),
        "StationExterior loop 3 should have the last Salvage Operative"
    );
    for loop_n in [1, 2, 4, 5] {
        let npcs = zone_npcs(ZoneKind::StationExterior, 20, 20, loop_n, &empty_flags());
        assert!(
            npcs.is_empty(),
            "StationExterior loop {loop_n} should have no NPCs"
        );
    }
}

#[test]
fn relay_array_and_excavation_site_always_empty() {
    for loop_n in 1..=5 {
        assert!(
            zone_npcs(ZoneKind::RelayArray, 20, 20, loop_n, &empty_flags()).is_empty(),
            "RelayArray loop {loop_n} should have no NPCs"
        );
        assert!(
            zone_npcs(ZoneKind::ExcavationSite, 20, 20, loop_n, &empty_flags()).is_empty(),
            "ExcavationSite loop {loop_n} should have no NPCs"
        );
    }
}

#[test]
fn npc_aggression_derives_from_loop_aggression() {
    // Salvage Operative: Friendly loops 1-2, Neutral loop 3.
    let npcs_l1 = zone_npcs(ZoneKind::MedicalBay, 20, 20, 1, &empty_flags());
    let op = npcs_l1
        .iter()
        .find(|n| n.kind == CharacterKind::SalvageOperative)
        .expect("should have salvage op in loop 1");
    assert_eq!(op.aggression, Aggression::Friendly);

    let npcs_l3 = zone_npcs(ZoneKind::MedicalBay, 20, 20, 3, &empty_flags());
    let op3 = npcs_l3
        .iter()
        .find(|n| n.kind == CharacterKind::SalvageOperative)
        .expect("should have salvage op in loop 3");
    assert_eq!(op3.aggression, Aggression::Neutral);
}

#[test]
fn npc_positions_are_distinct_when_multiple_in_zone() {
    // Medical Bay loop 1 has two NPCs — they must not share a grid tile.
    let npcs = zone_npcs(ZoneKind::MedicalBay, 20, 20, 1, &empty_flags());
    assert_eq!(npcs.len(), 2);
    assert_ne!(
        npcs[0].pos, npcs[1].pos,
        "two NPCs in the same zone must not occupy the same tile"
    );
}

#[test]
fn npc_positions_clamped_to_grid_interior() {
    // Even with a tiny grid the positions should stay within bounds.
    for loop_n in 1..=5 {
        for &kind in &[
            ZoneKind::ResearchWing,
            ZoneKind::CommandDeck,
            ZoneKind::MilitaryAnnex,
            ZoneKind::SystemsCore,
            ZoneKind::MedicalBay,
            ZoneKind::DockingBay,
            ZoneKind::StationExterior,
        ] {
            let cols = 10u32;
            let rows = 10u32;
            for npc in zone_npcs(kind, cols, rows, loop_n, &empty_flags()) {
                assert!(
                    npc.pos.0 >= 1 && npc.pos.0 <= cols as i32 - 2,
                    "{kind:?} loop {loop_n} NPC x={} out of bounds for cols={cols}",
                    npc.pos.0
                );
                assert!(
                    npc.pos.1 >= 1 && npc.pos.1 <= rows as i32 - 2,
                    "{kind:?} loop {loop_n} NPC y={} out of bounds for rows={rows}",
                    npc.pos.1
                );
            }
        }
    }
}

#[test]
fn higher_loop_produces_more_encounters_on_average() {
    let mut r = rng();
    let loop1_encounters: u32 = (0..200)
        .map(|_| Zone::enter(ZoneKind::StationExterior, 1, 1, &mut r).has_encounter() as u32)
        .sum();
    let loop5_encounters: u32 = (0..200)
        .map(|_| Zone::enter(ZoneKind::StationExterior, 1, 5, &mut r).has_encounter() as u32)
        .sum();
    assert!(
        loop5_encounters > loop1_encounters,
        "loop 5 ({} encounters) should have more than loop 1 ({} encounters) over 200 tries",
        loop5_encounters,
        loop1_encounters
    );
}
