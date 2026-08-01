//! Persistent app preferences and the preferences window.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::Context as _;
use gpui::prelude::FluentBuilder;
use gpui::*;
use serde::{Deserialize, Serialize};

use super::{VelotypeConfigDirs, read_recent_files};
use crate::components::{
    ShortcutCommand, install_keybindings, normalize_shortcut_config, switch::Switch,
};
use crate::i18n::{I18nManager, language_id_for_locale_preferences};
use crate::theme::{ThemeCatalogEntry, ThemeManager};
use crate::window_chrome::{
    custom_titlebar_height, render_custom_titlebar, velotype_window_options,
};

const DEFAULT_THEME_ID: &str = "velotype";
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
pub(crate) struct StatusBarPreferences {
    pub enabled: bool,
    pub show_word_count: bool,
    pub show_cursor_position: bool,
    pub show_sidebar_toggle: bool,
    pub show_mode_switch: bool,
    pub custom_buttons: Vec<StatusBarButton>,
}

impl Default for StatusBarPreferences {
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
pub(crate) enum StartupOpenPreference {
    NewFile,
    LastOpenedFile,
}

impl StartupOpenPreference {
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

/// User preferences persisted under the app config directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AppPreferences {
    pub(crate) startup_open: StartupOpenPreference,
    pub(crate) default_language_id: String,
    pub(crate) default_theme_id: String,
    pub(crate) show_table_headers: bool,
    pub(crate) image_paste_behavior: ImagePasteBehavior,
    pub(crate) keybindings: BTreeMap<String, Vec<String>>,
    pub(crate) status_bar: StatusBarPreferences,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            startup_open: StartupOpenPreference::NewFile,
            default_language_id: DEFAULT_LANGUAGE_ID.into(),
            default_theme_id: DEFAULT_THEME_ID.into(),
            show_table_headers: true,
            image_paste_behavior: ImagePasteBehavior::None,
            keybindings: BTreeMap::new(),
            status_bar: StatusBarPreferences::default(),
        }
    }
}

/// Status Bar Settings
struct StatusBarSettings {
    status_bar_enabled: bool,
    status_bar_show_word_count: bool,
    status_bar_show_cursor_position: bool,
    status_bar_show_sidebar_toggle: bool,
    status_bar_show_mode_switch: bool,
}

/// Runtime-accessible editor settings mirrored from [`AppPreferences`] so the
/// render path can read them without touching disk. Toggling persists the new
/// value back to the preferences file.
pub struct EditorSettings {
    show_table_headers: bool,
    status_bar_settings: StatusBarSettings,
}

impl Global for EditorSettings {}

impl EditorSettings {
    pub fn init(cx: &mut App, show_table_headers: bool) {
        let status_bar = read_app_preferences()
            .ok()
            .map(|p| p.status_bar)
            .unwrap_or_default();
        Self::set_global(cx, show_table_headers, &status_bar);
    }

    fn set_global(cx: &mut App, show_table_headers: bool, status_bar: &StatusBarPreferences) {
        cx.set_global(Self {
            show_table_headers,
            status_bar_settings: StatusBarSettings {
                status_bar_enabled: status_bar.enabled,
                status_bar_show_word_count: status_bar.show_word_count,
                status_bar_show_cursor_position: status_bar.show_cursor_position,
                status_bar_show_sidebar_toggle: status_bar.show_sidebar_toggle,
                status_bar_show_mode_switch: status_bar.show_mode_switch,
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
            .map(|s| StatusBarPreferences {
                enabled: s.status_bar_settings.status_bar_enabled,
                show_word_count: s.status_bar_settings.status_bar_show_word_count,
                show_cursor_position: s.status_bar_settings.status_bar_show_cursor_position,
                show_sidebar_toggle: s.status_bar_settings.status_bar_show_sidebar_toggle,
                show_mode_switch: s.status_bar_settings.status_bar_show_mode_switch,
                custom_buttons: Vec::new(),
            })
            .unwrap_or_default();
        Self::set_global(cx, show_table_headers, &status_bar);
        match read_app_preferences() {
            Ok(mut preferences) => {
                preferences.show_table_headers = show_table_headers;
                if let Err(err) = save_app_preferences(&preferences) {
                    eprintln!("failed to save table header preference: {err}");
                }
            }
            Err(err) => eprintln!("failed to read table header preference: {err}"),
        }
    }

    pub fn status_bar_preferences(cx: &App) -> StatusBarPreferences {
        cx.try_global::<Self>()
            .map(|s| StatusBarPreferences {
                enabled: s.status_bar_settings.status_bar_enabled,
                show_word_count: s.status_bar_settings.status_bar_show_word_count,
                show_cursor_position: s.status_bar_settings.status_bar_show_cursor_position,
                show_sidebar_toggle: s.status_bar_settings.status_bar_show_sidebar_toggle,
                show_mode_switch: s.status_bar_settings.status_bar_show_mode_switch,
                custom_buttons: Vec::new(),
            })
            .unwrap_or_default()
    }
}

#[derive(Serialize)]
struct PreferencesFile {
    startup: StartupPreferencesFile,
    language: LanguagePreferencesFile,
    theme: ThemePreferencesFile,
    editor: EditorPreferencesFile,
    status_bar: StatusBarPreferencesFile,
    keybindings: BTreeMap<String, Vec<String>>,
}

#[derive(Serialize)]
struct StartupPreferencesFile {
    open: String,
}

#[derive(Serialize)]
struct EditorPreferencesFile {
    show_table_headers: bool,
    image_paste_behavior: String,
}

#[derive(Serialize)]
struct LanguagePreferencesFile {
    default_language_id: String,
}

#[derive(Serialize)]
struct ThemePreferencesFile {
    default_theme_id: String,
}

#[derive(Serialize)]
struct StatusBarPreferencesFile {
    enabled: bool,
    show_word_count: bool,
    show_cursor_position: bool,
    show_sidebar_toggle: bool,
    show_mode_switch: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    custom_buttons: Vec<StatusBarButton>,
}

impl From<&StatusBarPreferences> for StatusBarPreferencesFile {
    fn from(value: &StatusBarPreferences) -> Self {
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

impl From<&AppPreferences> for PreferencesFile {
    fn from(value: &AppPreferences) -> Self {
        Self {
            startup: StartupPreferencesFile {
                open: value.startup_open.as_str().into(),
            },
            language: LanguagePreferencesFile {
                default_language_id: value.default_language_id.clone(),
            },
            theme: ThemePreferencesFile {
                default_theme_id: value.default_theme_id.clone(),
            },
            editor: EditorPreferencesFile {
                show_table_headers: value.show_table_headers,
                image_paste_behavior: value.image_paste_behavior.as_str().into(),
            },
            status_bar: StatusBarPreferencesFile::from(&value.status_bar),
            keybindings: normalize_shortcut_config(&value.keybindings),
        }
    }
}

pub(crate) fn read_app_preferences() -> anyhow::Result<AppPreferences> {
    read_app_preferences_with_dirs(&VelotypeConfigDirs::from_system()?)
}

pub(crate) fn read_app_preferences_with_dirs(
    dirs: &VelotypeConfigDirs,
) -> anyhow::Result<AppPreferences> {
    let path = dirs.app_config_file();
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(AppPreferences::default());
        }
        Err(err) => {
            return Err(err).with_context(|| format!("failed to read '{}'", path.display()));
        }
    };
    let Ok(value) = toml::from_str::<toml::Value>(&text) else {
        return Ok(AppPreferences::default());
    };

    Ok(app_preferences_from_toml_value(&value, DEFAULT_LANGUAGE_ID))
}

pub(crate) fn load_or_create_app_preferences() -> anyhow::Result<AppPreferences> {
    let dirs = VelotypeConfigDirs::from_system()?;
    load_or_create_app_preferences_with_dirs_and_locales(&dirs, sys_locale::get_locales())
}

fn app_preferences_from_toml_value(
    value: &toml::Value,
    fallback_language_id: &str,
) -> AppPreferences {
    let startup_open = value
        .get("startup")
        .and_then(|startup| startup.get("open"))
        .and_then(|open| open.as_str())
        .map(StartupOpenPreference::from_str)
        .unwrap_or(StartupOpenPreference::NewFile);
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
            StatusBarPreferences {
                enabled,
                show_word_count,
                show_cursor_position,
                show_sidebar_toggle,
                show_mode_switch,
                custom_buttons,
            }
        })
        .unwrap_or_default();

    AppPreferences {
        startup_open,
        default_language_id,
        default_theme_id,
        show_table_headers,
        image_paste_behavior,
        keybindings,
        status_bar,
    }
}

fn detected_language_id_from_locales<I, S>(locales: I) -> &'static str
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    language_id_for_locale_preferences(locales)
}

fn load_or_create_app_preferences_with_dirs_and_locales<I, S>(
    dirs: &VelotypeConfigDirs,
    locales: I,
) -> anyhow::Result<AppPreferences>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let detected_language_id = detected_language_id_from_locales(locales);
    let path = dirs.app_config_file();
    let preferences = match std::fs::read_to_string(&path) {
        Ok(text) => toml::from_str::<toml::Value>(&text)
            .map(|value| app_preferences_from_toml_value(&value, detected_language_id))
            .unwrap_or_else(|_| AppPreferences {
                default_language_id: detected_language_id.into(),
                ..AppPreferences::default()
            }),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => AppPreferences {
            default_language_id: detected_language_id.into(),
            ..AppPreferences::default()
        },
        Err(err) => {
            return Err(err).with_context(|| format!("failed to read '{}'", path.display()));
        }
    };
    save_app_preferences_with_dirs(&preferences, dirs)?;
    Ok(preferences)
}

pub(crate) fn save_app_preferences(preferences: &AppPreferences) -> anyhow::Result<()> {
    save_app_preferences_with_dirs(preferences, &VelotypeConfigDirs::from_system()?)
}

pub(crate) fn save_app_preferences_with_dirs(
    preferences: &AppPreferences,
    dirs: &VelotypeConfigDirs,
) -> anyhow::Result<()> {
    let path = dirs.app_config_file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create '{}'", parent.display()))?;
    }
    let text = toml::to_string_pretty(&PreferencesFile::from(preferences))?;
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
    update_app_preferences(|preferences| {
        preferences.default_language_id = language_id.into();
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
    update_app_preferences(|preferences| {
        preferences.default_theme_id = theme_id.into();
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
    update_app_preferences(|preferences| {
        preferences.default_language_id = imported_id.clone();
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
    update_app_preferences(|preferences| {
        preferences.default_theme_id = imported_id.clone();
    })?;
    Ok(imported_id)
}

pub(crate) fn save_preferences_from_window(
    startup_open: StartupOpenPreference,
    default_theme_id: &str,
    image_paste_behavior: ImagePasteBehavior,
    keybindings: BTreeMap<String, Vec<String>>,
    status_bar: &StatusBarPreferences,
) -> anyhow::Result<AppPreferences> {
    let dirs = VelotypeConfigDirs::from_system()?;
    save_preferences_from_window_with_dirs(
        startup_open,
        default_theme_id,
        image_paste_behavior,
        keybindings,
        status_bar,
        &dirs,
    )
}

fn save_preferences_from_window_with_dirs(
    startup_open: StartupOpenPreference,
    default_theme_id: &str,
    image_paste_behavior: ImagePasteBehavior,
    keybindings: BTreeMap<String, Vec<String>>,
    status_bar: &StatusBarPreferences,
    dirs: &VelotypeConfigDirs,
) -> anyhow::Result<AppPreferences> {
    let mut preferences =
        load_or_create_app_preferences_with_dirs_and_locales(dirs, sys_locale::get_locales())?;
    preferences.startup_open = startup_open;
    preferences.default_theme_id = default_theme_id.into();
    preferences.image_paste_behavior = image_paste_behavior;
    preferences.keybindings = normalize_shortcut_config(&keybindings);
    preferences.status_bar = status_bar.clone();
    save_app_preferences_with_dirs(&preferences, dirs)?;
    Ok(preferences)
}

fn update_app_preferences(
    update: impl FnOnce(&mut AppPreferences),
) -> anyhow::Result<AppPreferences> {
    let mut preferences = load_or_create_app_preferences()?;
    update(&mut preferences);
    save_app_preferences(&preferences)?;
    Ok(preferences)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PreferencesNav {
    Interface,
    Editing,
    Keymap,
}

/// Independent preferences window view.
pub(crate) struct PreferencesWindow {
    nav: PreferencesNav,
    startup_open: StartupOpenPreference,
    selected_theme_id: String,
    image_paste_behavior: ImagePasteBehavior,
    keybindings: BTreeMap<String, Vec<String>>,
    saved_startup_open: StartupOpenPreference,
    saved_theme_id: String,
    saved_image_paste_behavior: ImagePasteBehavior,
    saved_keybindings: BTreeMap<String, Vec<String>>,
    theme_options: Vec<ThemeCatalogEntry>,
    focus_handle: FocusHandle,
    startup_dropdown_open: bool,
    theme_dropdown_open: bool,
    image_dropdown_open: bool,
    #[allow(dead_code)]
    recording_shortcut: Option<ShortcutCommand>,
    #[allow(dead_code)]
    shortcut_error: Option<String>,
    status_bar_enabled: bool,
    status_bar_show_word_count: bool,
    status_bar_show_cursor_position: bool,
    status_bar_show_sidebar_toggle: bool,
    status_bar_show_mode_switch: bool,
    saved_status_bar_enabled: bool,
    saved_status_bar_show_word_count: bool,
    saved_status_bar_show_cursor_position: bool,
    saved_status_bar_show_sidebar_toggle: bool,
    saved_status_bar_show_mode_switch: bool,
}

impl PreferencesWindow {
    fn new(
        preferences: AppPreferences,
        theme_options: Vec<ThemeCatalogEntry>,
        cx: &mut Context<Self>,
    ) -> Self {
        let selected_theme_id = if theme_options
            .iter()
            .any(|entry| entry.id == preferences.default_theme_id)
        {
            preferences.default_theme_id
        } else {
            DEFAULT_THEME_ID.into()
        };
        let startup_open = preferences.startup_open;
        let image_paste_behavior = preferences.image_paste_behavior;
        let keybindings = preferences.keybindings;
        Self {
            nav: PreferencesNav::Interface,
            startup_open,
            selected_theme_id: selected_theme_id.clone(),
            image_paste_behavior,
            keybindings: keybindings.clone(),
            saved_startup_open: startup_open,
            saved_theme_id: selected_theme_id,
            saved_image_paste_behavior: image_paste_behavior,
            saved_keybindings: keybindings,
            theme_options,
            focus_handle: cx.focus_handle(),
            startup_dropdown_open: false,
            theme_dropdown_open: false,
            image_dropdown_open: false,
            recording_shortcut: None,
            shortcut_error: None,
            status_bar_enabled: preferences.status_bar.enabled,
            status_bar_show_word_count: preferences.status_bar.show_word_count,
            status_bar_show_cursor_position: preferences.status_bar.show_cursor_position,
            status_bar_show_sidebar_toggle: preferences.status_bar.show_sidebar_toggle,
            status_bar_show_mode_switch: preferences.status_bar.show_mode_switch,
            saved_status_bar_enabled: preferences.status_bar.enabled,
            saved_status_bar_show_word_count: preferences.status_bar.show_word_count,
            saved_status_bar_show_cursor_position: preferences.status_bar.show_cursor_position,
            saved_status_bar_show_sidebar_toggle: preferences.status_bar.show_sidebar_toggle,
            saved_status_bar_show_mode_switch: preferences.status_bar.show_mode_switch,
        }
    }

    fn selected_theme_name(&self) -> String {
        self.theme_options
            .iter()
            .find(|entry| entry.id == self.selected_theme_id)
            .map(|entry| entry.name.clone())
            .unwrap_or_else(|| "Velotype".into())
    }

    fn has_unsaved_changes(&self) -> bool {
        self.startup_open != self.saved_startup_open
            || self.selected_theme_id != self.saved_theme_id
            || self.image_paste_behavior != self.saved_image_paste_behavior
            || normalize_shortcut_config(&self.keybindings)
                != normalize_shortcut_config(&self.saved_keybindings)
            || self.status_bar_enabled != self.saved_status_bar_enabled
            || self.status_bar_show_word_count != self.saved_status_bar_show_word_count
            || self.status_bar_show_cursor_position != self.saved_status_bar_show_cursor_position
            || self.status_bar_show_sidebar_toggle != self.saved_status_bar_show_sidebar_toggle
            || self.status_bar_show_mode_switch != self.saved_status_bar_show_mode_switch
    }

    fn cancel(&mut self, _: &ClickEvent, window: &mut Window, _: &mut Context<Self>) {
        window.remove_window();
    }

    fn on_titlebar_close(
        &mut self,
        event: &ClickEvent,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        if event.standard_click() {
            window.remove_window();
        }
    }

    fn save(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_unsaved_changes() {
            return;
        }

        let preferences = match save_preferences_from_window(
            self.startup_open,
            &self.selected_theme_id,
            self.image_paste_behavior,
            self.keybindings.clone(),
            &StatusBarPreferences {
                enabled: self.status_bar_enabled,
                show_word_count: self.status_bar_show_word_count,
                show_cursor_position: self.status_bar_show_cursor_position,
                show_sidebar_toggle: self.status_bar_show_sidebar_toggle,
                show_mode_switch: self.status_bar_show_mode_switch,
                custom_buttons: Vec::new(),
            },
        ) {
            Ok(preferences) => preferences,
            Err(err) => {
                let strings = cx.global::<I18nManager>().strings().clone();
                let ok = strings.info_dialog_ok;
                let buttons = [ok.as_str()];
                let _ = window.prompt(
                    PromptLevel::Critical,
                    &strings.preferences_save_failed_title,
                    Some(&err.to_string()),
                    &buttons,
                    cx,
                );
                return;
            }
        };

        self.apply_saved_preferences(preferences, window, cx);
    }

    fn apply_saved_preferences(
        &mut self,
        preferences: AppPreferences,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let theme_changed = cx.update_global::<ThemeManager, _>(|theme_manager, _cx| {
            theme_manager.set_theme_by_id(&preferences.default_theme_id)
        });
        if !theme_changed {
            let _ = cx.update_global::<ThemeManager, _>(|theme_manager, _cx| {
                theme_manager.set_theme_by_id(DEFAULT_THEME_ID)
            });
        }
        cx.clear_key_bindings();
        install_keybindings(cx, &preferences.keybindings);
        crate::app_menu::install_menus(cx);
        cx.update_global::<EditorSettings, _>(|settings, _cx| {
            settings.status_bar_settings.status_bar_enabled = preferences.status_bar.enabled;
            settings.status_bar_settings.status_bar_show_word_count =
                preferences.status_bar.show_word_count;
            settings.status_bar_settings.status_bar_show_cursor_position =
                preferences.status_bar.show_cursor_position;
            settings.status_bar_settings.status_bar_show_sidebar_toggle =
                preferences.status_bar.show_sidebar_toggle;
            settings.status_bar_settings.status_bar_show_mode_switch =
                preferences.status_bar.show_mode_switch;
        });
        cx.refresh_windows();
        window.activate_window();
        self.focus_handle.focus(window);
        self.saved_startup_open = self.startup_open;
        self.saved_theme_id = self.selected_theme_id.clone();
        self.saved_image_paste_behavior = self.image_paste_behavior;
        self.saved_keybindings = normalize_shortcut_config(&self.keybindings);
        self.saved_status_bar_enabled = self.status_bar_enabled;
        self.saved_status_bar_show_word_count = self.status_bar_show_word_count;
        self.saved_status_bar_show_cursor_position = self.status_bar_show_cursor_position;
        self.saved_status_bar_show_sidebar_toggle = self.status_bar_show_sidebar_toggle;
        self.saved_status_bar_show_mode_switch = self.status_bar_show_mode_switch;
        cx.notify();
    }
}

impl Render for PreferencesWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeManager>().current().clone();
        let strings = cx.global::<I18nManager>().strings().clone();
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let can_save = self.has_unsaved_changes();
        let window_title =
            SharedString::from(format!("Velotype - {}", strings.preferences_window_title));
        window.set_window_title(window_title.as_ref());
        let titlebar_height = custom_titlebar_height(window, d);

        // Left Sidebar Navigation
        let nav_item = |id: &'static str, label: &'static str, is_selected: bool| -> AnyElement {
            div()
                .id(id)
                .cursor_pointer()
                .w_full()
                .px(px(14.0))
                .py(px(8.0))
                .rounded(px(d.menu_item_radius))
                .bg(if is_selected { c.dialog_secondary_button_hover } else { c.editor_background })
                .hover(|this| this.bg(c.dialog_secondary_button_hover))
                .text_size(px(13.0))
                .font_weight(if is_selected { gpui::FontWeight::BOLD } else { gpui::FontWeight::MEDIUM })
                .text_color(if is_selected { c.text_default } else { c.dialog_muted })
                .child(label)
                .into_any_element()
        };

        let win_ed1 = cx.entity().downgrade();
        let nav_interface = div().id("win-nav-wrap-1").w_full().child(nav_item("nav-interface", "Interface", self.nav == PreferencesNav::Interface)).on_click({
            let win_ed = win_ed1.clone();
            move |_ev, _win, cx| {
                let _ = win_ed.update(cx, |this, cx| {
                    this.nav = PreferencesNav::Interface;
                    cx.notify();
                });
            }
        });

        let win_ed2 = cx.entity().downgrade();
        let nav_editing = div().id("win-nav-wrap-2").w_full().child(nav_item("nav-editing", "Editing", self.nav == PreferencesNav::Editing)).on_click({
            let win_ed = win_ed2.clone();
            move |_ev, _win, cx| {
                let _ = win_ed.update(cx, |this, cx| {
                    this.nav = PreferencesNav::Editing;
                    cx.notify();
                });
            }
        });

        let win_ed3 = cx.entity().downgrade();
        let nav_keymap = div().id("win-nav-wrap-3").w_full().child(nav_item("nav-keymap", "Keymap", self.nav == PreferencesNav::Keymap)).on_click({
            let win_ed = win_ed3.clone();
            move |_ev, _win, cx| {
                let _ = win_ed.update(cx, |this, cx| {
                    this.nav = PreferencesNav::Keymap;
                    cx.notify();
                });
            }
        });

        let left_nav = div()
            .w(px(160.0))
            .h_full()
            .p(px(12.0))
            .border_r_1()
            .border_color(c.dialog_border)
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(nav_interface)
            .child(nav_editing)
            .child(nav_keymap);

        let inner_border_color = c.dialog_border;
        let make_row = |title: &'static str, desc: &'static str, ctrl: AnyElement| -> AnyElement {
            div()
                .w_full()
                .h(px(56.0))
                .px(px(14.0))
                .py(px(8.0))
                .rounded(px(d.menu_item_radius))
                .border_1()
                .border_color(inner_border_color)
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            div()
                                .text_size(px(12.5))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(c.text_default)
                                .child(title),
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(c.dialog_muted)
                                .child(desc),
                        ),
                )
                .child(ctrl)
                .into_any_element()
        };

        let make_section = |sec_id: &'static str, title: &'static str, items: Vec<AnyElement>| -> AnyElement {
            div()
                .relative()
                .w_full()
                .rounded(px(d.menu_panel_radius))
                .bg(c.dialog_surface)
                .border_1()
                .border_color(c.dialog_border)
                .flex()
                .flex_col()
                .child(
                    div()
                        .id(sec_id)
                        .w_full()
                        .px(px(14.0))
                        .py(px(10.0))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .text_size(px(13.0))
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(c.text_default)
                                .child(title),
                        ),
                )
                .child(
                    div()
                        .w_full()
                        .px(px(10.0))
                        .pb(px(10.0))
                        .pt(px(2.0))
                        .flex()
                        .flex_col()
                        .gap(px(8.0))
                        .children(items),
                )
                .into_any_element()
        };

        let mut sections = Vec::new();
        let mut active_panel_overlay: Option<AnyElement> = None;

        match self.nav {
            PreferencesNav::Interface => {
                // Section 1: Visual Theme & Language
                let mut sec1_items = Vec::new();
                let selected_theme_name = self.selected_theme_name();
                let theme_display_label: String = match selected_theme_name.as_str() {
                    "Velotype" | "Dark" => "Dark".to_string(),
                    "Velotype Light" | "Light" => "Light".to_string(),
                    other => other.to_string(),
                };
                let theme_icon_path = if theme_display_label == "Light" {
                    "icon/panel/sun.svg"
                } else {
                    "icon/panel/moon.svg"
                };

                let theme_btn_ed = cx.entity().downgrade();
                let ctrl_theme_btn = div()
                    .id("pref-btn-win-theme")
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_between()
                    .w(px(210.0))
                    .px(px(12.0))
                    .py(px(5.0))
                    .rounded(px(d.menu_item_radius))
                    .bg(c.dialog_secondary_button_bg)
                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                    .border_1()
                    .border_color(c.dialog_border)
                    .text_size(px(12.0))
                    .text_color(c.text_default)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                svg()
                                    .path(theme_icon_path)
                                    .size(px(13.0))
                                    .text_color(c.text_default),
                            )
                            .child(theme_display_label.clone()),
                    )
                    .child(
                        svg()
                            .path("icon/panel/select-chevron.svg")
                            .size(px(14.0))
                            .text_color(c.dialog_muted),
                    )
                    .on_click(move |_ev, _win, cx| {
                        let _ = theme_btn_ed.update(cx, |this, cx| {
                            this.theme_dropdown_open = !this.theme_dropdown_open;
                            cx.notify();
                        });
                    })
                    .into_any_element();

                sec1_items.push(make_row(
                    "Interface Theme",
                    "Customize overall application color scheme and appearance",
                    ctrl_theme_btn,
                ));

                let ctrl_lang_btn = div()
                    .id("pref-btn-win-lang")
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_between()
                    .w(px(210.0))
                    .px(px(12.0))
                    .py(px(5.0))
                    .rounded(px(d.menu_item_radius))
                    .bg(c.dialog_secondary_button_bg)
                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                    .border_1()
                    .border_color(c.dialog_border)
                    .text_size(px(12.0))
                    .text_color(c.text_default)
                    .child("English (en-US)")
                    .child(
                        svg()
                            .path("icon/panel/select-chevron.svg")
                            .size(px(14.0))
                            .text_color(c.dialog_muted),
                    )
                    .into_any_element();

                sec1_items.push(make_row(
                    "Display Language",
                    "Select preferred language for editor UI and dialogs",
                    ctrl_lang_btn,
                ));

                sections.push(make_section("win-sec-theme", "Visual Theme & Language", sec1_items));

                if self.theme_dropdown_open {
                    let mut menu_items = Vec::new();
                    for t_entry in &self.theme_options {
                        let t_id = t_entry.id.clone();
                        let display_label: String = match t_entry.name.as_str() {
                            "Velotype" | "Dark" => "Dark".to_string(),
                            "Velotype Light" | "Light" => "Light".to_string(),
                            other => other.to_string(),
                        };
                        let is_selected = t_id == self.selected_theme_id;
                        let item_ed = cx.entity().downgrade();
                        let item_icon = if display_label == "Light" {
                            "icon/panel/sun.svg"
                        } else {
                            "icon/panel/moon.svg"
                        };

                        menu_items.push(
                            div()
                                .id(ElementId::Name(format!("win-theme-item-{}", t_id).into()))
                                .cursor_pointer()
                                .flex()
                                .items_center()
                                .justify_between()
                                .px(px(10.0))
                                .py(px(6.0))
                                .rounded(px(4.0))
                                .bg(if is_selected { c.dialog_secondary_button_hover } else { c.dialog_surface })
                                .hover(|this| this.bg(c.dialog_secondary_button_hover))
                                .text_size(px(12.0))
                                .text_color(c.text_default)
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.0))
                                        .child(
                                            svg()
                                                .path(item_icon)
                                                .size(px(13.0))
                                                .text_color(c.text_default),
                                        )
                                        .child(display_label),
                                )
                                .child(if is_selected {
                                    svg()
                                        .path("icon/panel/check.svg")
                                        .size(px(13.0))
                                        .text_color(c.dialog_primary_button_bg)
                                        .into_any_element()
                                } else {
                                    div().w(px(13.0)).into_any_element()
                                })
                                .on_click(move |_ev, _win, cx| {
                                    let _ = item_ed.update(cx, |this, cx| {
                                        this.selected_theme_id = t_id.clone();
                                        this.theme_dropdown_open = false;
                                        cx.notify();
                                    });
                                })
                                .into_any_element(),
                        );
                    }

                    active_panel_overlay = Some(
                        div()
                            .absolute()
                            .top(px(80.0))
                            .right(px(26.0))
                            .w(px(210.0))
                            .occlude()
                            .bg(c.dialog_surface)
                            .border_1()
                            .border_color(c.dialog_border)
                            .rounded(px(6.0))
                            .shadow_lg()
                            .p(px(4.0))
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .children(menu_items)
                            .into_any_element(),
                    );
                }

                // Section 2: Status Bar Options
                let mut sec2_items = Vec::new();
                let sb_main_ed = cx.entity().downgrade();
                let ctrl_sb_main = Switch::new("win-switch-sb-main")
                    .checked(self.status_bar_enabled)
                    .on_click(move |_ev, _win, cx| {
                        let _ = sb_main_ed.update(cx, |this, cx| {
                            this.status_bar_enabled = !this.status_bar_enabled;
                            cx.notify();
                        });
                    })
                    .into_any_element();

                sec2_items.push(make_row("Status Bar Visibility", "Show or hide the persistent bottom status bar across window", ctrl_sb_main));

                let sb_words_ed = cx.entity().downgrade();
                let ctrl_sb_words = Switch::new("win-switch-sb-words")
                    .checked(self.status_bar_show_word_count)
                    .on_click(move |_ev, _win, cx| {
                        let _ = sb_words_ed.update(cx, |this, cx| {
                            this.status_bar_show_word_count = !this.status_bar_show_word_count;
                            cx.notify();
                        });
                    })
                    .into_any_element();

                sec2_items.push(make_row("Word Count Badge", "Display real-time document word count in status bar", ctrl_sb_words));

                let sb_pos_ed = cx.entity().downgrade();
                let ctrl_sb_pos = Switch::new("win-switch-sb-pos")
                    .checked(self.status_bar_show_cursor_position)
                    .on_click(move |_ev, _win, cx| {
                        let _ = sb_pos_ed.update(cx, |this, cx| {
                            this.status_bar_show_cursor_position = !this.status_bar_show_cursor_position;
                            cx.notify();
                        });
                    })
                    .into_any_element();

                sec2_items.push(make_row("Cursor Position Badge", "Display line and column coordinates in status bar", ctrl_sb_pos));

                sections.push(make_section("win-sec-sb", "Status Bar Options", sec2_items));
            }
            PreferencesNav::Editing => {
                // Startup and Image Paste
                let mut sec1_items = Vec::new();
                let startup_label = match self.startup_open {
                    StartupOpenPreference::NewFile => "New Blank Document",
                    StartupOpenPreference::LastOpenedFile => "Open Last Opened File",
                };
                let startup_btn_ed = cx.entity().downgrade();
                let ctrl_startup_btn = div()
                    .id("pref-btn-win-startup")
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_between()
                    .w(px(210.0))
                    .px(px(12.0))
                    .py(px(5.0))
                    .rounded(px(d.menu_item_radius))
                    .bg(c.dialog_secondary_button_bg)
                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                    .border_1()
                    .border_color(c.dialog_border)
                    .text_size(px(12.0))
                    .text_color(c.text_default)
                    .child(startup_label)
                    .child(
                        svg()
                            .path("icon/panel/select-chevron.svg")
                            .size(px(14.0))
                            .text_color(c.dialog_muted),
                    )
                    .on_click(move |_ev, _win, cx| {
                        let _ = startup_btn_ed.update(cx, |this, cx| {
                            this.startup_dropdown_open = !this.startup_dropdown_open;
                            cx.notify();
                        });
                    })
                    .into_any_element();

                sec1_items.push(make_row("On Startup", "Choose default document state when launching Velotype editor", ctrl_startup_btn));

                let image_label = match self.image_paste_behavior {
                    ImagePasteBehavior::CopyToAssetsFolder => "Save to Local Assets",
                    ImagePasteBehavior::CopyToDocumentFolder => "Copy to Document Folder",
                    ImagePasteBehavior::CopyToNamedAssetsFolder => "Insert Direct Link",
                    ImagePasteBehavior::None => "None",
                };
                let image_btn_ed = cx.entity().downgrade();
                let ctrl_image_btn = div()
                    .id("pref-btn-win-image")
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_between()
                    .w(px(210.0))
                    .px(px(12.0))
                    .py(px(5.0))
                    .rounded(px(d.menu_item_radius))
                    .bg(c.dialog_secondary_button_bg)
                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                    .border_1()
                    .border_color(c.dialog_border)
                    .text_size(px(12.0))
                    .text_color(c.text_default)
                    .child(image_label)
                    .child(
                        svg()
                            .path("icon/panel/select-chevron.svg")
                            .size(px(14.0))
                            .text_color(c.dialog_muted),
                    )
                    .on_click(move |_ev, _win, cx| {
                        let _ = image_btn_ed.update(cx, |this, cx| {
                            this.image_dropdown_open = !this.image_dropdown_open;
                            cx.notify();
                        });
                    })
                    .into_any_element();

                sec1_items.push(make_row("Image Paste Action", "Default storage location when pasting images into document", ctrl_image_btn));

                sections.push(make_section("win-sec-editing", "Editor & File Preferences", sec1_items));

                if self.startup_dropdown_open {
                    let startup_opts = [
                        (StartupOpenPreference::NewFile, "New Blank Document"),
                        (StartupOpenPreference::LastOpenedFile, "Open Last Opened File"),
                    ];
                    let mut menu_items = Vec::new();
                    for (pref, label) in startup_opts {
                        let is_selected = pref == self.startup_open;
                        let item_ed = cx.entity().downgrade();

                        menu_items.push(
                            div()
                                .id(ElementId::Name(format!("win-startup-item-{:?}", pref).into()))
                                .cursor_pointer()
                                .flex()
                                .items_center()
                                .justify_between()
                                .px(px(10.0))
                                .py(px(6.0))
                                .rounded(px(4.0))
                                .bg(if is_selected { c.dialog_secondary_button_hover } else { c.dialog_surface })
                                .hover(|this| this.bg(c.dialog_secondary_button_hover))
                                .text_size(px(12.0))
                                .text_color(c.text_default)
                                .child(label)
                                .child(if is_selected {
                                    svg()
                                        .path("icon/panel/check.svg")
                                        .size(px(13.0))
                                        .text_color(c.dialog_primary_button_bg)
                                        .into_any_element()
                                } else {
                                    div().w(px(13.0)).into_any_element()
                                })
                                .on_click(move |_ev, _win, cx| {
                                    let _ = item_ed.update(cx, |this, cx| {
                                        this.startup_open = pref;
                                        this.startup_dropdown_open = false;
                                        cx.notify();
                                    });
                                })
                                .into_any_element(),
                        );
                    }

                    active_panel_overlay = Some(
                        div()
                            .absolute()
                            .top(px(80.0))
                            .right(px(26.0))
                            .w(px(210.0))
                            .occlude()
                            .bg(c.dialog_surface)
                            .border_1()
                            .border_color(c.dialog_border)
                            .rounded(px(6.0))
                            .shadow_lg()
                            .p(px(4.0))
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .children(menu_items)
                            .into_any_element(),
                    );
                } else if self.image_dropdown_open {
                    let image_opts = [
                        (ImagePasteBehavior::CopyToAssetsFolder, "Save to Local Assets"),
                        (ImagePasteBehavior::CopyToDocumentFolder, "Copy to Document Folder"),
                        (ImagePasteBehavior::CopyToNamedAssetsFolder, "Insert Direct Link"),
                    ];
                    let mut menu_items = Vec::new();
                    for (pref, label) in image_opts {
                        let is_selected = pref == self.image_paste_behavior;
                        let item_ed = cx.entity().downgrade();

                        menu_items.push(
                            div()
                                .id(ElementId::Name(format!("win-image-item-{:?}", pref).into()))
                                .cursor_pointer()
                                .flex()
                                .items_center()
                                .justify_between()
                                .px(px(10.0))
                                .py(px(6.0))
                                .rounded(px(4.0))
                                .bg(if is_selected { c.dialog_secondary_button_hover } else { c.dialog_surface })
                                .hover(|this| this.bg(c.dialog_secondary_button_hover))
                                .text_size(px(12.0))
                                .text_color(c.text_default)
                                .child(label)
                                .child(if is_selected {
                                    svg()
                                        .path("icon/panel/check.svg")
                                        .size(px(13.0))
                                        .text_color(c.dialog_primary_button_bg)
                                        .into_any_element()
                                } else {
                                    div().w(px(13.0)).into_any_element()
                                })
                                .on_click(move |_ev, _win, cx| {
                                    let _ = item_ed.update(cx, |this, cx| {
                                        this.image_paste_behavior = pref;
                                        this.image_dropdown_open = false;
                                        cx.notify();
                                    });
                                })
                                .into_any_element(),
                        );
                    }

                    active_panel_overlay = Some(
                        div()
                            .absolute()
                            .top(px(144.0))
                            .right(px(26.0))
                            .w(px(210.0))
                            .occlude()
                            .bg(c.dialog_surface)
                            .border_1()
                            .border_color(c.dialog_border)
                            .rounded(px(6.0))
                            .shadow_lg()
                            .p(px(4.0))
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .children(menu_items)
                            .into_any_element(),
                    );
                }
            }
            PreferencesNav::Keymap => {
                let mut sec1_items = Vec::new();
                let doc_shortcuts = [
                    ("Save Document", "Save active file changes to disk", "Ctrl + S"),
                    ("Save Document As", "Save active document with a new name", "Ctrl + Shift + S"),
                    ("New Window", "Open a new editor window instance", "Ctrl + N"),
                    ("Close Window", "Close the currently focused editor window", "Ctrl + W"),
                    ("Toggle View Mode", "Switch between Edit, Preview, and Dual view layouts", "Ctrl + M"),
                ];

                for (name, desc, sc) in doc_shortcuts.iter() {
                    let ctrl_sc = div()
                        .px(px(8.0))
                        .py(px(2.0))
                        .rounded(px(d.menu_item_radius))
                        .bg(c.dialog_secondary_button_hover)
                        .text_size(px(11.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(c.text_default)
                        .child(*sc)
                        .into_any_element();

                    sec1_items.push(make_row(*name, *desc, ctrl_sc));
                }

                sections.push(make_section("win-sec-keymap", "Document & Keymap Shortcuts", sec1_items));
            }
        }

        let mut right_content = div()
            .id("win-pref-right-content")
            .relative()
            .flex_1()
            .h_full()
            .p(px(14.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .children(sections);

        if let Some(ol) = active_panel_overlay {
            right_content = right_content.child(ol);
        }

        // Bottom Action Bar (Cancel / Save)
        let bottom_bar = div()
            .w_full()
            .px(px(14.0))
            .py(px(10.0))
            .border_t_1()
            .border_color(c.dialog_border)
            .flex()
            .items_center()
            .justify_end()
            .gap(px(d.dialog_button_gap))
            .child(
                div()
                    .id("preferences-cancel")
                    .h(px(d.dialog_button_height))
                    .px(px(d.dialog_button_padding_x))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px((d.dialog_radius - 4.0).max(0.0)))
                    .border(px(d.dialog_border_width))
                    .border_color(c.dialog_border)
                    .bg(c.dialog_secondary_button_bg)
                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                    .cursor_pointer()
                    .text_size(px(t.dialog_button_size))
                    .font_weight(t.dialog_button_weight.to_font_weight())
                    .text_color(c.dialog_secondary_button_text)
                    .child(strings.preferences_cancel.clone())
                    .on_click(cx.listener(Self::cancel)),
            )
            .child(
                div()
                    .id("preferences-save")
                    .h(px(d.dialog_button_height))
                    .px(px(d.dialog_button_padding_x))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px((d.dialog_radius - 4.0).max(0.0)))
                    .border(px(if can_save { 0.0 } else { d.dialog_border_width }))
                    .border_color(c.dialog_border)
                    .bg(if can_save {
                        c.dialog_primary_button_bg
                    } else {
                        c.dialog_secondary_button_bg
                    })
                    .hover(move |this| {
                        if can_save {
                            this.bg(c.dialog_primary_button_hover)
                        } else {
                            this.bg(c.dialog_secondary_button_bg)
                        }
                    })
                    .when(can_save, |this| this.cursor_pointer())
                    .text_size(px(t.dialog_button_size))
                    .font_weight(t.dialog_button_weight.to_font_weight())
                    .text_color(if can_save {
                        c.dialog_primary_button_text
                    } else {
                        c.dialog_secondary_button_text
                    })
                    .child(strings.preferences_save.clone())
                    .on_click(cx.listener(Self::save)),
            );

        let main_body = div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_row()
            .child(left_nav)
            .child(right_content);

        let content = div()
            .size_full()
            .pt(px(titlebar_height))
            .flex()
            .flex_col()
            .key_context("Preferences")
            .track_focus(&self.focus_handle)
            .bg(c.editor_background)
            .text_color(c.dialog_body)
            .child(main_body)
            .child(bottom_bar);

        let root = div()
            .size_full()
            .relative()
            .bg(c.editor_background)
            .child(content);

        if let Some(titlebar) = render_custom_titlebar(
            "preferences-titlebar",
            window_title,
            None,
            &theme,
            window,
            cx,
            Self::on_titlebar_close,
        ) {
            root.child(titlebar)
        } else {
            root
        }
    }
}

fn open_preferences_window_with_state(
    cx: &mut App,
    preferences: AppPreferences,
    theme_options: Vec<ThemeCatalogEntry>,
    title: String,
) -> WindowHandle<PreferencesWindow> {
    let bounds = Bounds::centered(None, size(px(720.0), px(480.0)), cx);
    let window_title = SharedString::from(format!("Velotype - {title}"));
    let handle = cx
        .open_window(
            velotype_window_options(window_title, bounds),
            move |_window, cx| {
                cx.new(move |cx| PreferencesWindow::new(preferences, theme_options, cx))
            },
        )
        .expect("preferences window should open");

    handle
        .update(cx, |preferences, window, _cx| {
            window.activate_window();
            preferences.focus_handle.focus(window);
        })
        .expect("newly opened preferences window should be updateable");

    handle
}

pub(crate) fn open_preferences_window(cx: &mut App) -> WindowHandle<PreferencesWindow> {
    let preferences = match read_app_preferences() {
        Ok(preferences) => preferences,
        Err(err) => {
            eprintln!("failed to read app preferences: {err}");
            AppPreferences::default()
        }
    };
    let theme_options = cx.global::<ThemeManager>().available_themes().to_vec();
    let title = cx
        .global::<I18nManager>()
        .strings()
        .preferences_window_title
        .clone();
    open_preferences_window_with_state(cx, preferences, theme_options, title)
}

#[cfg(test)]
mod tests {
    use super::{
        AppPreferences, EditorSettings, ImagePasteBehavior, StartupOpenPreference,
        StatusBarPreferences, load_or_create_app_preferences_with_dirs_and_locales,
        open_preferences_window_with_state, read_app_preferences_with_dirs,
        save_app_preferences_with_dirs, save_preferences_from_window_with_dirs,
    };
    use crate::config::VelotypeConfigDirs;
    use crate::i18n::I18nManager;
    use crate::theme::{ThemeCatalogEntry, ThemeManager};
    use gpui::TestAppContext;
    use std::collections::BTreeMap;

    fn init_preferences_test_app(cx: &mut TestAppContext) {
        cx.update(|cx| {
            I18nManager::init_with_language_id(cx, "en-US");
            ThemeManager::init_with_theme_id(cx, "velotype");
            crate::components::init(cx);
            EditorSettings::init(cx, true);
        });
    }

    fn default_theme_options() -> Vec<ThemeCatalogEntry> {
        vec![ThemeCatalogEntry {
            id: "velotype".into(),
            name: "Velotype".into(),
        }]
    }

    #[test]
    fn missing_preferences_file_returns_defaults() {
        let root = std::env::temp_dir().join(format!(
            "velotype-preferences-missing-{}",
            uuid::Uuid::new_v4()
        ));
        let dirs = VelotypeConfigDirs::from_root(&root);
        let preferences =
            read_app_preferences_with_dirs(&dirs).expect("missing preferences should load");
        assert_eq!(preferences, AppPreferences::default());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn partial_or_invalid_preferences_fall_back_by_field() {
        let root = std::env::temp_dir().join(format!(
            "velotype-preferences-partial-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).expect("temp root should exist");
        let dirs = VelotypeConfigDirs::from_root(&root);
        std::fs::write(
            dirs.app_config_file(),
            r#"
                [startup]
                open = "not-valid"

                [theme]
                default_theme_id = "velotype-light"
            "#,
        )
        .expect("preferences should be written");

        let preferences =
            read_app_preferences_with_dirs(&dirs).expect("partial preferences should load");
        assert_eq!(preferences.startup_open, StartupOpenPreference::NewFile);
        assert_eq!(preferences.default_language_id, "en-US");
        assert_eq!(preferences.default_theme_id, "velotype-light");
        assert_eq!(preferences.image_paste_behavior, ImagePasteBehavior::None);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn invalid_image_paste_behavior_falls_back_to_none() {
        let root = std::env::temp_dir().join(format!(
            "velotype-preferences-image-invalid-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).expect("temp root should exist");
        let dirs = VelotypeConfigDirs::from_root(&root);
        std::fs::write(
            dirs.app_config_file(),
            r#"
                [editor]
                image_paste_behavior = "somewhere-dangerous"
            "#,
        )
        .expect("preferences should be written");

        let preferences = read_app_preferences_with_dirs(&dirs).expect("preferences should load");
        assert_eq!(preferences.image_paste_behavior, ImagePasteBehavior::None);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn damaged_preferences_file_returns_defaults() {
        let root = std::env::temp_dir().join(format!(
            "velotype-preferences-damaged-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).expect("temp root should exist");
        let dirs = VelotypeConfigDirs::from_root(&root);
        std::fs::write(dirs.app_config_file(), "not = [valid")
            .expect("preferences should be written");

        let preferences =
            read_app_preferences_with_dirs(&dirs).expect("damaged preferences should load");
        assert_eq!(preferences, AppPreferences::default());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn saves_and_reads_preferences() {
        let root = std::env::temp_dir().join(format!(
            "velotype-preferences-save-{}",
            uuid::Uuid::new_v4()
        ));
        let dirs = VelotypeConfigDirs::from_root(&root);
        let preferences = AppPreferences {
            startup_open: StartupOpenPreference::LastOpenedFile,
            default_language_id: "zh-CN".into(),
            default_theme_id: "velotype-light".into(),
            show_table_headers: false,
            image_paste_behavior: ImagePasteBehavior::CopyToAssetsFolder,
            keybindings: BTreeMap::new(),
            status_bar: StatusBarPreferences::default(),
        };

        save_app_preferences_with_dirs(&preferences, &dirs)
            .expect("preferences should save to config.toml");
        let loaded = read_app_preferences_with_dirs(&dirs).expect("preferences should read back");
        assert_eq!(loaded, preferences);

        let text =
            std::fs::read_to_string(dirs.app_config_file()).expect("config.toml should exist");
        assert!(text.contains("open = \"last_opened_file\""));
        assert!(text.contains("default_language_id = \"zh-CN\""));
        assert!(text.contains("default_theme_id = \"velotype-light\""));
        assert!(text.contains("show_table_headers = false"));
        assert!(text.contains("image_paste_behavior = \"copy_to_assets_folder\""));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn missing_preferences_file_is_created_with_detected_language() {
        let root = std::env::temp_dir().join(format!(
            "velotype-preferences-create-{}",
            uuid::Uuid::new_v4()
        ));
        let dirs = VelotypeConfigDirs::from_root(&root);
        let preferences = load_or_create_app_preferences_with_dirs_and_locales(&dirs, ["zh-HK"])
            .expect("preferences should be created");
        assert_eq!(preferences.default_language_id, "zh-CN");
        assert!(dirs.app_config_file().exists());
        let text =
            std::fs::read_to_string(dirs.app_config_file()).expect("config.toml should exist");
        assert!(text.contains("[language]"));
        assert!(text.contains("default_language_id = \"zh-CN\""));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_preferences_are_normalized_with_language() {
        let root = std::env::temp_dir().join(format!(
            "velotype-preferences-legacy-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).expect("temp root should exist");
        let dirs = VelotypeConfigDirs::from_root(&root);
        std::fs::write(
            dirs.app_config_file(),
            r#"
                [startup]
                open = "last_opened_file"

                [theme]
                default_theme_id = "velotype-light"
            "#,
        )
        .expect("legacy preferences should be written");

        let preferences = load_or_create_app_preferences_with_dirs_and_locales(&dirs, ["en-GB"])
            .expect("legacy preferences should normalize");
        assert_eq!(
            preferences.startup_open,
            StartupOpenPreference::LastOpenedFile
        );
        assert_eq!(preferences.default_language_id, "en-US");
        assert_eq!(preferences.default_theme_id, "velotype-light");
        let text =
            std::fs::read_to_string(dirs.app_config_file()).expect("config.toml should exist");
        assert!(text.contains("[language]"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn saving_preferences_window_preserves_language() {
        let root = std::env::temp_dir().join(format!(
            "velotype-preferences-window-{}",
            uuid::Uuid::new_v4()
        ));
        let dirs = VelotypeConfigDirs::from_root(&root);
        let preferences = AppPreferences {
            startup_open: StartupOpenPreference::NewFile,
            default_language_id: "zh-CN".into(),
            default_theme_id: "velotype".into(),
            show_table_headers: true,
            image_paste_behavior: ImagePasteBehavior::None,
            keybindings: BTreeMap::new(),
            status_bar: StatusBarPreferences::default(),
        };
        save_app_preferences_with_dirs(&preferences, &dirs)
            .expect("preferences should save to config.toml");

        let saved = save_preferences_from_window_with_dirs(
            StartupOpenPreference::LastOpenedFile,
            "velotype-light",
            ImagePasteBehavior::CopyToNamedAssetsFolder,
            BTreeMap::from([("save_document".to_string(), vec!["ctrl-alt-s".to_string()])]),
            &StatusBarPreferences::default(),
            &dirs,
        )
        .expect("window preferences should save");
        assert_eq!(saved.default_language_id, "zh-CN");
        assert_eq!(saved.startup_open, StartupOpenPreference::LastOpenedFile);
        assert_eq!(saved.default_theme_id, "velotype-light");
        assert_eq!(
            saved.image_paste_behavior,
            ImagePasteBehavior::CopyToNamedAssetsFolder
        );
        assert_eq!(
            saved.keybindings.get("save_document"),
            Some(&vec!["ctrl-alt-s".to_string()])
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[gpui::test]
    async fn preferences_window_activates_and_focuses_on_open(cx: &mut TestAppContext) {
        init_preferences_test_app(cx);

        let handle = cx.update(|cx| {
            open_preferences_window_with_state(
                cx,
                AppPreferences::default(),
                default_theme_options(),
                "Preferences".into(),
            )
        });
        cx.run_until_parked();

        let active_window = cx.update(|cx| cx.active_window().expect("window should be active"));
        assert_eq!(active_window.window_id(), handle.window_id());
        assert!(
            handle
                .update(cx, |preferences, window, _cx| preferences
                    .focus_handle
                    .is_focused(window))
                .expect("preferences window should be updateable")
        );
        assert!(
            !handle
                .update(cx, |preferences, _window, _cx| preferences
                    .has_unsaved_changes())
                .expect("preferences window should be updateable")
        );
    }

    #[gpui::test]
    async fn preferences_dirty_state_tracks_draft_changes(cx: &mut TestAppContext) {
        init_preferences_test_app(cx);

        let handle = cx.update(|cx| {
            open_preferences_window_with_state(
                cx,
                AppPreferences::default(),
                default_theme_options(),
                "Preferences".into(),
            )
        });
        cx.run_until_parked();

        handle
            .update(cx, |preferences, _window, _cx| {
                assert!(!preferences.has_unsaved_changes());
                preferences.startup_open = StartupOpenPreference::LastOpenedFile;
                assert!(preferences.has_unsaved_changes());
                preferences.startup_open = StartupOpenPreference::NewFile;
                assert!(!preferences.has_unsaved_changes());

                preferences.image_paste_behavior = ImagePasteBehavior::CopyToAssetsFolder;
                assert!(preferences.has_unsaved_changes());
                preferences.image_paste_behavior = ImagePasteBehavior::None;
                assert!(!preferences.has_unsaved_changes());

                preferences
                    .keybindings
                    .insert("save_document".into(), vec!["ctrl-alt-s".into()]);
                assert!(preferences.has_unsaved_changes());
            })
            .expect("preferences window should be updateable");
    }

    #[gpui::test]
    async fn applying_saved_preferences_keeps_window_open_and_focused(cx: &mut TestAppContext) {
        init_preferences_test_app(cx);

        let handle = cx.update(|cx| {
            open_preferences_window_with_state(
                cx,
                AppPreferences::default(),
                default_theme_options(),
                "Preferences".into(),
            )
        });
        cx.run_until_parked();

        handle
            .update(cx, |preferences, window, cx| {
                preferences.startup_open = StartupOpenPreference::LastOpenedFile;
                assert!(preferences.has_unsaved_changes());
                let saved = AppPreferences {
                    startup_open: StartupOpenPreference::LastOpenedFile,
                    ..AppPreferences::default()
                };
                preferences.apply_saved_preferences(saved, window, cx);
            })
            .expect("preferences window should be updateable");
        cx.run_until_parked();

        assert_eq!(cx.update(|cx| cx.windows().len()), 1);
        let active_window = cx.update(|cx| cx.active_window().expect("window should be active"));
        assert_eq!(active_window.window_id(), handle.window_id());
        assert!(
            handle
                .update(cx, |preferences, window, _cx| preferences
                    .focus_handle
                    .is_focused(window))
                .expect("preferences window should remain updateable")
        );
        assert!(
            !handle
                .update(cx, |preferences, _window, _cx| preferences
                    .has_unsaved_changes())
                .expect("preferences window should remain updateable")
        );
    }
}
