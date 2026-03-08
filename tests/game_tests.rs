use carbonthrone::character::CharacterKind;
use carbonthrone::game::{GamePhase, GameSession};
use carbonthrone::save::SaveData;
use carbonthrone::zone::ZoneKind;
use rand::SeedableRng;
use rand::rngs::StdRng;

// ── Helper ────────────────────────────────────────────────────────────────────

/// Build a minimal SaveData for the given zone and loop.
fn save_for(zone: ZoneKind, loop_number: u32) -> SaveData {
    SaveData {
        loop_number,
        flags: vec![],
        active_companion: None,
        current_zone: zone,
        party_kinds: vec![CharacterKind::Researcher],
        party_hp: vec![],
    }
}

// ── pending_battle set on zone entry ─────────────────────────────────────────

#[test]
fn pending_battle_true_when_zone_has_encounter() {
    // Loop 4 has 85% encounter chance — scan until we find a zone with one.
    for seed in 0u64..500 {
        let mut rng = StdRng::seed_from_u64(seed);
        let session = GameSession::from_save_data(save_for(ZoneKind::MedicalBay, 4), &mut rng);
        match &session.phase {
            GamePhase::Battle(_) => {
                // Encounter + no dialog → battle started immediately. Also valid.
                return;
            }
            GamePhase::Exploration(e) if e.zone.has_encounter() => {
                assert!(
                    e.pending_battle,
                    "pending_battle must be true when zone rolled an encounter"
                );
                return;
            }
            _ => continue,
        }
    }
    panic!("no seed produced a zone with encounter in 500 tries");
}

#[test]
fn pending_battle_false_when_zone_has_no_encounter() {
    // Loop 1 has only 40% encounter chance — plenty of no-encounter seeds.
    for seed in 0u64..500 {
        let mut rng = StdRng::seed_from_u64(seed);
        let session = GameSession::from_save_data(save_for(ZoneKind::MedicalBay, 1), &mut rng);
        let GamePhase::Exploration(e) = &session.phase else {
            continue;
        };
        if !e.zone.has_encounter() {
            assert!(
                !e.pending_battle,
                "pending_battle must be false when zone has no encounter"
            );
            return;
        }
    }
    panic!("no seed produced a zone without encounter in 500 tries");
}

// ── immediate battle when no dialog blocks ────────────────────────────────────

#[test]
fn encounter_starts_battle_immediately_when_no_dialog() {
    // Loop 4 MilitaryAnnex: no on_enter dialog in loop4 YAML, 85% encounter chance.
    // If encounter rolled → battle starts right away (no dialog to block it).
    for seed in 0u64..500 {
        let mut rng = StdRng::seed_from_u64(seed);
        let session = GameSession::from_save_data(save_for(ZoneKind::MilitaryAnnex, 4), &mut rng);
        if matches!(session.phase, GamePhase::Battle(_)) {
            return; // encounter + no dialog → immediate battle, as expected
        }
    }
    panic!("no seed produced immediate battle at loop 4 MilitaryAnnex in 500 tries");
}

// ── advance_dialog wrapper triggers battle ────────────────────────────────────

#[test]
fn advance_dialog_triggers_battle_when_pending() {
    // Loop 1 ResearchWing always fires an on_enter dialog.
    // Scan for a seed where the zone also has an encounter.
    for seed in 0u64..1000 {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut session =
            GameSession::from_save_data(save_for(ZoneKind::ResearchWing, 1), &mut rng);

        // Must be in Exploration with dialog open and a pending battle.
        let ok = match &session.phase {
            GamePhase::Exploration(e) => e.pending_battle && e.in_dialog,
            _ => false,
        };
        if !ok {
            continue;
        }

        // Advance through all dialog lines via the GameSession wrapper.
        loop {
            match &session.phase {
                GamePhase::Battle(_) => break,
                GamePhase::Exploration(e) if !e.in_dialog => break,
                GamePhase::Exploration(e) if e.at_choice_screen() => {
                    session.select_choice();
                    break;
                }
                _ => {
                    session.advance_dialog();
                }
            }
        }

        assert!(
            matches!(session.phase, GamePhase::Battle(_)),
            "battle should start after all dialog lines are advanced with pending_battle"
        );
        return;
    }
    panic!("no seed produced pending_battle+in_dialog at loop 1 ResearchWing in 1000 tries");
}

// ── select_choice wrapper triggers battle ─────────────────────────────────────

#[test]
fn select_choice_triggers_battle_when_pending() {
    // Find a session with pending_battle active, navigate to a choice screen,
    // and verify that selecting a terminal choice (no leads_to) starts battle.
    for seed in 0u64..2000 {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut session =
            GameSession::from_save_data(save_for(ZoneKind::ResearchWing, 1), &mut rng);

        let has_pending = match &session.phase {
            GamePhase::Exploration(e) => e.pending_battle && e.in_dialog,
            _ => false,
        };
        if !has_pending {
            continue;
        }

        // Advance to the choice screen (or end of dialog).
        loop {
            match &session.phase {
                GamePhase::Battle(_) => break,
                GamePhase::Exploration(e) if e.at_choice_screen() => break,
                GamePhase::Exploration(e) if !e.in_dialog => break,
                _ => {
                    session.advance_dialog();
                }
            }
        }

        match &session.phase {
            GamePhase::Battle(_) => return, // dialog advanced to battle directly
            GamePhase::Exploration(e) if e.at_choice_screen() => {}
            _ => continue, // dialog ended without battle and no choices — wrong seed
        }

        session.select_choice();

        // Either we're in Battle, or the choice led to a continuation scene.
        // If continuation, keep advancing until battle or dialog ends.
        loop {
            match &session.phase {
                GamePhase::Battle(_) => return,
                GamePhase::Exploration(e) if !e.in_dialog => break,
                GamePhase::Exploration(e) if e.at_choice_screen() => {
                    session.select_choice();
                }
                _ => {
                    session.advance_dialog();
                }
            }
        }
    }
    panic!("no seed produced a choice-closes-to-battle scenario in 2000 tries");
}

// ── reset_loop respects encounter ─────────────────────────────────────────────

#[test]
fn reset_loop_sets_pending_battle_on_encounter() {
    let mut session = GameSession::new();
    // Dismiss startup dialog via inner method (won't trigger battle).
    loop {
        let GamePhase::Exploration(e) = &mut session.phase else {
            break;
        };
        if !e.in_dialog {
            break;
        }
        e.advance_dialog();
    }

    // Only callable in Exploration phase; if somehow in Battle, skip.
    let GamePhase::Exploration(_) = &session.phase else {
        return;
    };

    for seed in 0u64..500 {
        let mut rng = StdRng::seed_from_u64(seed);
        session.reset_loop(&mut rng);
        match &session.phase {
            GamePhase::Battle(_) => {
                // Encounter + no dialog → battle started immediately. Valid.
                return;
            }
            GamePhase::Exploration(e) if e.zone.has_encounter() => {
                assert!(
                    e.pending_battle,
                    "pending_battle should be set after reset_loop when encounter exists"
                );
                return;
            }
            GamePhase::Exploration(e) if !e.zone.has_encounter() => {
                assert!(
                    !e.pending_battle,
                    "pending_battle should be false after reset_loop when no encounter"
                );
                // Try next seed for a more interesting case.
                continue;
            }
            _ => continue,
        }
    }
}

// ── exit_hallway arrival triggers battle ──────────────────────────────────────

#[test]
fn exit_hallway_arrival_at_encounter_zone_starts_battle_or_sets_pending() {
    // Use loop 4 (85% encounter, 35% arrival) to maximize encounter frequency.
    for seed in 0u64..1000 {
        let mut session = GameSession::new();
        session.loop_number = 4;

        // Dismiss initial dialog via inner method.
        loop {
            let GamePhase::Exploration(e) = &mut session.phase else {
                break;
            };
            if !e.in_dialog {
                break;
            }
            e.advance_dialog();
        }
        let GamePhase::Exploration(_) = &session.phase else {
            continue;
        };

        let mut rng = StdRng::seed_from_u64(seed);
        session.initiate_travel(ZoneKind::MedicalBay, &mut rng);

        // Try until arrival.
        let mut arrived = false;
        for _ in 0..30 {
            if session.exit_hallway(&mut rng) {
                arrived = true;
                break;
            }
        }
        if !arrived {
            continue;
        }

        match &session.phase {
            GamePhase::Battle(_) => return, // encounter + no dialog → immediate battle
            GamePhase::Exploration(e) if e.zone.has_encounter() && e.pending_battle => {
                return; // encounter + dialog blocking → pending battle set
            }
            GamePhase::Exploration(e) if !e.zone.has_encounter() => {
                continue; // no encounter this seed, try another
            }
            _ => continue,
        }
    }
    panic!("no seed produced an encounter on exit_hallway arrival in 1000 tries");
}
