use carbonthrone::dialog::DialogFlags;

// ── Flag management ───────────────────────────────────────────────────────

#[test]
fn set_flag_and_is_flag_set() {
    let mut flags = DialogFlags::new();
    assert!(!flags.is_flag_set("companion_orin"));
    flags.set_flag("companion_orin");
    assert!(flags.is_flag_set("companion_orin"));
}

#[test]
fn export_and_import_flags_round_trip() {
    let mut flags = DialogFlags::new();
    flags.set_flag("loop1_opened");
    flags.set_flag("companion_orin");

    let exported = flags.export_flags();
    assert!(exported.contains(&"loop1_opened".to_string()));
    assert!(exported.contains(&"companion_orin".to_string()));

    let mut flags2 = DialogFlags::new();
    flags2.import_flags(exported);
    assert!(flags2.is_flag_set("loop1_opened"));
    assert!(flags2.is_flag_set("companion_orin"));
}

#[test]
fn export_flags_are_sorted() {
    let mut flags = DialogFlags::new();
    flags.set_flag("zzz");
    flags.set_flag("aaa");
    flags.set_flag("mmm");

    let exported = flags.export_flags();
    assert_eq!(exported, vec!["aaa", "mmm", "zzz"]);
}

#[test]
fn unset_flag_returns_false() {
    let flags = DialogFlags::new();
    assert!(!flags.is_flag_set("nonexistent_flag"));
}
