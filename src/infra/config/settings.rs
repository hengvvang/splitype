//! Persistent app settings and the settings window.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::Context as _;
use gpui::*;
use serde::{Deserialize, Serialize};

use crate::infra::config::dirs::SplitypeConfigDirs;
use crate::infra::config::recent::read_recent_files;
use crate::infra::i18n::manager::I18nManager;
use crate::infra::i18n::packs::language_id_for_locale_settings;
use crate::editor::keybindings::normalize_shortcut_config;
use crate::theme::ThemeManager;

pub(crate) const DEFAULT_THEME_ID: &str = "splitype";
const DEFAULT_LANGUAGE_ID: &str = "en-US";

/// A user-configurable button shown in the status bar.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StatusBarButton {
    pub id: String,
    pub label: String,
    pub action_id: String,
}

/// Status bar visibility and component toggles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StatusBarSettings {
    pub enabled: bool,
    pub show_word_count: bool,
    pub show_cursor_position: bool,
    pub show_sidebar_toggle: bool,
    pub show_mode_switch: bool,
    pub custom_buttons: Vec<StatusBarButton>,
}

impl Default for StatusBarSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            show_word_count: true,
            show_cursor_position: true,
            show_sidebar_toggle: true,
            show_mode_switch: true,
            custom_buttons: Vec::new(),
        }
    }
}

/// Startup document selection stored in `config.toml`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StartupOpenSetting {
    NewFile,
    LastOpenedFile,
}

impl StartupOpenSetting {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::NewFile => "new_file",
            Self::LastOpenedFile => "last_opened_file",
        }
    }

    fn from_str(value: &str) -> Self {
        match value {
            "last_opened_file" => Self::LastOpenedFile,
            _ => Self::NewFile,
        }
    }
}

/// Explorer tree sorting mode (mirrors Zed's `ProjectPanelSortMode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExplorerSortMode {
    DirectoriesFirst,
    FilesFirst,
    Mixed,
}

impl ExplorerSortMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::DirectoriesFirst => "directories_first",
            Self::FilesFirst => "files_first",
            Self::Mixed => "mixed",
        }
    }

    fn from_str(value: &str) -> Self {
        match value {
            "files_first" => Self::FilesFirst,
            "mixed" => Self::Mixed,
            _ => Self::DirectoriesFirst,
        }
    }
}

/// Explorer tree sort order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExplorerSortOrder {
    Ascending,
    Descending,
}

impl ExplorerSortOrder {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Ascending => "ascending",
            Self::Descending => "descending",
        }
    }

    fn from_str(value: &str) -> Self {
        match value {
            "descending" => Self::Descending,
            _ => Self::Ascending,
        }
    }
}

/// Explorer sidebar settings persisted in `config.toml`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ExplorerSettings {
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
                    eprintln!("failed to save explorer settings: {err}");
                }
            }
            Err(err) => eprintln!("failed to read explorer settings: {err}"),
        }
    }
}

/// Where pasted clipboard images should be stored before inserting Markdown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ImagePasteBehavior {
    None,
    CopyToDocumentFolder,
    CopyToAssetsFolder,
    CopyToNamedAssetsFolder,
}

impl ImagePasteBehavior {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::CopyToDocumentFolder => "copy_to_document_folder",
            Self::CopyToAssetsFolder => "copy_to_assets_folder",
            Self::CopyToNamedAssetsFolder => "copy_to_named_assets_folder",
        }
    }

    fn from_str(value: &str) -> Self {
        match value {
            "copy_to_document_folder" => Self::CopyToDocumentFolder,
            "copy_to_assets_folder" => Self::CopyToAssetsFolder,
            "copy_to_named_assets_folder" => Self::CopyToNamedAssetsFolder,
            _ => Self::None,
        }
    }
}

/// User settings persisted under the app config directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AppSettings {
    pub(crate) startup_open: StartupOpenSetting,
    pub(crate) default_language_id: String,
    pub(crate) default_theme_id: String,
    pub(crate) show_table_headers: bool,
    pub(crate) image_paste_behavior: ImagePasteBehavior,
    pub(crate) keybindings: BTreeMap<String, Vec<String>>,
    pub(crate) status_bar: StatusBarSettings,
    pub(crate) explorer: ExplorerSettings,
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
        }
    }
}

/// Runtime-accessible editor settings mirrored from [`AppSettings`] so the
/// render path can read them without touching disk. Toggling persists the new
/// value back to the settings file.
pub struct EditorSettings {
    show_table_headers: bool,
    pub(crate) status_bar_settings: StatusBarSettings,
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
                show_sidebar_toggle: status_bar.show_sidebar_toggle,
                show_mode_switch: status_bar.show_mode_switch,
                custom_buttons: Vec::new(),
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
                show_sidebar_toggle: s.status_bar_settings.show_sidebar_toggle,
                show_mode_switch: s.status_bar_settings.show_mode_switch,
                custom_buttons: Vec::new(),
            })
            .unwrap_or_default();
        Self::set_global(cx, show_table_headers, &status_bar);
        match read_app_settings() {
            Ok(mut settings) => {
                settings.show_table_headers = show_table_headers;
                if let Err(err) = save_app_settings(&settings) {
                    eprintln!("failed to save table header setting: {err}");
                }
            }
            Err(err) => eprintln!("failed to read table header setting: {err}"),
        }
    }

    pub fn status_bar_settings(cx: &App) -> StatusBarSettings {
        cx.try_global::<Self>()
            .map(|s| StatusBarSettings {
                enabled: s.status_bar_settings.enabled,
                show_word_count: s.status_bar_settings.show_word_count,
                show_cursor_position: s.status_bar_settings.show_cursor_position,
                show_sidebar_toggle: s.status_bar_settings.show_sidebar_toggle,
                show_mode_switch: s.status_bar_settings.show_mode_switch,
                custom_buttons: Vec::new(),
            })
            .unwrap_or_default()
    }
}

#[derive(Serialize)]
struct SettingsFile {
    startup: StartupSettingsFile,
    language: LanguageSettingsFile,
    theme: ThemeSettingsFile,
    editor: EditorSettingsFile,
    status_bar: StatusBarSettingsFile,
    keybindings: BTreeMap<String, Vec<String>>,
    explorer: ExplorerSettingsFile,
}

#[derive(Serialize)]
struct StartupSettingsFile {
    open: String,
}

#[derive(Serialize)]
struct EditorSettingsFile {
    show_table_headers: bool,
    image_paste_behavior: String,
}

#[derive(Serialize)]
struct LanguageSettingsFile {
    default_language_id: String,
}

#[derive(Serialize)]
struct ThemeSettingsFile {
    default_theme_id: String,
}

#[derive(Serialize)]
struct StatusBarSettingsFile {
    enabled: bool,
    show_word_count: bool,
    show_cursor_position: bool,
    show_sidebar_toggle: bool,
    show_mode_switch: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    custom_buttons: Vec<StatusBarButton>,
}

#[derive(Serialize)]
struct ExplorerSettingsFile {
    hide_hidden: bool,
    sort_mode: String,
    sort_order: String,
}

impl From<&StatusBarSettings> for StatusBarSettingsFile {
    fn from(value: &StatusBarSettings) -> Self {
        Self {
            enabled: value.enabled,
            show_word_count: value.show_word_count,
            show_cursor_position: value.show_cursor_position,
            show_sidebar_toggle: value.show_sidebar_toggle,
            show_mode_switch: value.show_mode_switch,
            custom_buttons: value.custom_buttons.clone(),
        }
    }
}

impl From<&AppSettings> for SettingsFile {
    fn from(value: &AppSettings) -> Self {
        Self {
            startup: StartupSettingsFile {
                open: value.startup_open.as_str().into(),
            },
            language: LanguageSettingsFile {
                default_language_id: value.default_language_id.clone(),
            },
            theme: ThemeSettingsFile {
                default_theme_id: value.default_theme_id.clone(),
            },
            editor: EditorSettingsFile {
                show_table_headers: value.show_table_headers,
                image_paste_behavior: value.image_paste_behavior.as_str().into(),
            },
            status_bar: StatusBarSettingsFile::from(&value.status_bar),
            keybindings: normalize_shortcut_config(&value.keybindings),
            explorer: ExplorerSettingsFile {
                hide_hidden: value.explorer.hide_hidden,
                sort_mode: value.explorer.sort_mode.as_str().into(),
                sort_order: value.explorer.sort_order.as_str().into(),
            },
        }
    }
}

pub(crate) fn read_app_settings() -> anyhow::Result<AppSettings> {
    read_app_settings_with_dirs(&SplitypeConfigDirs::from_system()?)
}

pub(crate) fn read_app_settings_with_dirs(
    dirs: &SplitypeConfigDirs,
) -> anyhow::Result<AppSettings> {
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
    let Ok(value) = toml::from_str::<toml::Value>(&text) else {
        return Ok(AppSettings::default());
    };

    Ok(app_settings_from_toml_value(&value, DEFAULT_LANGUAGE_ID))
}

pub(crate) fn load_or_create_app_settings() -> anyhow::Result<AppSettings> {
    let dirs = SplitypeConfigDirs::from_system()?;
    load_or_create_app_settings_with_dirs_and_locales(&dirs, sys_locale::get_locales())
}

fn app_settings_from_toml_value(value: &toml::Value, fallback_language_id: &str) -> AppSettings {
    let startup_open = value
        .get("startup")
        .and_then(|startup| startup.get("open"))
        .and_then(|open| open.as_str())
        .map(StartupOpenSetting::from_str)
        .unwrap_or(StartupOpenSetting::NewFile);
    let default_language_id = value
        .get("language")
        .and_then(|language| language.get("default_language_id"))
        .and_then(|id| id.as_str())
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .unwrap_or(fallback_language_id)
        .to_string();
    let default_theme_id = value
        .get("theme")
        .and_then(|theme| theme.get("default_theme_id"))
        .and_then(|id| id.as_str())
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .unwrap_or(DEFAULT_THEME_ID)
        .to_string();
    let keybindings = value
        .get("keybindings")
        .and_then(|keybindings| keybindings.as_table())
        .map(|table| {
            table
                .iter()
                .filter_map(|(key, value)| {
                    let keys = value
                        .as_array()?
                        .iter()
                        .filter_map(|value| value.as_str().map(str::to_string))
                        .collect::<Vec<_>>();
                    Some((key.clone(), keys))
                })
                .collect::<BTreeMap<_, _>>()
        })
        .map(|keybindings| normalize_shortcut_config(&keybindings))
        .unwrap_or_default();

    let show_table_headers = value
        .get("editor")
        .and_then(|editor| editor.get("show_table_headers"))
        .and_then(|value| value.as_bool())
        .unwrap_or(true);
    let image_paste_behavior = value
        .get("editor")
        .and_then(|editor| editor.get("image_paste_behavior"))
        .and_then(|value| value.as_str())
        .map(ImagePasteBehavior::from_str)
        .unwrap_or(ImagePasteBehavior::None);

    let status_bar = value
        .get("status_bar")
        .map(|sb| {
            let enabled = sb.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
            let show_word_count = sb
                .get("show_word_count")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let show_cursor_position = sb
                .get("show_cursor_position")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let show_sidebar_toggle = sb
                .get("show_sidebar_toggle")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let show_mode_switch = sb
                .get("show_mode_switch")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let custom_buttons = sb
                .get("custom_buttons")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|item| {
                            let id = item.get("id")?.as_str()?.to_string();
                            let label = item.get("label")?.as_str()?.to_string();
                            Some(StatusBarButton {
                                id,
                                label,
                                action_id: item
                                    .get("action_id")
                                    .and_then(|a| a.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            StatusBarSettings {
                enabled,
                show_word_count,
                show_cursor_position,
                show_sidebar_toggle,
                show_mode_switch,
                custom_buttons,
            }
        })
        .unwrap_or_default();

    let explorer = value
        .get("explorer")
        .map(|explorer| ExplorerSettings {
            hide_hidden: explorer
                .get("hide_hidden")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            sort_mode: explorer
                .get("sort_mode")
                .and_then(|v| v.as_str())
                .map(ExplorerSortMode::from_str)
                .unwrap_or(ExplorerSortMode::DirectoriesFirst),
            sort_order: explorer
                .get("sort_order")
                .and_then(|v| v.as_str())
                .map(ExplorerSortOrder::from_str)
                .unwrap_or(ExplorerSortOrder::Ascending),
        })
        .unwrap_or_default();

    AppSettings {
        startup_open,
        default_language_id,
        default_theme_id,
        show_table_headers,
        image_paste_behavior,
        keybindings,
        status_bar,
        explorer,
    }
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
        Ok(text) => toml::from_str::<toml::Value>(&text)
            .map(|value| app_settings_from_toml_value(&value, detected_language_id))
            .unwrap_or_else(|_| AppSettings {
                default_language_id: detected_language_id.into(),
                ..AppSettings::default()
            }),
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

pub(crate) fn save_app_settings(settings: &AppSettings) -> anyhow::Result<()> {
    save_app_settings_with_dirs(settings, &SplitypeConfigDirs::from_system()?)
}

pub(crate) fn save_app_settings_with_dirs(
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

pub(crate) fn first_existing_recent_markdown_file() -> Option<PathBuf> {
    let recent_files = read_recent_files().ok()?;
    recent_files.into_iter().find(|path| path.is_file())
}

pub(crate) fn apply_configured_language(cx: &mut App, language_id: &str) -> anyhow::Result<bool> {
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

pub(crate) fn apply_configured_theme(cx: &mut App, theme_id: &str) -> anyhow::Result<bool> {
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

pub(crate) fn import_language_config_and_select(
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

pub(crate) fn import_theme_config_and_select(
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

pub(crate) fn save_settings_from_window(
    startup_open: StartupOpenSetting,
    default_theme_id: &str,
    image_paste_behavior: ImagePasteBehavior,
    keybindings: BTreeMap<String, Vec<String>>,
    status_bar: &StatusBarSettings,
) -> anyhow::Result<AppSettings> {
    let dirs = SplitypeConfigDirs::from_system()?;
    save_settings_from_window_with_dirs(
        startup_open,
        default_theme_id,
        image_paste_behavior,
        keybindings,
        status_bar,
        &dirs,
    )
}

fn save_settings_from_window_with_dirs(
    startup_open: StartupOpenSetting,
    default_theme_id: &str,
    image_paste_behavior: ImagePasteBehavior,
    keybindings: BTreeMap<String, Vec<String>>,
    status_bar: &StatusBarSettings,
    dirs: &SplitypeConfigDirs,
) -> anyhow::Result<AppSettings> {
    let mut settings =
        load_or_create_app_settings_with_dirs_and_locales(dirs, sys_locale::get_locales())?;
    settings.startup_open = startup_open;
    settings.default_theme_id = default_theme_id.into();
    settings.image_paste_behavior = image_paste_behavior;
    settings.keybindings = normalize_shortcut_config(&keybindings);
    settings.status_bar = status_bar.clone();
    save_app_settings_with_dirs(&settings, dirs)?;
    Ok(settings)
}

fn update_app_settings(update: impl FnOnce(&mut AppSettings)) -> anyhow::Result<AppSettings> {
    let mut settings = load_or_create_app_settings()?;
    update(&mut settings);
    save_app_settings(&settings)?;
    Ok(settings)
}
