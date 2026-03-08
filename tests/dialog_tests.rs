use carbonthrone::dialog::{DialogEngine, Trigger};

const LOOP1_YAML: &str = include_str!("../data/loops/loop1.yaml");
const LOOP2_YAML: &str = include_str!("../data/loops/loop2.yaml");

// ── Loading ───────────────────────────────────────────────────────────────

#[test]
fn load_loop1_parses_without_error() {
    let mut engine = DialogEngine::new();
    engine.load_script(LOOP1_YAML).expect("loop1 should parse");
}

#[test]
fn load_multiple_loops() {
    let mut engine = DialogEngine::new();
    engine.load_script(LOOP1_YAML).expect("loop1 should parse");
    engine.load_script(LOOP2_YAML).expect("loop2 should parse");
}

// ── Triggering scenes ─────────────────────────────────────────────────────

#[test]
fn opening_scene_fires_on_enter() {
    let mut engine = DialogEngine::new();
    engine.load_script(LOOP1_YAML).unwrap();
    engine.set_companion("orin");

    let scene = engine.trigger(&Trigger::OnEnter, "research_wing");
    assert!(scene.is_some());
    let scene = scene.unwrap();
    assert_eq!(scene.id, "loop1_opening");
    assert!(!scene.lines.is_empty());
}

#[test]
fn no_scene_for_unknown_location() {
    let mut engine = DialogEngine::new();
    engine.load_script(LOOP1_YAML).unwrap();

    let scene = engine.trigger(&Trigger::OnEnter, "nowhere");
    assert!(scene.is_none());
}

// ── Flag management ───────────────────────────────────────────────────────

#[test]
fn set_flag_and_is_flag_set() {
    let mut engine = DialogEngine::new();
    assert!(!engine.is_flag_set("companion_orin"));
    engine.set_flag("companion_orin");
    assert!(engine.is_flag_set("companion_orin"));
}

#[test]
fn scene_sets_flag_automatically() {
    let mut engine = DialogEngine::new();
    engine.load_script(LOOP2_YAML).unwrap();
    engine.set_companion("orin");

    // loop2_orin_locket requires companion_orin and flags_unset: [orin_locket_noticed]
    // and it sets_flag: orin_locket_noticed
    engine.set_flag("companion_orin");
    assert!(!engine.is_flag_set("orin_locket_noticed"));

    let scene = engine.trigger(&Trigger::OnInteract, "command_deck");
    assert!(scene.is_some());
    assert_eq!(scene.unwrap().id, "loop2_orin_locket");
    assert!(engine.is_flag_set("orin_locket_noticed"));
}

// ── Requirements: flags_set / flags_unset ─────────────────────────────────

#[test]
fn scene_blocked_when_required_flag_absent() {
    let mut engine = DialogEngine::new();
    engine.load_script(LOOP1_YAML).unwrap();
    engine.set_companion("orin");

    // loop1_post_split_orin requires flags_set: [companion_orin]
    // Without that flag it should not fire.
    let scene = engine.trigger(&Trigger::OnEnter, "command_deck");
    assert!(scene.is_none());
}

#[test]
fn scene_available_when_required_flag_set() {
    let mut engine = DialogEngine::new();
    engine.load_script(LOOP1_YAML).unwrap();
    engine.set_companion("orin");
    engine.set_flag("companion_orin");

    let scene = engine.trigger(&Trigger::OnEnter, "command_deck");
    assert!(scene.is_some());
    assert_eq!(scene.unwrap().id, "loop1_post_split_orin");
}

#[test]
fn scene_blocked_when_unset_flag_is_present() {
    let mut engine = DialogEngine::new();
    engine.load_script(LOOP2_YAML).unwrap();
    engine.set_companion("orin");
    engine.set_flag("companion_orin");
    // orin_locket_noticed being set should block loop2_orin_locket
    engine.set_flag("orin_locket_noticed");

    let scene = engine.trigger(&Trigger::OnInteract, "command_deck");
    assert!(scene.is_none());
}

// ── Companion requirements ────────────────────────────────────────────────

#[test]
fn scene_blocked_by_wrong_companion() {
    let mut engine = DialogEngine::new();
    engine.load_script(LOOP1_YAML).unwrap();
    engine.set_companion("doss"); // wrong companion
    engine.set_flag("companion_orin");

    let scene = engine.trigger(&Trigger::OnEnter, "command_deck");
    assert!(scene.is_none());
}

// ── Choices ───────────────────────────────────────────────────────────────

#[test]
fn selecting_choice_sets_flag_and_navigates() {
    let mut engine = DialogEngine::new();
    engine.load_script(LOOP1_YAML).unwrap();
    engine.set_companion("orin");

    // loop1_split requires loop1_opened (set by loop1_opening on first entry)
    engine.set_flag("loop1_opened");
    // Trigger the split scene which has [Follow Orin.] / [Follow Doss.]
    let scene = engine.trigger(&Trigger::OnEnter, "research_wing");
    assert!(scene.is_some());
    assert_eq!(scene.unwrap().id, "loop1_split");

    // Select choice 0: [Follow Orin.] → sets companion_orin, leads_to loop1_post_split_orin
    let next = engine.select_choice(0);
    assert!(next.is_some());
    assert_eq!(next.unwrap().id, "loop1_post_split_orin");
    assert!(engine.is_flag_set("companion_orin"));
}

#[test]
fn selecting_choice_with_null_leads_to_returns_none() {
    let mut engine = DialogEngine::new();
    engine.load_script(LOOP1_YAML).unwrap();
    engine.set_companion("doss");
    engine.set_flag("companion_doss");

    // loop1_doss_armory requires doss_arrived_annex (set by loop1_post_split_doss on first entry)
    engine.set_flag("doss_arrived_annex");
    // loop1_doss_armory choice 1: "[Take equipment and move on.]" leads_to: ~
    let scene = engine.trigger(&Trigger::OnEnter, "military_annex");
    assert!(scene.is_some());
    assert_eq!(scene.unwrap().id, "loop1_doss_armory");

    let next = engine.select_choice(1);
    assert!(next.is_none());
}

#[test]
fn out_of_bounds_choice_index_returns_none() {
    let mut engine = DialogEngine::new();
    engine.load_script(LOOP1_YAML).unwrap();
    engine.set_companion("orin");

    engine.trigger(&Trigger::OnEnter, "research_wing_corridor");
    let next = engine.select_choice(99);
    assert!(next.is_none());
}

// ── go_to_scene ───────────────────────────────────────────────────────────

#[test]
fn go_to_scene_navigates_when_requirements_met() {
    let mut engine = DialogEngine::new();
    engine.load_script(LOOP1_YAML).unwrap();
    engine.set_companion("orin");
    engine.set_flag("companion_orin");

    let scene = engine.go_to_scene("loop1_orin_briefing");
    assert!(scene.is_some());
    assert_eq!(scene.unwrap().id, "loop1_orin_briefing");
}

#[test]
fn go_to_scene_blocked_when_requirements_unmet() {
    let mut engine = DialogEngine::new();
    engine.load_script(LOOP1_YAML).unwrap();
    engine.set_companion("doss");

    let scene = engine.go_to_scene("loop1_orin_briefing");
    assert!(scene.is_none());
}

#[test]
fn go_to_unknown_scene_returns_none() {
    let mut engine = DialogEngine::new();
    engine.load_script(LOOP1_YAML).unwrap();

    assert!(engine.go_to_scene("does_not_exist").is_none());
}
