//! Persistent app settings and the settings window.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::Context as _;
use gpui::*;
use serde::{Deserialize, Serialize};

use crate::infra::config::dirs::SplitypeConfigDirs;
use crate::infra::config::keybindings::normalize_shortcut_config;
use crate::infra::config::recent::read_recent_files;
use crate::infra::i18n::manager::I18nManager;
use crate::infra::i18n::packs::language_id_for_locale_settings;
use crate::infra::theme::ThemeManager;

pub const DEFAULT_THEME_ID: &str = "splitype";
const DEFAULT_LANGUAGE_ID: &str = "en-US";

/// Status bar visibility and component toggles.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusBarSettings {
    pub enabled: bool,
    pub show_word_count: bool,
    pub show_cursor_position: bool,
}

impl Default for StatusBarSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            show_word_count: true,
            show_cursor_position: true,
        }
    }
}

/// Startup document selection stored in `config.toml`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupOpenSetting {
    #[default]
    NewFile,
    LastOpenedFile,
}

impl StartupOpenSetting {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NewFile => "new_file",
            Self::LastOpenedFile => "last_opened_file",
        }
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
            _ => Ok(Self::NewFile),
        }
    }
}

/// Explorer tree sorting mode (mirrors Zed's `ProjectPanelSortMode`).
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

/// Explorer sidebar settings persisted in `config.toml`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplorerSettings {
    pub hide_hidden: bool,
    pub sort_mode: ExplorerSortMode,
    pub sort_order: ExplorerSortOrder,
}

impl Default for ExplorerSettings {
    fn default() -> Self {
        Self {
            hide_hidden: false,
            sort_mode: ExplorerSortMode::DirectoriesFirst,
            sort_order: ExplorerSortOrder::Ascending,
        }
    }
}

/// Runtime mirror of [`ExplorerSettings`] so the scan path reads them
/// without touching disk; toggles persist back to the settings file.
pub struct ExplorerSettingsStore {
    pub settings: ExplorerSettings,
}

impl Global for ExplorerSettingsStore {}

impl ExplorerSettingsStore {
    pub fn init(cx: &mut App) {
        let settings = read_app_settings()
            .ok()
            .map(|settings| settings.explorer)
            .unwrap_or_default();
        cx.set_global(Self { settings });
    }

    pub fn settings(cx: &App) -> ExplorerSettings {
        cx.try_global::<Self>()
            .map(|store| store.settings)
            .unwrap_or_default()
    }

    pub fn set(cx: &mut App, settings: ExplorerSettings) {
        cx.set_global(Self { settings });
        match read_app_settings() {
            Ok(mut app_settings) => {
                app_settings.explorer = settings;
                if let Err(err) = save_app_settings(&app_settings) {
                    tracing::warn!(error = %err, "failed to save explorer settings");
                }
            }
            Err(err) => tracing::warn!(error = %err, "failed to read explorer settings"),
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

/// User settings persisted under the app config directory.
#[derive(Clone, Debug, PartialEq)]
pub struct AppSettings {
    pub startup_open: StartupOpenSetting,
    pub default_language_id: String,
    pub default_theme_id: String,
    pub show_table_headers: bool,
    pub image_paste_behavior: ImagePasteBehavior,
    pub keybindings: BTreeMap<String, Vec<String>>,
    pub status_bar: StatusBarSettings,
    pub explorer: ExplorerSettings,
    pub typography: TypographySettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            startup_open: StartupOpenSetting::NewFile,
            default_language_id: DEFAULT_LANGUAGE_ID.into(),
            default_theme_id: DEFAULT_THEME_ID.into(),
            show_table_headers: true,
            image_paste_behavior: ImagePasteBehavior::None,
            keybindings: BTreeMap::new(),
            status_bar: StatusBarSettings::default(),
            explorer: ExplorerSettings::default(),
            typography: TypographySettings::default(),
        }
    }
}

/// Runtime-accessible editor settings mirrored from [`AppSettings`] so the
/// render path can read them without touching disk. Toggling persists the new
/// value back to the settings file.
pub struct EditorSettings {
    show_table_headers: bool,
    pub status_bar_settings: StatusBarSettings,
}

impl Global for EditorSettings {}

impl EditorSettings {
    pub fn init(cx: &mut App, show_table_headers: bool) {
        let status_bar = read_app_settings()
            .ok()
            .map(|p| p.status_bar)
            .unwrap_or_default();
        Self::set_global(cx, show_table_headers, &status_bar);
    }

    fn set_global(cx: &mut App, show_table_headers: bool, status_bar: &StatusBarSettings) {
        cx.set_global(Self {
            show_table_headers,
            status_bar_settings: StatusBarSettings {
                enabled: status_bar.enabled,
                show_word_count: status_bar.show_word_count,
                show_cursor_position: status_bar.show_cursor_position,
            },
        });
    }

    /// Whether table top rows are styled as headers. Defaults to `true` when
    /// the global has not been installed (e.g. in unit tests).
    pub fn show_table_headers(cx: &App) -> bool {
        cx.try_global::<Self>()
            .map(|settings| settings.show_table_headers)
            .unwrap_or(true)
    }

    pub fn set_show_table_headers(cx: &mut App, show_table_headers: bool) {
        let status_bar = cx
            .try_global::<Self>()
            .map(|s| StatusBarSettings {
                enabled: s.status_bar_settings.enabled,
                show_word_count: s.status_bar_settings.show_word_count,
                show_cursor_position: s.status_bar_settings.show_cursor_position,
            })
            .unwrap_or_default();
        Self::set_global(cx, show_table_headers, &status_bar);
        match read_app_settings() {
            Ok(mut settings) => {
                settings.show_table_headers = show_table_headers;
                if let Err(err) = save_app_settings(&settings) {
                    tracing::warn!(error = %err, "failed to save table header setting");
                }
            }
            Err(err) => tracing::warn!(error = %err, "failed to read table header setting"),
        }
    }

    pub fn status_bar_settings(cx: &App) -> StatusBarSettings {
        cx.try_global::<Self>()
            .map(|s| StatusBarSettings {
                enabled: s.status_bar_settings.enabled,
                show_word_count: s.status_bar_settings.show_word_count,
                show_cursor_position: s.status_bar_settings.show_cursor_position,
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SettingsFile {
    #[serde(default)]
    startup: StartupSettingsFile,
    #[serde(default)]
    language: LanguageSettingsFile,
    #[serde(default)]
    theme: ThemeSettingsFile,
    #[serde(default)]
    editor: EditorSettingsFile,
    #[serde(default)]
    status_bar: StatusBarSettingsFile,
    #[serde(default)]
    keybindings: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    explorer: ExplorerSettingsFile,
    #[serde(default)]
    typography: TypographySettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct StartupSettingsFile {
    #[serde(default)]
    open: StartupOpenSetting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EditorSettingsFile {
    #[serde(default = "default_true")]
    show_table_headers: bool,
    #[serde(default)]
    image_paste_behavior: ImagePasteBehavior,
}

impl Default for EditorSettingsFile {
    fn default() -> Self {
        Self {
            show_table_headers: true,
            image_paste_behavior: ImagePasteBehavior::default(),
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LanguageSettingsFile {
    #[serde(default = "default_language_id_str")]
    default_language_id: String,
}

impl Default for LanguageSettingsFile {
    fn default() -> Self {
        Self {
            default_language_id: default_language_id_str(),
        }
    }
}

fn default_language_id_str() -> String {
    DEFAULT_LANGUAGE_ID.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ThemeSettingsFile {
    #[serde(default = "default_theme_id_str")]
    default_theme_id: String,
}

impl Default for ThemeSettingsFile {
    fn default() -> Self {
        Self {
            default_theme_id: default_theme_id_str(),
        }
    }
}

fn default_theme_id_str() -> String {
    DEFAULT_THEME_ID.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatusBarSettingsFile {
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default = "default_true")]
    show_word_count: bool,
    #[serde(default = "default_true")]
    show_cursor_position: bool,
}

impl Default for StatusBarSettingsFile {
    fn default() -> Self {
        Self {
            enabled: true,
            show_word_count: true,
            show_cursor_position: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ExplorerSettingsFile {
    #[serde(default)]
    hide_hidden: bool,
    #[serde(default)]
    sort_mode: ExplorerSortMode,
    #[serde(default)]
    sort_order: ExplorerSortOrder,
}

impl From<SettingsFile> for AppSettings {
    fn from(file: SettingsFile) -> Self {
        Self {
            startup_open: file.startup.open,
            default_language_id: file.language.default_language_id,
            default_theme_id: file.theme.default_theme_id,
            show_table_headers: file.editor.show_table_headers,
            image_paste_behavior: file.editor.image_paste_behavior,
            keybindings: normalize_shortcut_config(&file.keybindings),
            status_bar: StatusBarSettings {
                enabled: file.status_bar.enabled,
                show_word_count: file.status_bar.show_word_count,
                show_cursor_position: file.status_bar.show_cursor_position,
            },
            explorer: ExplorerSettings {
                hide_hidden: file.explorer.hide_hidden,
                sort_mode: file.explorer.sort_mode,
                sort_order: file.explorer.sort_order,
            },
            typography: file.typography,
        }
    }
}

impl From<&AppSettings> for SettingsFile {
    fn from(value: &AppSettings) -> Self {
        Self {
            startup: StartupSettingsFile {
                open: value.startup_open,
            },
            language: LanguageSettingsFile {
                default_language_id: value.default_language_id.clone(),
            },
            theme: ThemeSettingsFile {
                default_theme_id: value.default_theme_id.clone(),
            },
            editor: EditorSettingsFile {
                show_table_headers: value.show_table_headers,
                image_paste_behavior: value.image_paste_behavior,
            },
            status_bar: StatusBarSettingsFile {
                enabled: value.status_bar.enabled,
                show_word_count: value.status_bar.show_word_count,
                show_cursor_position: value.status_bar.show_cursor_position,
            },
            keybindings: normalize_shortcut_config(&value.keybindings),
            explorer: ExplorerSettingsFile {
                hide_hidden: value.explorer.hide_hidden,
                sort_mode: value.explorer.sort_mode,
                sort_order: value.explorer.sort_order,
            },
            typography: value.typography.clone(),
        }
    }
}

pub fn read_app_settings() -> anyhow::Result<AppSettings> {
    read_app_settings_with_dirs(&SplitypeConfigDirs::from_system()?)
}

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
    let file: SettingsFile = toml::from_str(&text).unwrap_or_default();
    Ok(AppSettings::from(file))
}

pub fn load_or_create_app_settings() -> anyhow::Result<AppSettings> {
    let dirs = SplitypeConfigDirs::from_system()?;
    load_or_create_app_settings_with_dirs_and_locales(&dirs, sys_locale::get_locales())
}

fn detected_language_id_from_locales<I, S>(locales: I) -> &'static str
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    language_id_for_locale_settings(locales)
}

fn load_or_create_app_settings_with_dirs_and_locales<I, S>(
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
        Ok(text) => match toml::from_str::<SettingsFile>(&text) {
            Ok(file) => AppSettings::from(file),
            Err(_) => AppSettings {
                default_language_id: detected_language_id.into(),
                ..AppSettings::default()
            },
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => AppSettings {
            default_language_id: detected_language_id.into(),
            ..AppSettings::default()
        },
        Err(err) => {
            return Err(err).with_context(|| format!("failed to read '{}'", path.display()));
        }
    };
    save_app_settings_with_dirs(&settings, dirs)?;
    Ok(settings)
}

pub fn save_app_settings(settings: &AppSettings) -> anyhow::Result<()> {
    save_app_settings_with_dirs(settings, &SplitypeConfigDirs::from_system()?)
}

pub fn save_app_settings_with_dirs(
    settings: &AppSettings,
    dirs: &SplitypeConfigDirs,
) -> anyhow::Result<()> {
    let path = dirs.app_config_file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create '{}'", parent.display()))?;
    }
    let text = toml::to_string_pretty(&SettingsFile::from(settings))?;
    std::fs::write(&path, text).with_context(|| format!("failed to write '{}'", path.display()))
}

pub fn first_existing_recent_markdown_file() -> Option<PathBuf> {
    let recent_files = read_recent_files().ok()?;
    recent_files.into_iter().find(|path| path.is_file())
}

pub fn apply_configured_language(cx: &mut App, language_id: &str) -> anyhow::Result<bool> {
    let mut applied = false;
    let changed = cx.update_global::<I18nManager, _>(|i18n_manager, _cx| {
        let changed = i18n_manager.set_language_by_id(language_id);
        applied = changed || i18n_manager.current_language_id() == language_id;
        changed
    });
    if !applied {
        return Ok(false);
    }
    update_app_settings(|settings| {
        settings.default_language_id = language_id.into();
    })?;
    Ok(changed)
}

pub fn apply_configured_theme(cx: &mut App, theme_id: &str) -> anyhow::Result<bool> {
    let mut applied = false;
    let changed = cx.update_global::<ThemeManager, _>(|theme_manager, _cx| {
        let changed = theme_manager.set_theme_by_id(theme_id);
        applied = changed || theme_manager.current_theme_id() == theme_id;
        changed
    });
    if !applied {
        return Ok(false);
    }
    update_app_settings(|settings| {
        settings.default_theme_id = theme_id.into();
    })?;
    Ok(changed)
}

pub fn import_language_config_and_select(
    cx: &mut App,
    path: impl AsRef<std::path::Path>,
) -> anyhow::Result<String> {
    let imported_id = cx.update_global::<I18nManager, _>(|i18n_manager, _cx| {
        i18n_manager.import_language_config(path)
    })?;
    update_app_settings(|settings| {
        settings.default_language_id = imported_id.clone();
    })?;
    Ok(imported_id)
}

pub fn import_theme_config_and_select(
    cx: &mut App,
    path: impl AsRef<std::path::Path>,
) -> anyhow::Result<String> {
    let imported_id = cx.update_global::<ThemeManager, _>(|theme_manager, _cx| {
        theme_manager.import_theme_config(path)
    })?;
    update_app_settings(|settings| {
        settings.default_theme_id = imported_id.clone();
    })?;
    Ok(imported_id)
}

pub fn save_settings_from_window(
    startup_open: StartupOpenSetting,
    default_theme_id: &str,
    image_paste_behavior: ImagePasteBehavior,
    keybindings: BTreeMap<String, Vec<String>>,
    status_bar: &StatusBarSettings,
    typography: &TypographySettings,
) -> anyhow::Result<AppSettings> {
    let dirs = SplitypeConfigDirs::from_system()?;
    save_settings_from_window_with_dirs(
        startup_open,
        default_theme_id,
        image_paste_behavior,
        keybindings,
        status_bar,
        typography,
        &dirs,
    )
}

pub fn save_settings_from_window_with_dirs(
    startup_open: StartupOpenSetting,
    default_theme_id: &str,
    image_paste_behavior: ImagePasteBehavior,
    keybindings: BTreeMap<String, Vec<String>>,
    status_bar: &StatusBarSettings,
    typography: &TypographySettings,
    dirs: &SplitypeConfigDirs,
) -> anyhow::Result<AppSettings> {
    let mut settings =
        load_or_create_app_settings_with_dirs_and_locales(dirs, sys_locale::get_locales())?;
    settings.startup_open = startup_open;
    settings.default_theme_id = default_theme_id.into();
    settings.image_paste_behavior = image_paste_behavior;
    settings.keybindings = normalize_shortcut_config(&keybindings);
    settings.status_bar = status_bar.clone();
    settings.typography = typography.clone();
    save_app_settings_with_dirs(&settings, dirs)?;
    Ok(settings)
}

fn update_app_settings(update: impl FnOnce(&mut AppSettings)) -> anyhow::Result<AppSettings> {
    let mut settings = load_or_create_app_settings()?;
    update(&mut settings);
    save_app_settings(&settings)?;
    Ok(settings)
}
