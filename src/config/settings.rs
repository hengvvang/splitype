//! Persistent app settings and the settings window.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::Context as _;
use gpui::*;
use serde::{Deserialize, Serialize};

use super::{VelotypeConfigDirs, read_recent_files};
use crate::components::{
    ShortcutCommand, install_keybindings, normalize_shortcut_config, switch::Switch,
};
use crate::i18n::{I18nManager, language_id_for_locale_settings};
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
        }
    }
}

/// Runtime-accessible editor settings mirrored from [`AppSettings`] so the
/// render path can read them without touching disk. Toggling persists the new
/// value back to the settings file.
pub struct EditorSettings {
    show_table_headers: bool,
    status_bar_settings: StatusBarSettings,
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
        }
    }
}

pub(crate) fn read_app_settings() -> anyhow::Result<AppSettings> {
    read_app_settings_with_dirs(&VelotypeConfigDirs::from_system()?)
}

pub(crate) fn read_app_settings_with_dirs(
    dirs: &VelotypeConfigDirs,
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
    let dirs = VelotypeConfigDirs::from_system()?;
    load_or_create_app_settings_with_dirs_and_locales(&dirs, sys_locale::get_locales())
}

fn app_settings_from_toml_value(
    value: &toml::Value,
    fallback_language_id: &str,
) -> AppSettings {
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

    AppSettings {
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
    language_id_for_locale_settings(locales)
}

fn load_or_create_app_settings_with_dirs_and_locales<I, S>(
    dirs: &VelotypeConfigDirs,
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
    save_app_settings_with_dirs(settings, &VelotypeConfigDirs::from_system()?)
}

pub(crate) fn save_app_settings_with_dirs(
    settings: &AppSettings,
    dirs: &VelotypeConfigDirs,
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
    let dirs = VelotypeConfigDirs::from_system()?;
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
    dirs: &VelotypeConfigDirs,
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

fn update_app_settings(
    update: impl FnOnce(&mut AppSettings),
) -> anyhow::Result<AppSettings> {
    let mut settings = load_or_create_app_settings()?;
    update(&mut settings);
    save_app_settings(&settings)?;
    Ok(settings)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SettingsNav {
    Interface,
    Editing,
    Keymap,
}

/// Independent settings window view.
pub(crate) struct SettingsWindow {
    nav: SettingsNav,
    startup_open: StartupOpenSetting,
    selected_theme_id: String,
    image_paste_behavior: ImagePasteBehavior,
    keybindings: BTreeMap<String, Vec<String>>,
    saved_startup_open: StartupOpenSetting,
    saved_theme_id: String,
    saved_image_paste_behavior: ImagePasteBehavior,
    saved_keybindings: BTreeMap<String, Vec<String>>,
    theme_options: Vec<ThemeCatalogEntry>,
    focus_handle: FocusHandle,
    startup_dropdown_open: bool,
    theme_dropdown_open: bool,
    lang_dropdown_open: bool,
    image_dropdown_open: bool,
    font_size: u32,
    line_height: f32,
    editing_stepper: Option<String>,
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
    expanded_sections: std::collections::HashSet<String>,
}

impl SettingsWindow {
    fn new(
        settings: AppSettings,
        theme_options: Vec<ThemeCatalogEntry>,
        cx: &mut Context<Self>,
    ) -> Self {
        let selected_theme_id = if theme_options
            .iter()
            .any(|entry| entry.id == settings.default_theme_id)
        {
            settings.default_theme_id
        } else {
            DEFAULT_THEME_ID.into()
        };
        let startup_open = settings.startup_open;
        let image_paste_behavior = settings.image_paste_behavior;
        let keybindings = settings.keybindings;

        let mut expanded_sections = std::collections::HashSet::new();
        expanded_sections.insert("theme".to_string());
        expanded_sections.insert("status_bar".to_string());
        expanded_sections.insert("typography".to_string());
        expanded_sections.insert("markdown".to_string());
        expanded_sections.insert("startup".to_string());
        expanded_sections.insert("doc_actions".to_string());
        expanded_sections.insert("view_controls".to_string());
        expanded_sections.insert("keymap".to_string());

        Self {
            nav: SettingsNav::Interface,
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
            lang_dropdown_open: false,
            image_dropdown_open: false,
            font_size: 14,
            line_height: 1.6,
            editing_stepper: None,
            recording_shortcut: None,
            shortcut_error: None,
            status_bar_enabled: settings.status_bar.enabled,
            status_bar_show_word_count: settings.status_bar.show_word_count,
            status_bar_show_cursor_position: settings.status_bar.show_cursor_position,
            status_bar_show_sidebar_toggle: settings.status_bar.show_sidebar_toggle,
            status_bar_show_mode_switch: settings.status_bar.show_mode_switch,
            saved_status_bar_enabled: settings.status_bar.enabled,
            saved_status_bar_show_word_count: settings.status_bar.show_word_count,
            saved_status_bar_show_cursor_position: settings.status_bar.show_cursor_position,
            saved_status_bar_show_sidebar_toggle: settings.status_bar.show_sidebar_toggle,
            saved_status_bar_show_mode_switch: settings.status_bar.show_mode_switch,
            expanded_sections,
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

    #[allow(dead_code)]
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

        let settings = match save_settings_from_window(
            self.startup_open,
            &self.selected_theme_id,
            self.image_paste_behavior,
            self.keybindings.clone(),
            &StatusBarSettings {
                enabled: self.status_bar_enabled,
                show_word_count: self.status_bar_show_word_count,
                show_cursor_position: self.status_bar_show_cursor_position,
                show_sidebar_toggle: self.status_bar_show_sidebar_toggle,
                show_mode_switch: self.status_bar_show_mode_switch,
                custom_buttons: Vec::new(),
            },
        ) {
            Ok(settings) => settings,
            Err(err) => {
                let strings = cx.global::<I18nManager>().strings().clone();
                let ok = strings.info_dialog_ok;
                let buttons = [ok.as_str()];
                let _ = window.prompt(
                    PromptLevel::Critical,
                    &strings.settings_save_failed_title,
                    Some(&err.to_string()),
                    &buttons,
                    cx,
                );
                return;
            }
        };

        self.apply_saved_settings(settings, window, cx);
    }

    fn apply_saved_settings(
        &mut self,
        settings: AppSettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let theme_changed = cx.update_global::<ThemeManager, _>(|theme_manager, _cx| {
            theme_manager.set_theme_by_id(&settings.default_theme_id)
        });
        if !theme_changed {
            let _ = cx.update_global::<ThemeManager, _>(|theme_manager, _cx| {
                theme_manager.set_theme_by_id(DEFAULT_THEME_ID)
            });
        }
        cx.clear_key_bindings();
        install_keybindings(cx, &settings.keybindings);
        crate::app_menu::install_menus(cx);
        cx.update_global::<EditorSettings, _>(|ed_settings, _cx| {
            ed_settings.status_bar_settings.enabled = settings.status_bar.enabled;
            ed_settings.status_bar_settings.show_word_count =
                settings.status_bar.show_word_count;
            ed_settings.status_bar_settings.show_cursor_position =
                settings.status_bar.show_cursor_position;
            ed_settings.status_bar_settings.show_sidebar_toggle =
                settings.status_bar.show_sidebar_toggle;
            ed_settings.status_bar_settings.show_mode_switch =
                settings.status_bar.show_mode_switch;
        });

        let _strings = cx.global::<I18nManager>().strings().clone();
        let _ = apply_configured_language(cx, &settings.default_language_id);

        self.saved_startup_open = settings.startup_open;
        self.saved_theme_id = settings.default_theme_id.clone();
        self.saved_image_paste_behavior = settings.image_paste_behavior;
        self.saved_keybindings = settings.keybindings;
        self.saved_status_bar_enabled = settings.status_bar.enabled;
        self.saved_status_bar_show_word_count = settings.status_bar.show_word_count;
        self.saved_status_bar_show_cursor_position = settings.status_bar.show_cursor_position;
        self.saved_status_bar_show_sidebar_toggle = settings.status_bar.show_sidebar_toggle;
        self.saved_status_bar_show_mode_switch = settings.status_bar.show_mode_switch;

        self.selected_theme_id = settings.default_theme_id;
        self.startup_open = settings.startup_open;
        self.image_paste_behavior = settings.image_paste_behavior;

        window.refresh();
        cx.refresh_windows();
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeManager>().current().clone();
        let strings = cx.global::<I18nManager>().strings().clone();
        let c = &theme.colors;
        let d = &theme.dimensions;
        let window_title = SharedString::from(strings.settings_window_title.clone());
        window.set_window_title(window_title.as_ref());
        let titlebar_height = custom_titlebar_height(window, d);

        let mut inner_border_color = c.dialog_border;
        inner_border_color.a *= 0.4;

        // Left Sidebar Navigation
        let nav_item = |id: &'static str, label: &'static str, is_selected: bool| -> AnyElement {
            div()
                .id(id)
                .cursor_pointer()
                .w_full()
                .px(px(12.0))
                .py(px(8.0))
                .rounded(px(d.menu_item_radius))
                .bg(if is_selected {
                    c.dialog_secondary_button_hover
                } else {
                    c.dialog_surface
                })
                .hover(|this| this.bg(c.dialog_secondary_button_hover))
                .active(|this| this.opacity(0.92))
                .text_size(px(d.menu_text_size))
                .font_weight(if is_selected {
                    gpui::FontWeight::BOLD
                } else {
                    gpui::FontWeight::NORMAL
                })
                .text_color(if is_selected {
                    c.text_default
                } else {
                    c.dialog_muted
                })
                .child(label)
                .into_any_element()
        };

        let nav_interface = div().id("win-nav-wrap-1").w_full().child(nav_item("nav-interface", "Interface", self.nav == SettingsNav::Interface)).on_click({
            let ed = cx.entity().downgrade();
            move |_ev, _win, cx| {
                let _ = ed.update(cx, |this, cx| {
                    this.nav = SettingsNav::Interface;
                    cx.notify();
                });
            }
        });

        let nav_editing = div().id("win-nav-wrap-2").w_full().child(nav_item("nav-editing", "Editing", self.nav == SettingsNav::Editing)).on_click({
            let ed = cx.entity().downgrade();
            move |_ev, _win, cx| {
                let _ = ed.update(cx, |this, cx| {
                    this.nav = SettingsNav::Editing;
                    cx.notify();
                });
            }
        });

        let nav_keymap = div().id("win-nav-wrap-3").w_full().child(nav_item("nav-keymap", "Keymap", self.nav == SettingsNav::Keymap)).on_click({
            let ed = cx.entity().downgrade();
            move |_ev, _win, cx| {
                let _ = ed.update(cx, |this, cx| {
                    this.nav = SettingsNav::Keymap;
                    cx.notify();
                });
            }
        });

        let left_nav = div()
            .id("win-pref-left-nav")
            .w(px(160.0))
            .h_full()
            .flex_shrink_0()
            .p(px(8.0))
            .border_r_1()
            .border_color(c.dialog_border)
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(nav_interface)
            .child(nav_editing)
            .child(nav_keymap);

        // Helper closures for Sections and Rows
        let make_row = |title: &'static str, desc: &'static str, control: AnyElement| -> AnyElement {
            div()
                .w_full()
                .h(px(56.0))
                .px(px(16.0))
                .rounded(px(d.menu_panel_radius))
                .bg(c.dialog_surface)
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
                .child(control)
                .into_any_element()
        };

        let render_zed_stepper = |id_dec: &'static str,
                                  id_inc: &'static str,
                                  val_str: String,
                                  is_editing: bool,
                                  on_dec: Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
                                  on_inc: Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
                                  on_click_center: Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>|
         -> AnyElement {
            div()
                .flex()
                .items_center()
                .h(px(28.0))
                .rounded(px(d.menu_item_radius))
                .border_1()
                .border_color(c.dialog_border)
                .bg(c.dialog_secondary_button_bg)
                .child(
                    div()
                        .id(id_dec)
                        .cursor_pointer()
                        .h_full()
                        .px(px(10.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .hover(|this| this.bg(c.dialog_secondary_button_hover))
                        .text_size(px(13.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(c.text_default)
                        .child("-")
                        .on_click(on_dec),
                )
                .child(div().w(px(1.0)).h_full().bg(c.dialog_border))
                .child(
                    div()
                        .id(ElementId::Name(format!("{}-center", id_dec).into()))
                        .cursor_pointer()
                        .h_full()
                        .min_w(px(56.0))
                        .px(px(8.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap(px(2.0))
                        .bg(if is_editing { c.dialog_surface } else { c.dialog_secondary_button_bg })
                        .border_1()
                        .border_color(if is_editing { c.dialog_primary_button_bg } else { c.dialog_border })
                        .text_size(px(12.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(c.text_default)
                        .child(val_str)
                        .child(if is_editing {
                            div().w(px(1.5)).h(px(12.0)).bg(c.dialog_primary_button_bg).into_any_element()
                        } else {
                            div().into_any_element()
                        })
                        .on_click(on_click_center),
                )
                .child(div().w(px(1.0)).h_full().bg(c.dialog_border))
                .child(
                    div()
                        .id(id_inc)
                        .cursor_pointer()
                        .h_full()
                        .px(px(10.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .hover(|this| this.bg(c.dialog_secondary_button_hover))
                        .text_size(px(13.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(c.text_default)
                        .child("+")
                        .on_click(on_inc),
                )
                .into_any_element()
        };

        let is_section_expanded = |key: &str| self.expanded_sections.contains(key);
        let toggle_section_ed = cx.entity().downgrade();

        let make_section = move |id: &'static str,
                                key: &'static str,
                                title: &'static str,
                                items: Vec<AnyElement>|
              -> AnyElement {
            let expanded = is_section_expanded(key);
            let toggle_ed = toggle_section_ed.clone();

            let header = div()
                .id(ElementId::Name(format!("{}-header", id).into()))
                .w_full()
                .px(px(14.0))
                .py(px(10.0))
                .cursor_pointer()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    svg()
                        .path(if expanded {
                            "icon/panel/chevron-down.svg"
                        } else {
                            "icon/panel/chevron-right.svg"
                        })
                        .size(px(14.0))
                        .text_color(c.text_default),
                )
                .child(
                    div()
                        .text_size(px(13.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(c.text_default)
                        .child(title),
                )
                .on_click(move |_ev, _win, cx| {
                    let _ = toggle_ed.update(cx, |this, cx| {
                        if this.expanded_sections.contains(key) {
                            this.expanded_sections.remove(key);
                        } else {
                            this.expanded_sections.insert(key.to_string());
                        }
                        cx.notify();
                    });
                });

            let mut card = div()
                .id(id)
                .relative()
                .w_full()
                .rounded(px(d.menu_panel_radius))
                .bg(c.dialog_surface)
                .border_1()
                .border_color(c.dialog_border)
                .flex()
                .flex_col()
                .child(header);

            if expanded && !items.is_empty() {
                let body = div()
                    .w_full()
                    .px(px(10.0))
                    .pb(px(10.0))
                    .pt(px(2.0))
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .children(items);

                card = card.child(body);
            }

            card.into_any_element()
        };

        let mut sections: Vec<AnyElement> = Vec::new();
        let mut active_panel_overlay: Option<AnyElement> = None;

        match self.nav {
            SettingsNav::Interface => {
                // Section 1: Visual Theme & Language
                let sec1_key = "theme";
                let _is_sec1_expanded = self.expanded_sections.contains(sec1_key);
                let mut sec1_items = Vec::new();

                let selected_theme_label = self.selected_theme_name();
                let theme_btn_ed = cx.entity().downgrade();
                let theme_icon_path = if selected_theme_label == "Light" {
                    "icon/panel/sun.svg"
                } else {
                    "icon/panel/moon.svg"
                };

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
                            .child(selected_theme_label),
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
                            this.lang_dropdown_open = false;
                            this.startup_dropdown_open = false;
                            this.image_dropdown_open = false;
                            cx.notify();
                        });
                    })
                    .into_any_element();

                sec1_items.push(make_row("Interface Theme", "Customize overall application color scheme and appearance", ctrl_theme_btn));

                let cur_lang = cx.global::<I18nManager>().current_language_id();
                let lang_display = if cur_lang == "zh-CN" { "简体中文 (zh-CN)" } else { "English (en-US)" };
                let lang_btn_ed = cx.entity().downgrade();
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
                    .child(lang_display)
                    .child(
                        svg()
                            .path("icon/panel/select-chevron.svg")
                            .size(px(14.0))
                            .text_color(c.dialog_muted),
                    )
                    .on_click(move |_ev, _win, cx| {
                        let _ = lang_btn_ed.update(cx, |this, cx| {
                            this.lang_dropdown_open = !this.lang_dropdown_open;
                            this.theme_dropdown_open = false;
                            this.startup_dropdown_open = false;
                            this.image_dropdown_open = false;
                            cx.notify();
                        });
                    })
                    .into_any_element();

                sec1_items.push(make_row("Display Language", "Select preferred language for editor UI and dialogs", ctrl_lang_btn));

                sections.push(make_section("win-sec-theme", sec1_key, "Visual Theme & Language", sec1_items));

                if self.theme_dropdown_open {
                    let mut menu_items = Vec::new();
                    for entry in &self.theme_options {
                        let t_id = entry.id.clone();
                        let display_label = entry.name.clone();
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
                                .on_click(move |ev, win, cx| {
                                    let _ = item_ed.update(cx, |this, cx| {
                                        this.selected_theme_id = t_id.clone();
                                        this.theme_dropdown_open = false;
                                        this.save(ev, win, cx);
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
                } else if self.lang_dropdown_open {
                    let lang_opts = [("en-US", "English (en-US)"), ("zh-CN", "简体中文 (zh-CN)")];
                    let mut menu_items = Vec::new();
                    for (code, label) in lang_opts {
                        let is_selected = label == lang_display;
                        let item_ed = cx.entity().downgrade();

                        menu_items.push(
                            div()
                                .id(ElementId::Name(format!("win-lang-item-{}", code).into()))
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
                                .on_click(move |ev, win, cx| {
                                    let _ = item_ed.update(cx, |this, cx| {
                                        this.lang_dropdown_open = false;
                                        let _ = apply_configured_language(cx, code);
                                        this.save(ev, win, cx);
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

                // Section 2: Status Bar Options
                let sec2_key = "status_bar";
                let mut sec2_items = Vec::new();

                let sb_main_ed = cx.entity().downgrade();
                let ctrl_sb_main = Switch::new("win-switch-sb-main")
                    .checked(self.status_bar_enabled)
                    .on_click(move |ev, win, cx| {
                        let _ = sb_main_ed.update(cx, |this, cx| {
                            this.status_bar_enabled = !this.status_bar_enabled;
                            this.save(ev, win, cx);
                        });
                    })
                    .into_any_element();

                sec2_items.push(make_row("Status Bar Visibility", "Show or hide the persistent bottom status bar across window", ctrl_sb_main));

                let sb_words_ed = cx.entity().downgrade();
                let ctrl_sb_words = Switch::new("win-switch-sb-words")
                    .checked(self.status_bar_show_word_count)
                    .on_click(move |ev, win, cx| {
                        let _ = sb_words_ed.update(cx, |this, cx| {
                            this.status_bar_show_word_count = !this.status_bar_show_word_count;
                            this.save(ev, win, cx);
                        });
                    })
                    .into_any_element();

                sec2_items.push(make_row("Word Count Badge", "Display real-time document word count in status bar", ctrl_sb_words));

                let sb_pos_ed = cx.entity().downgrade();
                let ctrl_sb_pos = Switch::new("win-switch-sb-pos")
                    .checked(self.status_bar_show_cursor_position)
                    .on_click(move |ev, win, cx| {
                        let _ = sb_pos_ed.update(cx, |this, cx| {
                            this.status_bar_show_cursor_position = !this.status_bar_show_cursor_position;
                            this.save(ev, win, cx);
                        });
                    })
                    .into_any_element();

                sec2_items.push(make_row("Cursor Position Badge", "Display line and column coordinates in status bar", ctrl_sb_pos));

                let sb_side_ed = cx.entity().downgrade();
                let ctrl_sb_side = Switch::new("win-switch-sb-side")
                    .checked(self.status_bar_show_sidebar_toggle)
                    .on_click(move |ev, win, cx| {
                        let _ = sb_side_ed.update(cx, |this, cx| {
                            this.status_bar_show_sidebar_toggle = !this.status_bar_show_sidebar_toggle;
                            this.save(ev, win, cx);
                        });
                    })
                    .into_any_element();

                sec2_items.push(make_row("Sidebar Toggle Button", "Display button to toggle file tree sidebar in status bar", ctrl_sb_side));

                let sb_mode_ed = cx.entity().downgrade();
                let ctrl_sb_mode = Switch::new("win-switch-sb-mode")
                    .checked(self.status_bar_show_mode_switch)
                    .on_click(move |ev, win, cx| {
                        let _ = sb_mode_ed.update(cx, |this, cx| {
                            this.status_bar_show_mode_switch = !this.status_bar_show_mode_switch;
                            this.save(ev, win, cx);
                        });
                    })
                    .into_any_element();

                sec2_items.push(make_row("Mode Switch Button", "Display button to switch Edit/Preview modes in status bar", ctrl_sb_mode));

                sections.push(make_section("win-sec-sb", sec2_key, "Status Bar Options", sec2_items));
            }
            SettingsNav::Editing => {
                // Section 1: Typography & Formatting
                let sec1_key = "typography";
                let mut sec1_items = Vec::new();

                let font_dec = cx.entity().downgrade();
                let font_inc = cx.entity().downgrade();
                let font_ctr = cx.entity().downgrade();
                let curr_size = self.font_size;
                let is_editing_font = self.editing_stepper.as_deref() == Some("font");

                let ctrl_font = render_zed_stepper(
                    "win-font-dec",
                    "win-font-inc",
                    format!("{} px", curr_size),
                    is_editing_font,
                    Box::new(move |_ev, _win, cx| {
                        let _ = font_dec.update(cx, |this, cx| {
                            this.editing_stepper = None;
                            if this.font_size > 8 {
                                this.font_size -= 1;
                                cx.notify();
                            }
                        });
                    }),
                    Box::new(move |_ev, _win, cx| {
                        let _ = font_inc.update(cx, |this, cx| {
                            this.editing_stepper = None;
                            if this.font_size < 48 {
                                this.font_size += 1;
                                cx.notify();
                            }
                        });
                    }),
                    Box::new(move |_ev, _win, cx| {
                        let _ = font_ctr.update(cx, |this, cx| {
                            this.editing_stepper = Some("font".to_string());
                            cx.notify();
                        });
                    }),
                );

                sec1_items.push(make_row(
                    "Editor Font Size",
                    "Baseline font size in pixels for text editor content",
                    ctrl_font,
                ));

                let lh_dec = cx.entity().downgrade();
                let lh_inc = cx.entity().downgrade();
                let lh_ctr = cx.entity().downgrade();
                let curr_lh = self.line_height;
                let is_editing_lh = self.editing_stepper.as_deref() == Some("line_height");

                let ctrl_lh = render_zed_stepper(
                    "win-lh-dec",
                    "win-lh-inc",
                    format!("{:.1}", curr_lh),
                    is_editing_lh,
                    Box::new(move |_ev, _win, cx| {
                        let _ = lh_dec.update(cx, |this, cx| {
                            this.editing_stepper = None;
                            if this.line_height > 1.05 {
                                this.line_height = (this.line_height - 0.1).max(1.0);
                                cx.notify();
                            }
                        });
                    }),
                    Box::new(move |_ev, _win, cx| {
                        let _ = lh_inc.update(cx, |this, cx| {
                            this.editing_stepper = None;
                            if this.line_height < 3.0 {
                                this.line_height = (this.line_height + 0.1).min(3.0);
                                cx.notify();
                            }
                        });
                    }),
                    Box::new(move |_ev, _win, cx| {
                        let _ = lh_ctr.update(cx, |this, cx| {
                            this.editing_stepper = Some("line_height".to_string());
                            cx.notify();
                        });
                    }),
                );

                sec1_items.push(make_row(
                    "Line Height Multiplier",
                    "Adjust vertical line spacing ratio for reading comfort",
                    ctrl_lh,
                ));

                sections.push(make_section("win-sec-typo", sec1_key, "Typography & Formatting", sec1_items));

                // Section 2: Markdown & Assets
                let sec2_key = "markdown";
                let mut sec2_items = Vec::new();

                let tbl_ed = cx.entity().downgrade();
                let show_tb_headers = EditorSettings::show_table_headers(cx);
                let ctrl_tbl = Switch::new("win-switch-table-headers")
                    .checked(show_tb_headers)
                    .on_click(move |_ev, _win, cx| {
                        let _ = tbl_ed.update(cx, |_this, cx| {
                            let new_val = !EditorSettings::show_table_headers(cx);
                            EditorSettings::set_show_table_headers(cx, new_val);
                            cx.notify();
                        });
                    })
                    .into_any_element();

                sec2_items.push(make_row(
                    "Table Column Headers",
                    "Automatically render header row when formatting markdown tables",
                    ctrl_tbl,
                ));

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
                            this.startup_dropdown_open = false;
                            this.theme_dropdown_open = false;
                            this.lang_dropdown_open = false;
                            cx.notify();
                        });
                    })
                    .into_any_element();

                sec2_items.push(make_row("Image Paste Action", "Default storage location when pasting images into document", ctrl_image_btn));

                sections.push(make_section("win-sec-markdown", sec2_key, "Markdown & Assets", sec2_items));

                // Section 3: Startup Options
                let sec3_key = "startup";
                let mut sec3_items = Vec::new();

                let startup_label = match self.startup_open {
                    StartupOpenSetting::NewFile => "New Blank Document",
                    StartupOpenSetting::LastOpenedFile => "Open Last Opened File",
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
                            this.image_dropdown_open = false;
                            this.theme_dropdown_open = false;
                            this.lang_dropdown_open = false;
                            cx.notify();
                        });
                    })
                    .into_any_element();

                sec3_items.push(make_row("On Startup", "Choose default document state when launching Velotype editor", ctrl_startup_btn));

                sections.push(make_section("win-sec-startup", sec3_key, "Startup Options", sec3_items));

                if self.startup_dropdown_open {
                    let startup_opts = [
                        (StartupOpenSetting::NewFile, "New Blank Document"),
                        (StartupOpenSetting::LastOpenedFile, "Open Last Opened File"),
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
                                .on_click(move |ev, win, cx| {
                                    let _ = item_ed.update(cx, |this, cx| {
                                        this.startup_open = pref;
                                        this.startup_dropdown_open = false;
                                        this.save(ev, win, cx);
                                    });
                                })
                                .into_any_element(),
                        );
                    }

                    active_panel_overlay = Some(
                        div()
                            .absolute()
                            .top(px(210.0))
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
                                .on_click(move |ev, win, cx| {
                                    let _ = item_ed.update(cx, |this, cx| {
                                        this.image_paste_behavior = pref;
                                        this.image_dropdown_open = false;
                                        this.save(ev, win, cx);
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
            SettingsNav::Keymap => {
                // Section 1: Document Actions
                let sec1_key = "doc_actions";
                let mut sec1_items = Vec::new();

                let doc_shortcuts = [
                    ("Save Document", "Save active file changes to disk", "Ctrl + S"),
                    ("Save Document As", "Save active document with a new name", "Ctrl + Shift + S"),
                    ("New Window", "Open a new editor window instance", "Ctrl + N"),
                    ("Close Window", "Close the currently focused editor window", "Ctrl + W"),
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

                sections.push(make_section("win-sec-doc-actions", sec1_key, "Document Actions", sec1_items));

                // Section 2: Interface & View Controls
                let sec2_key = "view_controls";
                let mut sec2_items = Vec::new();

                let view_shortcuts = [
                    ("Toggle View Mode", "Switch between Edit, Preview, and Dual view layouts", "Ctrl + M"),
                    ("Toggle Workspace Tree", "Show or collapse the left file navigation sidebar", "Ctrl + E"),
                    ("Quit Application", "Safely exit application and save session", "Ctrl + Q"),
                ];

                for (name, desc, sc) in view_shortcuts.iter() {
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

                    sec2_items.push(make_row(*name, *desc, ctrl_sc));
                }

                sections.push(make_section("win-sec-view-controls", sec2_key, "Interface & View Controls", sec2_items));
            }
        }

        let mut right_content = div()
            .id("win-pref-right-content")
            .relative()
            .w_full()
            .flex_1()
            .min_w(px(0.0))
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

        let main_body = div()
            .w_full()
            .h_full()
            .flex()
            .flex_row()
            .child(left_nav)
            .child(right_content);

        let content = div()
            .size_full()
            .pt(px(titlebar_height))
            .flex()
            .flex_col()
            .key_context("Settings")
            .track_focus(&self.focus_handle)
            .bg(c.editor_background)
            .text_color(c.dialog_body)
            .child(main_body);

        let root = div()
            .size_full()
            .relative()
            .bg(c.editor_background)
            .child(content);

        if let Some(titlebar) = render_custom_titlebar(
            "win-pref-titlebar",
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

fn open_settings_window_with_state(
    cx: &mut App,
    settings: AppSettings,
    theme_options: Vec<ThemeCatalogEntry>,
    title: String,
) -> WindowHandle<SettingsWindow> {
    let bounds = Bounds::centered(None, size(px(720.0), px(480.0)), cx);
    let window_title = SharedString::from(title);
    let handle = cx
        .open_window(
            velotype_window_options(window_title, bounds),
            move |_window, cx| {
                cx.new(move |cx| SettingsWindow::new(settings, theme_options, cx))
            },
        )
        .expect("settings window should open");

    handle
        .update(cx, |settings_win, window, _cx| {
            window.activate_window();
            settings_win.focus_handle.focus(window);
        })
        .expect("newly opened settings window should be updateable");

    handle
}

pub(crate) fn open_settings_window(cx: &mut App) -> WindowHandle<SettingsWindow> {
    let settings = match read_app_settings() {
        Ok(settings) => settings,
        Err(err) => {
            eprintln!("failed to read app settings: {err}");
            AppSettings::default()
        }
    };
    let theme_options = cx.global::<ThemeManager>().available_themes().to_vec();
    let title = cx
        .global::<I18nManager>()
        .strings()
        .settings_window_title
        .clone();
    open_settings_window_with_state(cx, settings, theme_options, title)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_settings_default() {
        let settings = AppSettings::default();
        assert_eq!(settings.startup_open, StartupOpenSetting::NewFile);
        assert_eq!(settings.default_language_id, "en-US");
        assert_eq!(settings.default_theme_id, "velotype");
    }
}
