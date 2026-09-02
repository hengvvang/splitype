//! Settings view state — the active plugin page plus the transient
//! interaction state of the settings UI.
//!
//! Each in-window settings panel owns its own [`SettingsUiState`] entity, so
//! splitting or cloning the settings panel yields independent instances.
//! Canonical configuration lives in `SettingsStore` (`config::settings`);
//! this entity only remembers which plugin page the user is viewing and the
//! transient dropdown/editing state of the settings UI itself.

use std::collections::BTreeMap;

use gpui::{App, FocusHandle};
use serde::{Deserialize, Serialize};

/// Pure transient view state of the settings UI.
pub struct SettingsUiState {
    /// Id of the currently active plugin page.
    pub active_plugin: String,
    /// Declaration key with an open dropdown / picker, if any.
    pub open_picker: Option<String>,
    /// Declaration key → inline edit buffer.
    pub edit_buffers: BTreeMap<String, String>,
    /// Declaration key → search query of its searchable picker.
    pub search_queries: BTreeMap<String, String>,
    /// Declaration key → focus handle for its inline editor.
    focus_handles: BTreeMap<String, FocusHandle>,
}

impl Default for SettingsUiState {
    fn default() -> Self {
        Self::new()
    }
}

/// Durable settings panel facts persisted across launches: the active plugin
/// page. Canonical configuration lives in `SettingsStore`; this only
/// remembers which page the user was viewing.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersistedSettingsState {
    pub active_plugin: String,
}

impl SettingsUiState {
    /// Initialize default settings UI view state.
    pub fn new() -> Self {
        Self {
            active_plugin: String::new(),
            open_picker: None,
            edit_buffers: BTreeMap::new(),
            search_queries: BTreeMap::new(),
            focus_handles: BTreeMap::new(),
        }
    }

    /// Builds the state from a persisted snapshot.
    pub fn from_persisted(persisted: &PersistedSettingsState) -> Self {
        let mut state = Self::new();
        if !persisted.active_plugin.is_empty() {
            state.active_plugin = persisted.active_plugin.clone();
        }
        state
    }

    /// Returns the cached focus handle for `key`, creating and caching one on
    /// first use.
    pub fn focus_handle(&mut self, key: &str, cx: &mut App) -> FocusHandle {
        if let Some(handle) = self.focus_handles.get(key) {
            return handle.clone();
        }
        let handle = cx.focus_handle();
        self.focus_handles.insert(key.to_string(), handle.clone());
        handle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_settings_state_round_trips() {
        let state = PersistedSettingsState {
            active_plugin: "splitype.explorer".to_string(),
        };
        let json = serde_json::to_value(&state).expect("serialize");
        let restored: PersistedSettingsState = serde_json::from_value(json).expect("deserialize");
        assert_eq!(restored.active_plugin, "splitype.explorer");
    }
}
