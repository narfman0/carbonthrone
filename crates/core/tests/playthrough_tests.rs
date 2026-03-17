use carbonthrone::character::CharacterKind;
use carbonthrone::console::defeat_all_enemies;
use carbonthrone::game::{EndingKind, GamePhase, GameSession};
use carbonthrone::save::SaveData;
use carbonthrone::zone::ZoneKind;
use rand::SeedableRng;
use rand::rngs::StdRng;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn save_at_loop5() -> SaveData {
    SaveData {
        loop_number: 5,
        flags: vec![],
        active_companion: None,
        current_zone: ZoneKind::ResearchWing,
        party_kinds: vec![CharacterKind::Researcher],
        party_hp: vec![],
        party_levels: vec![],
        fought_scripted_encounters: vec![],
        ending: None,
        battle_snapshot: None,
        last_dialog_lines: std::collections::HashMap::new(),
    }
}

/// If the session is in Battle, defeat all enemies and transition back.
/// Also clears the post-battle combat_end dialog.
fn resolve_any_battle(session: &mut GameSession) {
    if matches!(session.phase, GamePhase::Battle(_)) {
        defeat_all_enemies(&mut session.world);
        session.transition_to_exploration();
        session.dismiss_dialog(); // clears the combat_end dialog fired by transition
    }
}

/// Dismiss pending dialog (no-op if in Battle), then resolve any resulting battle.
fn dismiss_and_resolve(session: &mut GameSession) {
    session.dismiss_dialog();
    resolve_any_battle(session);
}

// ── Smoke tests: each flag triggers the correct EndingKind at loop 5 ──────────

#[test]
fn end_dialog_loop5_orin_helped() {
    for seed in 0..200u64 {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut session = GameSession::from_save_data(save_at_loop5(), &mut rng);
        dismiss_and_resolve(&mut session);
        if !matches!(session.phase, GamePhase::Exploration(_)) {
            continue;
        }
        session.end_dialog(vec!["orin_helped".to_string()]);
        assert!(
            matches!(session.phase, GamePhase::Ended(EndingKind::SableDestroyed)),
            "orin_helped at loop 5 should produce SableDestroyed"
        );
        return;
    }
    panic!("no usable loop-5 exploration state found in 200 seeds");
}

#[test]
fn end_dialog_loop5_ending_doss() {
    for seed in 0..200u64 {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut session = GameSession::from_save_data(save_at_loop5(), &mut rng);
        dismiss_and_resolve(&mut session);
        if !matches!(session.phase, GamePhase::Exploration(_)) {
            continue;
        }
        session.end_dialog(vec!["ending_doss".to_string()]);
        assert!(
            matches!(session.phase, GamePhase::Ended(EndingKind::DossPreserved)),
            "ending_doss at loop 5 should produce DossPreserved"
        );
        return;
    }
    panic!("no usable loop-5 exploration state found in 200 seeds");
}

#[test]
fn end_dialog_loop5_ending_kaleo() {
    for seed in 0..200u64 {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut session = GameSession::from_save_data(save_at_loop5(), &mut rng);
        dismiss_and_resolve(&mut session);
        if !matches!(session.phase, GamePhase::Exploration(_)) {
            continue;
        }
        session.end_dialog(vec!["ending_kaleo".to_string()]);
        assert!(
            matches!(session.phase, GamePhase::Ended(EndingKind::KaleoCompromise)),
            "ending_kaleo at loop 5 should produce KaleoCompromise"
        );
        return;
    }
    panic!("no usable loop-5 exploration state found in 200 seeds");
}

#[test]
fn end_dialog_loop5_orin_stopped() {
    for seed in 0..200u64 {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut session = GameSession::from_save_data(save_at_loop5(), &mut rng);
        dismiss_and_resolve(&mut session);
        if !matches!(session.phase, GamePhase::Exploration(_)) {
            continue;
        }
        session.end_dialog(vec!["orin_stopped".to_string()]);
        assert!(
            matches!(session.phase, GamePhase::Ended(EndingKind::SableStopped)),
            "orin_stopped at loop 5 (no convergence) should produce SableStopped"
        );
        return;
    }
    panic!("no usable loop-5 exploration state found in 200 seeds");
}

#[test]
fn end_dialog_loop5_orin_stopped_plus_convergence_does_not_end() {
    for seed in 0..200u64 {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut session = GameSession::from_save_data(save_at_loop5(), &mut rng);
        dismiss_and_resolve(&mut session);
        if !matches!(session.phase, GamePhase::Exploration(_)) {
            continue;
        }
        session.end_dialog(vec![
            "orin_stopped".to_string(),
            "convergence_triggered".to_string(),
        ]);
        assert!(
            !matches!(session.phase, GamePhase::Ended(_)),
            "orin_stopped + convergence_triggered should NOT produce an ending"
        );
        return;
    }
    panic!("no usable loop-5 exploration state found in 200 seeds");
}

#[test]
fn end_dialog_below_loop5_never_ends() {
    // All four ending flags at loop 4 should produce no ending.
    let all_flags = vec![
        "orin_helped".to_string(),
        "ending_doss".to_string(),
        "ending_kaleo".to_string(),
        "orin_stopped".to_string(),
    ];
    for seed in 0..200u64 {
        let mut rng = StdRng::seed_from_u64(seed);
        let save = SaveData {
            loop_number: 4,
            flags: vec![],
            active_companion: None,
            current_zone: ZoneKind::ResearchWing,
            party_kinds: vec![CharacterKind::Researcher],
            party_hp: vec![],
            party_levels: vec![],
            fought_scripted_encounters: vec![],
            ending: None,
            battle_snapshot: None,
            last_dialog_lines: std::collections::HashMap::new(),
        };
        let mut session = GameSession::from_save_data(save, &mut rng);
        dismiss_and_resolve(&mut session);
        if !matches!(session.phase, GamePhase::Exploration(_)) {
            continue;
        }
        session.end_dialog(all_flags.clone());
        assert!(
            !matches!(session.phase, GamePhase::Ended(_)),
            "ending flags at loop 4 should NOT produce an ending"
        );
        return;
    }
    panic!("no usable loop-4 exploration state found in 200 seeds");
}

// ── Edge cases ────────────────────────────────────────────────────────────────

#[test]
fn reset_loop_clamps_at_5() {
    let mut session = GameSession::new();
    session.loop_number = 5;
    dismiss_and_resolve(&mut session);
    let GamePhase::Exploration(_) = &session.phase else {
        return;
    };
    let mut rng = StdRng::seed_from_u64(0);
    session.reset_loop(&mut rng);
    assert_eq!(session.loop_number, 5, "loop_number should not exceed 5");
}

#[test]
fn reset_loop_increments_correctly() {
    let mut session = GameSession::new();
    dismiss_and_resolve(&mut session);
    assert_eq!(session.loop_number, 1);
    let mut rng = StdRng::seed_from_u64(0);
    for expected in 2u32..=5 {
        let GamePhase::Exploration(_) = &session.phase else {
            break;
        };
        session.reset_loop(&mut rng);
        assert_eq!(
            session.loop_number, expected,
            "loop_number should be {expected} after reset"
        );
        dismiss_and_resolve(&mut session);
    }
}

// ── Full playthrough tests ────────────────────────────────────────────────────

/// Advance a session from loop 1 through loop 5 with a fixed seed, applying
/// dismiss_and_resolve at each step.  Returns an Exploration-phase session
/// at loop 5 ready for ending injection.
fn advance_to_loop5(seed: u64) -> GameSession {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut session = GameSession::new();
    dismiss_and_resolve(&mut session);
    assert_eq!(session.loop_number, 1);

    for target in 2u32..=5 {
        let GamePhase::Exploration(_) = &session.phase else {
            panic!("expected Exploration before reset_loop to loop {target}");
        };
        session.reset_loop(&mut rng);
        assert_eq!(session.loop_number, target);
        dismiss_and_resolve(&mut session);
        assert!(
            matches!(session.phase, GamePhase::Exploration(_)),
            "expected Exploration after dismiss_and_resolve at loop {target}"
        );
    }
    session
}

#[test]
fn full_playthrough_loop1_to_loop5_sable_destroyed() {
    let mut session = advance_to_loop5(12345);
    session.end_dialog(vec!["orin_helped".to_string()]);
    assert!(matches!(
        session.phase,
        GamePhase::Ended(EndingKind::SableDestroyed)
    ));
}

#[test]
fn full_playthrough_loop1_to_loop5_doss_preserved() {
    let mut session = advance_to_loop5(99999);
    session.end_dialog(vec!["ending_doss".to_string()]);
    assert!(matches!(
        session.phase,
        GamePhase::Ended(EndingKind::DossPreserved)
    ));
}

#[test]
fn full_playthrough_loop1_to_loop5_kaleo_compromise() {
    let mut session = advance_to_loop5(55555);
    session.end_dialog(vec!["ending_kaleo".to_string()]);
    assert!(matches!(
        session.phase,
        GamePhase::Ended(EndingKind::KaleoCompromise)
    ));
}
