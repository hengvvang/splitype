//! Window and application command actions — the GPUI action protocol for
//! app-level commands (windows, menus, settings, explorer, CLI, links).
//!
//! Document-level actions (save / export / view mode) live in
//! `crate::editor::actions`; text-editing actions live in
//! `crate::editor::editing::input::actions`.

use gpui::*;
use schemars::JsonSchema;
use serde::Deserialize;

actions!(
    splitype,
    [
        NewWindow,
        OpenFile,
        OpenSettings,
        NoRecentFiles,
        AddLanguageConfig,
        AddThemeConfig,
        QuitApplication,
        CloseWindow,
        CheckForUpdates,
        ShowAbout,
        InstallCliTool,
        UninstallCliTool,
        ToggleExplorer,
        ToggleMaximizeArea,
        CloseExplorerFolder,
        OpenSplitypeRepository,
        OpenBugReport,
        OpenFeatureRequest,
        OpenDiscussions,
    ]
);

/// Selects a theme from the app-level theme registry.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema, gpui::Action)]
#[action(namespace = splitype)]
#[serde(deny_unknown_fields)]
pub struct SelectTheme {
    /// Stable theme id from the built-in theme catalog.
    pub theme_id: String,
}

/// Selects a UI language from the app-level language registry.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema, gpui::Action)]
#[action(namespace = splitype)]
#[serde(deny_unknown_fields)]
pub struct SelectLanguage {
    /// Stable language id from the built-in language catalog.
    pub language_id: String,
}

/// Opens a previously recorded Markdown file path.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema, gpui::Action)]
#[action(namespace = splitype)]
#[serde(deny_unknown_fields)]
pub struct OpenRecentFile {
    /// Path stored in splitype's recent-file history.
    pub path: String,
}
