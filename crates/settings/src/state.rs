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
    /// Global settings search query.
    pub search_query: String,
    /// Declaration key with an open dropdown / picker, if any.
    pub open_picker: Option<String>,
    /// Declaration key → inline edit buffer.
    pub edit_buffers: BTreeMap<String, String>,
    /// Declaration key → search query of its searchable picker.
    pub search_queries: BTreeMap<String, String>,
    /// Declaration key → focus handle for its inline editor.
    focus_handles: BTreeMap<String, FocusHandle>,
    /// Focus handle for the settings bottombar search input.
    search_focus: Option<FocusHandle>,
    /// Whether the compact category drawer/menu is currently open.
    pub is_menu_open: bool,
    /// Explicit collapsed state overrides for settings groups (group_id -> is_collapsed).
    pub collapsed_groups: BTreeMap<String, bool>,
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
            search_query: String::new(),
            open_picker: None,
            edit_buffers: BTreeMap::new(),
            search_queries: BTreeMap::new(),
            focus_handles: BTreeMap::new(),
            search_focus: None,
            is_menu_open: false,
            collapsed_groups: BTreeMap::new(),
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

    /// Returns the focus handle for the settings bottombar search input.
    pub fn search_focus_handle(&mut self, cx: &mut App) -> FocusHandle {
        if let Some(handle) = &self.search_focus {
            return handle.clone();
        }
        let handle = cx.focus_handle();
        self.search_focus = Some(handle.clone());
        handle
    }

    /// Clears the global settings search query.
    pub fn clear_search(&mut self) {
        self.search_query.clear();
    }

    /// Toggles the compact category drawer/menu open state.
    pub fn toggle_menu(&mut self) {
        self.is_menu_open = !self.is_menu_open;
    }

    /// Closes the compact category drawer/menu.
    pub fn close_menu(&mut self) {
        self.is_menu_open = false;
    }

    /// Opens the compact category drawer/menu.
    pub fn open_menu(&mut self) {
        self.is_menu_open = true;
    }

    /// Checks whether a settings group is currently collapsed.
    /// When there is an active search query, groups are always expanded so search matches are visible.
    pub fn is_group_collapsed(&self, group_id: &str, default_collapsed: bool) -> bool {
        if !self.search_query.trim().is_empty() {
            return false;
        }
        if let Some(collapsed) = self.collapsed_groups.get(group_id) {
            *collapsed
        } else {
            default_collapsed
        }
    }

    /// Toggles the collapsed state of a settings group.
    pub fn toggle_group_collapsed(&mut self, group_id: &str, default_collapsed: bool) {
        let currently_collapsed = self.is_group_collapsed(group_id, default_collapsed);
        self.collapsed_groups
            .insert(group_id.to_string(), !currently_collapsed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_collapse_and_search_auto_expand() {
        let mut state = SettingsUiState::new();
        // Defaults to false
        assert!(!state.is_group_collapsed("group_a", false));
        // Defaults to true
        assert!(state.is_group_collapsed("group_b", true));

        // Toggle group_a -> becomes collapsed
        state.toggle_group_collapsed("group_a", false);
        assert!(state.is_group_collapsed("group_a", false));

        // When search query is entered, all groups are auto-expanded!
        state.search_query = "word count".to_string();
        assert!(!state.is_group_collapsed("group_a", false));
        assert!(!state.is_group_collapsed("group_b", true));

        // When search query is cleared, prior collapsed states are restored!
        state.clear_search();
        assert!(state.is_group_collapsed("group_a", false));
        assert!(state.is_group_collapsed("group_b", true));
    }

    #[test]
    fn test_menu_state() {
        let mut state = SettingsUiState::new();
        assert!(!state.is_menu_open);
        state.toggle_menu();
        assert!(state.is_menu_open);
        state.close_menu();
        assert!(!state.is_menu_open);
        state.open_menu();
        assert!(state.is_menu_open);
    }

    #[test]
    fn persisted_settings_state_round_trips() {
        let state = PersistedSettingsState {
            active_plugin: "splitype.explorer".to_string(),
        };
        let json = serde_json::to_value(&state).expect("serialize");
        let restored: PersistedSettingsState = serde_json::from_value(json).expect("deserialize");
        assert_eq!(restored.active_plugin, "splitype.explorer");
    }

    #[test]
    fn test_search_queries_state() {
        let mut state = SettingsUiState::new();
        assert!(state.search_queries.is_empty());

        state.search_queries.insert("font_family".to_string(), "cascadia".to_string());
        assert_eq!(state.search_queries.get("font_family").map(|s| s.as_str()), Some("cascadia"));

        state.search_queries.insert("theme-overrides".to_string(), "border".to_string());
        assert_eq!(state.search_queries.get("theme-overrides").map(|s| s.as_str()), Some("border"));
    }
}

