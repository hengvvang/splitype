//! Persistent application settings, domain models, and centralized reactive store.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::Context as _;
use gpui::*;
use serde::{Deserialize, Serialize};

use crate::dirs::SplitypeConfigDirs;
use crate::recent::read_recent_files;

pub type SubsystemSyncHook = fn(&mut App, &AppSettings);
static SYNC_HOOKS: std::sync::RwLock<Vec<SubsystemSyncHook>> = std::sync::RwLock::new(Vec::new());

pub const DEFAULT_THEME_ID: &str = "splitype";
pub const DEFAULT_LANGUAGE_ID: &str = "en-US";

/// Document selection behavior when launching the application.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupOpenSetting {
    #[default]
    NewFile,
    LastOpenedFile,
    Empty,
}

impl StartupOpenSetting {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NewFile => "new_file",
            Self::LastOpenedFile => "last_opened_file",
            Self::Empty => "empty",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::NewFile => "Open New Document",
            Self::LastOpenedFile => "Open Last Active Document",
            Self::Empty => "Open Empty Workspace",
        }
    }

    pub fn all() -> &'static [Self] {
        &[Self::NewFile, Self::LastOpenedFile, Self::Empty]
    }
}

impl std::fmt::Display for StartupOpenSetting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for StartupOpenSetting {
    type Err = std::convert::Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "last_opened_file" => Ok(Self::LastOpenedFile),
            "empty" => Ok(Self::Empty),
            _ => Ok(Self::NewFile),
        }
    }
}

/// Startup and general lifecycle configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartupSettings {
    #[serde(default)]
    pub open: StartupOpenSetting,
    #[serde(default = "default_true")]
    pub restore_window_state: bool,
}

impl Default for StartupSettings {
    fn default() -> Self {
        Self {
            open: StartupOpenSetting::NewFile,
            restore_window_state: true,
        }
    }
}

/// Interface appearance, theme, and language configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterfaceSettings {
    #[serde(default = "default_theme_id_string")]
    pub theme_id: String,
    #[serde(default = "default_language_id_string")]
    pub language_id: String,
}

impl Default for InterfaceSettings {
    fn default() -> Self {
        Self {
            theme_id: DEFAULT_THEME_ID.to_string(),
            language_id: DEFAULT_LANGUAGE_ID.to_string(),
        }
    }
}

fn default_theme_id_string() -> String {
    DEFAULT_THEME_ID.to_string()
}

fn default_language_id_string() -> String {
    DEFAULT_LANGUAGE_ID.to_string()
}

/// Status bar visibility and granular metrics toggles.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusBarSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub show_word_count: bool,
    #[serde(default = "default_true")]
    pub show_cursor_position: bool,
    #[serde(default = "default_true")]
    pub show_character_count: bool,
    #[serde(default = "default_true")]
    pub show_reading_time: bool,
}

impl Default for StatusBarSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            show_word_count: true,
            show_cursor_position: true,
            show_character_count: true,
            show_reading_time: true,
        }
    }
}

/// Core editor behavior settings (line numbers, wrapping, indentation, active line).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorBehaviorSettings {
    #[serde(default = "default_true")]
    pub line_numbers: bool,
    #[serde(default = "default_true")]
    pub word_wrap: bool,
    #[serde(default = "default_tab_size")]
    pub tab_size: u32,
    #[serde(default = "default_true")]
    pub insert_spaces: bool,
    #[serde(default = "default_true")]
    pub highlight_active_line: bool,
}

fn default_tab_size() -> u32 {
    4
}

impl Default for EditorBehaviorSettings {
    fn default() -> Self {
        Self {
            line_numbers: true,
            word_wrap: true,
            tab_size: 4,
            insert_spaces: true,
            highlight_active_line: true,
        }
    }
}

/// Typography preferences (UI, Prose, and Code fonts, sizes, and line heights).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TypographySettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prose_font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_font_family: Option<String>,
    #[serde(default = "default_font_size")]
    pub font_size: u32,
    #[serde(default = "default_line_height")]
    pub line_height: f32,
}

fn default_font_size() -> u32 {
    16
}

fn default_line_height() -> f32 {
    1.6
}

impl Default for TypographySettings {
    fn default() -> Self {
        Self {
            ui_font_family: None,
            prose_font_family: None,
            code_font_family: None,
            font_size: default_font_size(),
            line_height: default_line_height(),
        }
    }
}

/// Where pasted clipboard images should be stored before inserting Markdown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImagePasteBehavior {
    #[default]
    None,
    CopyToDocumentFolder,
    CopyToAssetsFolder,
    CopyToNamedAssetsFolder,
}

impl ImagePasteBehavior {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::CopyToDocumentFolder => "copy_to_document_folder",
            Self::CopyToAssetsFolder => "copy_to_assets_folder",
            Self::CopyToNamedAssetsFolder => "copy_to_named_assets_folder",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::None => "No local copy (standard insertion)",
            Self::CopyToDocumentFolder => "Copy image to document folder (./)",
            Self::CopyToAssetsFolder => "Copy image to assets folder (./assets/)",
            Self::CopyToNamedAssetsFolder => "Copy image to named assets folder (./.assets/)",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::None,
            Self::CopyToDocumentFolder,
            Self::CopyToAssetsFolder,
            Self::CopyToNamedAssetsFolder,
        ]
    }
}

impl std::fmt::Display for ImagePasteBehavior {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ImagePasteBehavior {
    type Err = std::convert::Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "copy_to_document_folder" => Ok(Self::CopyToDocumentFolder),
            "copy_to_assets_folder" => Ok(Self::CopyToAssetsFolder),
            "copy_to_named_assets_folder" => Ok(Self::CopyToNamedAssetsFolder),
            _ => Ok(Self::None),
        }
    }
}

/// Markdown rendering and assets configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownSettings {
    #[serde(default = "default_true")]
    pub show_table_headers: bool,
    #[serde(default)]
    pub image_paste_behavior: ImagePasteBehavior,
    #[serde(default = "default_true")]
    pub render_math: bool,
    #[serde(default = "default_true")]
    pub render_diagrams: bool,
}

impl Default for MarkdownSettings {
    fn default() -> Self {
        Self {
            show_table_headers: true,
            image_paste_behavior: ImagePasteBehavior::None,
            render_math: true,
            render_diagrams: true,
        }
    }
}

/// Explorer tree sorting mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplorerSortMode {
    #[default]
    DirectoriesFirst,
    FilesFirst,
    Mixed,
}

impl ExplorerSortMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DirectoriesFirst => "directories_first",
            Self::FilesFirst => "files_first",
            Self::Mixed => "mixed",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::DirectoriesFirst => "Directories First",
            Self::FilesFirst => "Files First",
            Self::Mixed => "Mixed (Alphabetical)",
        }
    }

    pub fn all() -> &'static [Self] {
        &[Self::DirectoriesFirst, Self::FilesFirst, Self::Mixed]
    }
}

impl std::fmt::Display for ExplorerSortMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ExplorerSortMode {
    type Err = std::convert::Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "files_first" => Ok(Self::FilesFirst),
            "mixed" => Ok(Self::Mixed),
            _ => Ok(Self::DirectoriesFirst),
        }
    }
}

/// Explorer tree sort order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplorerSortOrder {
    #[default]
    Ascending,
    Descending,
}

impl ExplorerSortOrder {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ascending => "ascending",
            Self::Descending => "descending",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Ascending => "Ascending (A to Z)",
            Self::Descending => "Descending (Z to A)",
        }
    }

    pub fn all() -> &'static [Self] {
        &[Self::Ascending, Self::Descending]
    }
}

impl std::fmt::Display for ExplorerSortOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ExplorerSortOrder {
    type Err = std::convert::Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "descending" => Ok(Self::Descending),
            _ => Ok(Self::Ascending),
        }
    }
}

/// Explorer panel settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplorerSettings {
    #[serde(default = "default_true")]
    pub hide_hidden: bool,
    #[serde(default)]
    pub sort_mode: ExplorerSortMode,
    #[serde(default)]
    pub sort_order: ExplorerSortOrder,
    #[serde(default = "default_true")]
    pub auto_reveal: bool,
}

impl Default for ExplorerSettings {
    fn default() -> Self {
        Self {
            hide_hidden: true,
            sort_mode: ExplorerSortMode::DirectoriesFirst,
            sort_order: ExplorerSortOrder::Ascending,
            auto_reveal: true,
        }
    }
}

fn default_true() -> bool {
    true
}

/// Unified, canonical user settings persisted under `config.toml`.
///
/// Zero redundant DTOs or compatibility shims — serializes and deserializes
/// directly to/from disk.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct AppSettings {
    #[serde(default)]
    pub startup: StartupSettings,
    #[serde(default)]
    pub interface: InterfaceSettings,
    #[serde(default)]
    pub status_bar: StatusBarSettings,
    #[serde(default)]
    pub editor: EditorBehaviorSettings,
    #[serde(default)]
    pub typography: TypographySettings,
    #[serde(default)]
    pub markdown: MarkdownSettings,
    #[serde(default)]
    pub explorer: ExplorerSettings,
    /// User shortcut overrides keyed by full command id (e.g.
    /// `splitype.editor.save`); values are gpui keystroke strings.
    #[serde(default)]
    pub keybindings: BTreeMap<String, Vec<String>>,
}

/// Central reactive in-memory GPUI Global store for [`AppSettings`].
///
/// All mutations must go through [`SettingsStore::update`] to guarantee:
/// 1. Instant in-memory synchronization.
/// 2. Subsystem updates (Theme, Typography, I18n, Keybindings).
/// 3. Atomic disk persistence to `config.toml`.
/// 4. Global window repaint notification via `cx.refresh_windows()`.
pub struct SettingsStore {
    pub settings: AppSettings,
}

impl Global for SettingsStore {}

impl SettingsStore {
    /// Initialize the global settings store in GPUI context.
    pub fn init(cx: &mut App, settings: AppSettings) {
        cx.set_global(Self { settings });
    }

    /// Initialize default settings store in test environments.
    pub fn init_default(cx: &mut App) {
        Self::init(cx, AppSettings::default());
    }

    /// Read the active global settings by reference.
    pub fn get(cx: &App) -> &AppSettings {
        cx.try_global::<Self>()
            .map(|store| &store.settings)
            .unwrap_or_else(|| {
                // Static fallback in uninitialized contexts / tests
                static DEFAULT: std::sync::OnceLock<AppSettings> = std::sync::OnceLock::new();
                DEFAULT.get_or_init(AppSettings::default)
            })
    }

    /// Read a cloned copy of the active global settings.
    pub fn settings(cx: &App) -> AppSettings {
        Self::get(cx).clone()
    }

    /// Mutate settings in-place, persist to disk, sync subsystems, and refresh windows.
    pub fn update<R>(
        cx: &mut App,
        mutate: impl FnOnce(&mut AppSettings) -> R,
    ) -> anyhow::Result<R> {
        let (result, new_settings) = {
            let store = cx
                .try_global::<Self>()
                .context("SettingsStore global not initialized")?;
            let mut updated = store.settings.clone();
            let res = mutate(&mut updated);
            (res, updated)
        };

        // Update global store
        cx.set_global(Self {
            settings: new_settings.clone(),
        });

        // Persist to disk
        if let Err(err) = save_app_settings(&new_settings) {
            tracing::warn!(error = %err, "failed to persist settings to disk");
        }

        // Synchronize subsystems
        Self::sync_subsystems(cx, &new_settings);

        // Refresh all application windows
        cx.refresh_windows();

        Ok(result)
    }

    /// Replace the entire settings configuration.
    pub fn set(cx: &mut App, new_settings: AppSettings) -> anyhow::Result<()> {
        Self::update(cx, |settings| {
            *settings = new_settings;
        })
    }

    /// Register a hook to be called on settings mutation to synchronize subsystems (e.g. Theme, Typography, I18n).
    pub fn register_sync_hook(hook: SubsystemSyncHook) {
        if let Ok(mut hooks) = SYNC_HOOKS.write() {
            hooks.push(hook);
        }
    }

    fn sync_subsystems(cx: &mut App, settings: &AppSettings) {
        if let Ok(hooks) = SYNC_HOOKS.read() {
            for hook in hooks.iter() {
                hook(cx, settings);
            }
        }
    }
}

/// Read configuration from disk using system configuration directories.
pub fn read_app_settings() -> anyhow::Result<AppSettings> {
    read_app_settings_with_dirs(&SplitypeConfigDirs::from_system()?)
}

/// Read configuration from disk using the specified configuration directories.
pub fn read_app_settings_with_dirs(dirs: &SplitypeConfigDirs) -> anyhow::Result<AppSettings> {
    let path = dirs.app_config_file();
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(AppSettings::default());
        }
        Err(err) => {
            return Err(err).with_context(|| format!("failed to read '{}'", path.display()));
        }
    };
    let settings: AppSettings = toml::from_str(&text).unwrap_or_default();
    Ok(settings)
}

/// Load configuration or create initial settings file with locale detection.
pub fn load_or_create_app_settings() -> anyhow::Result<AppSettings> {
    let dirs = SplitypeConfigDirs::from_system()?;
    load_or_create_app_settings_with_dirs_and_locales(&dirs, sys_locale::get_locales())
}

fn detected_language_id_from_locales<I, S>(locales: I) -> &'static str
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    for locale in locales {
        let tag = locale.as_ref().to_ascii_lowercase();
        if tag.starts_with("zh") {
            return "zh-CN";
        }
    }
    "en-US"
}

pub fn load_or_create_app_settings_with_dirs_and_locales<I, S>(
    dirs: &SplitypeConfigDirs,
    locales: I,
) -> anyhow::Result<AppSettings>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let detected_language_id = detected_language_id_from_locales(locales);
    let path = dirs.app_config_file();
    let settings = match std::fs::read_to_string(&path) {
        Ok(text) => match toml::from_str::<AppSettings>(&text) {
            Ok(settings) => settings,
            Err(_) => {
                let mut def = AppSettings::default();
                def.interface.language_id = detected_language_id.into();
                def
            }
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let mut def = AppSettings::default();
            def.interface.language_id = detected_language_id.into();
            def
        }
        Err(err) => {
            return Err(err).with_context(|| format!("failed to read '{}'", path.display()));
        }
    };
    save_app_settings_with_dirs(&settings, dirs)?;
    Ok(settings)
}

/// Save configuration to disk using system configuration directories.
pub fn save_app_settings(settings: &AppSettings) -> anyhow::Result<()> {
    save_app_settings_with_dirs(settings, &SplitypeConfigDirs::from_system()?)
}

/// Save configuration to disk using the specified configuration directories.
pub fn save_app_settings_with_dirs(
    settings: &AppSettings,
    dirs: &SplitypeConfigDirs,
) -> anyhow::Result<()> {
    let path = dirs.app_config_file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create '{}'", parent.display()))?;
    }
    let text = toml::to_string_pretty(settings)?;
    std::fs::write(&path, text).with_context(|| format!("failed to write '{}'", path.display()))
}

/// First existing recent markdown file for startup opening.
pub fn first_existing_recent_markdown_file() -> Option<PathBuf> {
    let recent_files = read_recent_files().ok()?;
    recent_files.into_iter().find(|path| path.is_file())
}
