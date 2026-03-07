use carbonthrone::game::{GamePhase, GameSession};
use carbonthrone::position::Position;
use carbonthrone::travel::{TravelState, arrival_chance};
use carbonthrone::zone::{CardinalDir, ZoneKind};
use rand::SeedableRng;
use rand::rngs::StdRng;

// ── arrival_chance ────────────────────────────────────────────────────────────

#[test]
fn arrival_chance_values() {
    assert!((arrival_chance(1) - 0.80).abs() < 1e-9);
    assert!((arrival_chance(2) - 0.65).abs() < 1e-9);
    assert!((arrival_chance(3) - 0.50).abs() < 1e-9);
    assert!((arrival_chance(4) - 0.35).abs() < 1e-9);
    assert!((arrival_chance(5) - 0.20).abs() < 1e-9);
}

#[test]
fn arrival_chance_clamped_at_floor() {
    assert!((arrival_chance(10) - 0.10).abs() < 1e-9);
    assert!((arrival_chance(100) - 0.10).abs() < 1e-9);
}

// ── TravelState ───────────────────────────────────────────────────────────────

#[test]
fn travel_state_initial_hallways_zero() {
    let t = TravelState::new(
        ZoneKind::CommandDeck,
        ZoneKind::ResearchWing,
        CardinalDir::South,
    );
    assert_eq!(t.origin, ZoneKind::CommandDeck);
    assert_eq!(t.destination, ZoneKind::ResearchWing);
    assert_eq!(t.hallways_traversed, 0);
    assert_eq!(t.travel_dir, CardinalDir::South);
}

// ── initiate_travel ───────────────────────────────────────────────────────────

#[test]
fn initiate_travel_enters_hallway() {
    let mut session = GameSession::new();
    let mut rng = StdRng::seed_from_u64(42);
    session.initiate_travel(ZoneKind::ResearchWing, &mut rng);

    let GamePhase::Exploration(state) = &session.phase else {
        panic!("expected Exploration phase");
    };
    assert_eq!(state.zone.kind, ZoneKind::Hallway);
    assert!(state.travel.is_some());
    assert_eq!(
        state.travel.as_ref().unwrap().destination,
        ZoneKind::ResearchWing
    );
}

#[test]
fn initiate_travel_noop_if_already_traveling() {
    let mut session = GameSession::new();
    let mut rng = StdRng::seed_from_u64(1);
    session.initiate_travel(ZoneKind::ResearchWing, &mut rng);
    // Second call should be ignored.
    session.initiate_travel(ZoneKind::MedicalBay, &mut rng);

    let GamePhase::Exploration(state) = &session.phase else {
        panic!("expected Exploration phase");
    };
    // Destination unchanged from the first call.
    assert_eq!(
        state.travel.as_ref().unwrap().destination,
        ZoneKind::ResearchWing
    );
}

// ── exit_hallway ──────────────────────────────────────────────────────────────

#[test]
fn exit_hallway_noop_when_not_traveling() {
    let mut session = GameSession::new();
    let mut rng = StdRng::seed_from_u64(99);
    let arrived = session.exit_hallway(&mut rng);
    assert!(!arrived);
}

#[test]
fn exit_hallway_eventually_arrives_loop1() {
    let mut session = GameSession::new();
    session.loop_number = 1;
    let mut rng = StdRng::seed_from_u64(7);
    session.initiate_travel(ZoneKind::ResearchWing, &mut rng);

    // With loop 1 (80% chance), we should arrive within a small number of attempts.
    let mut arrived = false;
    for _ in 0..20 {
        if session.exit_hallway(&mut rng) {
            arrived = true;
            break;
        }
    }
    assert!(arrived, "should arrive within 20 attempts at loop 1");
}

#[test]
fn exit_hallway_arrives_at_correct_zone() {
    // Use a seed where the first roll succeeds (>= 80% chance at loop 1).
    // Scan seeds until we find one that succeeds on the first exit.
    for seed in 0u64..1000 {
        let mut session = GameSession::new();
        session.loop_number = 1;
        let mut rng = StdRng::seed_from_u64(seed);
        session.initiate_travel(ZoneKind::MedicalBay, &mut rng);
        if session.exit_hallway(&mut rng) {
            let GamePhase::Exploration(state) = &session.phase else {
                continue;
            };
            assert_eq!(state.zone.kind, ZoneKind::MedicalBay);
            assert!(state.travel.is_none());
            return;
        }
    }
    panic!("no seed produced an immediate arrival in 1000 tries");
}

#[test]
fn exit_hallway_increments_hallways_traversed_on_miss() {
    // Find a seed that fails the first exit at loop 5 (only 20% chance).
    for seed in 0u64..1000 {
        let mut session = GameSession::new();
        session.loop_number = 5;
        let mut rng = StdRng::seed_from_u64(seed);
        session.initiate_travel(ZoneKind::DockingBay, &mut rng);
        if !session.exit_hallway(&mut rng) {
            let GamePhase::Exploration(state) = &session.phase else {
                continue;
            };
            assert_eq!(state.travel.as_ref().unwrap().hallways_traversed, 1);
            assert_eq!(state.zone.kind, ZoneKind::Hallway);
            return;
        }
    }
    panic!("no seed produced a miss in 1000 tries at loop 5");
}

// ── move_player edge-exit ─────────────────────────────────────────────────────

/// Dismiss any active dialog so movement is unblocked.
fn dismiss_dialog(session: &mut GameSession) {
    loop {
        let GamePhase::Exploration(s) = &mut session.phase else {
            break;
        };
        if !s.in_dialog {
            break;
        }
        s.advance_dialog();
    }
}

#[test]
fn move_player_off_edge_initiates_travel_when_connection_exists() {
    // CommandDeck's south connection is ResearchWing. Find the south door tile,
    // place the player one step north of it, then step south onto it.
    let mut session = GameSession::new();
    let mut rng = StdRng::seed_from_u64(42);
    dismiss_dialog(&mut session);

    let (player_entity, door_pos) = {
        let GamePhase::Exploration(s) = &session.phase else {
            panic!()
        };
        let door = s
            .zone
            .doors
            .iter()
            .find(|&(_, &dir)| dir == CardinalDir::South)
            .map(|(&pos, _)| pos)
            .expect("CommandDeck should have a south door");
        (s.player_entity, door)
    };
    // Place player one tile north of the door, then step south onto it.
    *session
        .world
        .get_mut::<Position>(player_entity)
        .expect("player has Position") = Position::new(door_pos.0, door_pos.1 - 1);
    session.move_player(0, 1, &mut rng);

    let GamePhase::Exploration(state) = &session.phase else {
        panic!("expected Exploration");
    };
    assert!(
        state.travel.is_some() || state.zone.kind != ZoneKind::CommandDeck,
        "stepping onto south door should initiate travel"
    );
}

#[test]
fn move_player_off_edge_no_op_when_no_connection() {
    // CommandDeck has no west connection. Teleport to x=0 and step west.
    let mut session = GameSession::new();
    let mut rng = StdRng::seed_from_u64(1);
    dismiss_dialog(&mut session);

    let player_entity = {
        let GamePhase::Exploration(s) = &session.phase else {
            panic!()
        };
        s.player_entity
    };
    *session
        .world
        .get_mut::<Position>(player_entity)
        .expect("player has Position") = Position::new(0, 0);
    session.move_player(-1, 0, &mut rng);

    let GamePhase::Exploration(state) = &session.phase else {
        panic!("expected Exploration");
    };
    assert!(
        state.travel.is_none(),
        "no west connection — travel should not start"
    );
    assert_eq!(state.zone.kind, ZoneKind::CommandDeck);
}

#[test]
fn move_player_off_hallway_edge_advances_travel() {
    // CommandDeck → ResearchWing = South direction.
    // Hallway exit door is on the south side. Place player one step north of it.
    let mut session = GameSession::new();
    session.loop_number = 1;
    let mut rng = StdRng::seed_from_u64(7);
    dismiss_dialog(&mut session);
    session.initiate_travel(ZoneKind::ResearchWing, &mut rng);

    let mut arrived = false;
    for _ in 0..20 {
        // Find the south (exit) door tile, place player one step north of it.
        let (player_entity, door_pos) = {
            let GamePhase::Exploration(s) = &session.phase else {
                panic!()
            };
            let door = s
                .zone
                .doors
                .iter()
                .find(|entry| *entry.1 == CardinalDir::South)
                .map(|entry| *entry.0)
                .expect("hallway should have a south exit door");
            (s.player_entity, door)
        };
        *session
            .world
            .get_mut::<Position>(player_entity)
            .expect("player has Position") = Position::new(door_pos.0, door_pos.1 - 1);

        session.move_player(0, 1, &mut rng);

        let GamePhase::Exploration(s) = &session.phase else {
            panic!()
        };
        if s.travel.is_none() {
            arrived = true;
            break;
        }
    }
    assert!(
        arrived,
        "should arrive within 20 hallway exits at loop 1 (80% chance)"
    );
}

// ── ZoneKind::Hallway ─────────────────────────────────────────────────────────

#[test]
fn hallway_zone_kind_metadata() {
    assert_eq!(ZoneKind::Hallway.location_id(), "hallway");
    assert_eq!(ZoneKind::Hallway.display_name(), "Connecting Corridor");
    assert!(!ZoneKind::Hallway.enemy_pool().is_empty());
}
