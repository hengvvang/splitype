//! Settings panel UI state — the in-memory view state of the settings
//! panel (active tab, expanded sections, panel-internal preference toggles).
//!
//! Rendered by `panels.rs`; persisted configuration lives in
//! `crate::infra::config::settings`.

use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SettingsTab {
    #[default]
    Interface, // Interface, Theme & Status Bar
    Editing, // Editing, Typography & Startup
    Keymap,  // Keymap / Keyboard Shortcuts
}

impl SettingsTab {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Interface => "Interface",
            Self::Editing => "Editing",
            Self::Keymap => "Keymap",
        }
    }

    pub fn all() -> &'static [SettingsTab] {
        &[Self::Interface, Self::Editing, Self::Keymap]
    }
}

/// View state of the settings panel.
pub struct SettingsUiState {
    pub tab: SettingsTab,
    pub expanded_sections: HashSet<String>,
    pub expanded_cards: HashSet<String>,
    pub pref_show_status_bar: bool,
    pub pref_show_word_count: bool,
    pub pref_show_cursor_pos: bool,
    pub pref_show_sidebar_toggle: bool,
    pub pref_show_mode_switch: bool,
    pub pref_show_table_headers: bool,
    pub pref_font_size: u32,
    pub pref_line_height: f32,
    pub pref_image_paste_action: usize,
    pub pref_startup_option: usize,
    pub open_dropdown: Option<String>,
    pub editing_stepper: Option<String>,
}

impl Default for SettingsUiState {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsUiState {
    /// Open the settings panel on the Interface tab with the default
    /// sections expanded.
    pub fn new() -> Self {
        let mut sections = HashSet::new();
        sections.insert("theme".to_string());
        sections.insert("status_bar".to_string());
        sections.insert("typography".to_string());
        sections.insert("markdown".to_string());
        sections.insert("startup".to_string());
        sections.insert("doc_actions".to_string());
        sections.insert("view_controls".to_string());

        let mut cards = HashSet::new();
        cards.insert("status_bar".to_string());
        cards.insert("markdown_options".to_string());

        Self {
            tab: SettingsTab::Interface,
            expanded_sections: sections,
            expanded_cards: cards,
            pref_show_status_bar: true,
            pref_show_word_count: true,
            pref_show_cursor_pos: true,
            pref_show_sidebar_toggle: true,
            pref_show_mode_switch: true,
            pref_show_table_headers: true,
            pref_font_size: 14,
            pref_line_height: 1.6,
            pref_image_paste_action: 0,
            pref_startup_option: 0,
            open_dropdown: None,
            editing_stepper: None,
        }
    }

    pub fn toggle_section(&mut self, section_key: &str) {
        if self.expanded_sections.contains(section_key) {
            self.expanded_sections.remove(section_key);
        } else {
            self.expanded_sections.insert(section_key.to_string());
        }
    }

    #[allow(dead_code)]
    pub fn toggle_card(&mut self, card_key: &str) {
        if self.expanded_cards.contains(card_key) {
            self.expanded_cards.remove(card_key);
        } else {
            self.expanded_cards.insert(card_key.to_string());
        }
    }
}
