//! Shortcut key normalization — pure keystroke parsing shared by shortcut
//! configuration consumers.
//!
//! Default shortcuts and key contexts are plugin contributions declared in
//! plugin manifests and recorded by the command registry; user overrides
//! live in the settings store as a map from full command id to keystroke
//! strings. This module only normalizes a raw keystroke list.

use std::collections::BTreeSet;

use gpui::Keystroke;

/// Normalizes a user shortcut override: parses every keystroke, rejects
/// IME-in-progress entries, and drops duplicates. Returns `None` when the
/// list is empty or any entry fails to parse.
pub fn normalize_shortcut_keys(keys: &[String]) -> Option<Vec<String>> {
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for key in keys {
        let parsed = Keystroke::parse(key.trim()).ok()?;
        if parsed.is_ime_in_progress() {
            return None;
        }
        let key = parsed.unparse();
        if seen.insert(key.clone()) {
            normalized.push(key);
        }
    }
    (!normalized.is_empty()).then_some(normalized)
}
