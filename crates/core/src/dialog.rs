use std::collections::HashSet;

/// Lightweight flag store that tracks which named events have occurred.
///
/// Replaces the old `DialogEngine` now that dialog is driven by
/// `bevy_yarnspinner` in the GUI crate.  Flag values are also kept in the
/// `DialogueRunner`'s variable storage for `.yarn` conditional logic; this
/// struct is the authoritative copy used by core game systems (zone NPC
/// spawning, ending resolution, save/load).
#[derive(Debug, Default)]
pub struct DialogFlags {
    flags: HashSet<String>,
}

impl DialogFlags {
    pub fn new() -> Self {
        Self::default()
    }

    /// Raise a named flag.
    pub fn set_flag(&mut self, flag: impl Into<String>) {
        self.flags.insert(flag.into());
    }

    /// Returns `true` when the named flag has been raised.
    pub fn is_flag_set(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }

    /// Read-only view of all current flags.
    pub fn flags(&self) -> &HashSet<String> {
        &self.flags
    }

    /// Return all currently-set flags as a sorted `Vec<String>` for serialization.
    pub fn export_flags(&self) -> Vec<String> {
        let mut v: Vec<String> = self.flags.iter().cloned().collect();
        v.sort();
        v
    }

    /// Merge a previously-exported flag list back into the store.
    pub fn import_flags(&mut self, flags: Vec<String>) {
        self.flags.extend(flags);
    }
}
