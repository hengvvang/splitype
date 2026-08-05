//! Window and menu command actions — the GPUI action protocol for
//! editor-window and app-level commands.
//!
//! Text-editing actions and the keybinding configuration table live in
//! `editing::input::shortcuts`; this module holds the command actions that
//! menus dispatch and window chrome binds.

use gpui::*;
use schemars::JsonSchema;
use serde::Deserialize;

actions!(
    splitype,
    [
        SaveDocument,
        NewWindow,
        OpenFile,
        OpenSettings,
        NoRecentFiles,
        SaveDocumentAs,
        ExportHtml,
        ExportPdf,
        AddLanguageConfig,
        AddThemeConfig,
        QuitApplication,
        CloseWindow,
        CheckForUpdates,
        ShowAbout,
        InstallCliTool,
        UninstallCliTool,
        ToggleViewMode,
        ToggleExplorer,
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
