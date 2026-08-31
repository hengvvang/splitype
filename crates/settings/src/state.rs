//! Settings panel UI state — transient view state of the settings UI.
//!
//! A gpui `Global` (like `ThemeManager` / `SettingsStore`): the in-window
//! settings panel reads and mutates it through [`SettingsUiState::global`]
//! / [`SettingsUiState::update`], so the panel code never touches the
//! window shell. Pure view state only (active tab, expanded sections,
//! dropdown states, search queries, inline editing buffers). Canonical
//! configuration lives in `SettingsStore` (`config::settings`).

use std::collections::HashSet;

use gpui::{BorrowAppContext, Global};

/// Expanded, categorized settings navigation tabs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SettingsTab {
    #[default]
    Interface, // Interface, Theme & Status Bar metrics
    Editor,    // Typography & Editor behavior (line numbers, wrapping, tab size)
    Markdown,  // Markdown rendering, Table headers, LaTeX math, Mermaid & Asset paste behavior
    Explorer,  // File Explorer tree, Hidden files, Sort mode & Sort order
    Startup,   // Startup document selection & Window state restore
    Keymap,    // Keymap / Keyboard Shortcuts list
}

impl SettingsTab {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Interface => "Interface",
            Self::Editor => "Editor",
            Self::Markdown => "Markdown",
            Self::Explorer => "Explorer",
            Self::Startup => "Startup",
            Self::Keymap => "Keymap",
        }
    }

    pub fn all() -> &'static [SettingsTab] {
        &[
            Self::Interface,
            Self::Editor,
            Self::Markdown,
            Self::Explorer,
            Self::Startup,
            Self::Keymap,
        ]
    }
}

/// Pure transient view state of the settings UI.
pub struct SettingsUiState {
    pub tab: SettingsTab,
    pub expanded_sections: HashSet<String>,
    pub open_dropdown: Option<String>,
    pub search_query_ui_font: String,
    pub search_query_prose_font: String,
    pub search_query_code_font: String,
    pub editing_font_size: Option<String>,
    pub editing_line_height: Option<String>,
    pub editing_tab_size: Option<String>,
    pub font_size_focus_handle: Option<gpui::FocusHandle>,
    pub line_height_focus_handle: Option<gpui::FocusHandle>,
    pub tab_size_focus_handle: Option<gpui::FocusHandle>,
}

impl Global for SettingsUiState {}

impl Default for SettingsUiState {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsUiState {
    /// Initialize default settings UI view state.
    pub fn new() -> Self {
        let mut sections = HashSet::new();
        sections.insert("theme".to_string());
        sections.insert("status_bar".to_string());
        sections.insert("typography".to_string());
        sections.insert("editor_behavior".to_string());
        sections.insert("markdown".to_string());
        sections.insert("explorer".to_string());
        sections.insert("startup".to_string());
        sections.insert("doc_actions".to_string());
        sections.insert("view_controls".to_string());
        sections.insert("editor_shortcuts".to_string());

        Self {
            tab: SettingsTab::Interface,
            expanded_sections: sections,
            open_dropdown: None,
            search_query_ui_font: String::new(),
            search_query_prose_font: String::new(),
            search_query_code_font: String::new(),
            editing_font_size: None,
            editing_line_height: None,
            editing_tab_size: None,
            font_size_focus_handle: None,
            line_height_focus_handle: None,
            tab_size_focus_handle: None,
        }
    }

    /// The app-wide settings UI state; panics when not installed by the
    /// app bootstrap.
    pub fn global(cx: &gpui::App) -> &Self {
        cx.global::<Self>()
    }

    /// Mutate the app-wide settings UI state and notify all windows.
    ///
    /// The closure receives the state and the app context (for nested
    /// global access such as `SettingsStore::update`).
    pub fn update<R>(
        cx: &mut gpui::App,
        f: impl FnOnce(&mut Self, &mut gpui::App) -> R,
    ) -> R {
        let result = cx.update_global::<Self, R>(f);
        cx.refresh_windows();
        result
    }

    /// Return the cached focus handle for `slot`, lazily creating and
    /// storing one when the settings UI has not used it yet.
    pub fn cached_focus_handle(
        cx: &mut gpui::App,
        slot: fn(&mut Self) -> &mut Option<gpui::FocusHandle>,
    ) -> gpui::FocusHandle {
        let existing =
            cx.update_global::<Self, Option<gpui::FocusHandle>>(|s, _cx| slot(s).clone());
        if let Some(handle) = existing {
            return handle;
        }
        let handle = cx.focus_handle();
        cx.update_global::<Self, _>(|s, _cx| *slot(s) = Some(handle.clone()));
        handle
    }
}

