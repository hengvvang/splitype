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

pub const DEFAULT_THEME_FAMILY: &str = "splitype";

/// Light/dark appearance of a theme.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Appearance {
    #[default]
    Dark,
    Light,
    /// Follow the operating system's appearance. Resolved to `Dark` or
    /// `Light` whenever theme settings are applied.
    Auto,
}

/// Document selection behavior when launching the application.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupOpenSetting {
    #[default]
    NewFile,
    LastOpenedFile,
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

/// Interface appearance and language configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterfaceSettings {
    #[serde(default = "default_language_id_string")]
    pub language_id: String,
}

impl Default for InterfaceSettings {
    fn default() -> Self {
        Self {
            language_id: crate::language::packs::BUILTIN_LANGUAGE_EN_US_ID.to_string(),
        }
    }
}

fn default_language_id_string() -> String {
    crate::language::packs::BUILTIN_LANGUAGE_EN_US_ID.to_string()
}

/// Theme selection and per-user overrides.
///
/// The theme manager resolves the active theme from this snapshot — settings
/// are the single source of truth for the active theme. Override keys are
/// `colors.<field>` token paths or plugin extension token keys; dimension
/// and typography overrides use their plain field names.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeSettingsContent {
    /// Dark/light appearance of the selected theme.
    #[serde(default)]
    pub appearance: Appearance,
    /// Selected theme family id.
    #[serde(default = "default_theme_family_string")]
    pub family: String,
    /// User color overrides applied above every theme file.
    #[serde(default)]
    pub overrides: BTreeMap<String, gpui::Hsla>,
    /// User dimension overrides keyed by field name (e.g. `block_gap`).
    #[serde(default)]
    pub dimension_overrides: BTreeMap<String, f32>,
    /// User typography overrides keyed by field name (e.g. `text_size`,
    /// `h1_weight`); sizes are numbers, weights are lowercase weight names.
    #[serde(default)]
    pub typography_overrides: BTreeMap<String, serde_json::Value>,
}

impl Default for ThemeSettingsContent {
    fn default() -> Self {
        Self {
            appearance: Appearance::default(),
            family: DEFAULT_THEME_FAMILY.to_string(),
            overrides: BTreeMap::new(),
            dimension_overrides: BTreeMap::new(),
            typography_overrides: BTreeMap::new(),
        }
    }
}

fn default_theme_family_string() -> String {
    DEFAULT_THEME_FAMILY.to_string()
}

/// Typography settings (UI, Prose, and Code font families).
///
/// Font families are plain strings; the empty string means "use the
/// platform default family for the scope". Sizes and line heights live in
/// theme typography, not here.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TypographySettings {
    #[serde(default)]
    pub ui_font_family: String,
    #[serde(default)]
    pub prose_font_family: String,
    #[serde(default)]
    pub code_font_family: String,
}

fn default_true() -> bool {
    true
}

/// The core plugin's settings: application-level configuration owned by the
/// app itself and stored like every other plugin's settings blob.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoreSettings {
    #[serde(default)]
    pub startup: StartupSettings,
    #[serde(default)]
    pub interface: InterfaceSettings,
    #[serde(default)]
    pub typography: TypographySettings,
    #[serde(default)]
    pub theme: ThemeSettingsContent,
    /// User shortcut overrides keyed by full command id (e.g.
    /// `splitype.editor.save`); values are gpui keystroke strings.
    #[serde(default)]
    pub keybindings: BTreeMap<String, Vec<String>>,
}

impl PluginSettingsDefinition for CoreSettings {
    const PLUGIN_ID: &'static str = "splitype.core";
}

/// Unified, canonical user settings persisted under `config.toml`.
///
/// Every plugin — including the core plugin — owns one opaque settings blob
/// keyed by its reverse-domain id; the app core never interprets any of
/// them. Zero redundant DTOs or compatibility shims — serializes and
/// deserializes directly to/from disk.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct AppSettings {
    /// Plugin-contributed settings, keyed by the owning plugin's
    /// reverse-domain id (e.g. `splitype.explorer`). Values are opaque JSON
    /// owned by the plugin.
    #[serde(default)]
    pub plugins: BTreeMap<String, serde_json::Value>,
}

impl AppSettings {
    /// Reads a plugin's typed settings from this snapshot, falling back to
    /// the plugin's defaults when the blob is absent or unparseable.
    pub fn plugin_settings<T: PluginSettingsDefinition>(&self) -> T {
        self.plugins
            .get(T::PLUGIN_ID)
            .and_then(|value| serde_json::from_value(value.clone()).ok())
            .unwrap_or_default()
    }

    /// Reads one settings value by dotted key path (e.g.
    /// `theme.family`) from a plugin's blob.
    pub fn plugin_value(&self, plugin: &str, key: &str) -> Option<serde_json::Value> {
        let blob = self.plugins.get(plugin)?;
        let mut current: &serde_json::Value = blob;
        for segment in key.split('.') {
            current = current.as_object()?.get(segment)?;
        }
        Some(current.clone())
    }
}

/// A plugin's settings model: declared by the plugin crate that owns it and
/// stored by the app core under [`Self::PLUGIN_ID`] without interpretation.
pub trait PluginSettingsDefinition:
    serde::Serialize + serde::de::DeserializeOwned + Default + Clone
{
    /// The owning plugin's reverse-domain id (e.g. `splitype.explorer`),
    /// used as the `config.toml` key for this settings blob.
    const PLUGIN_ID: &'static str;
}

/// Typed accessor over one plugin's settings blob inside
/// [`AppSettings::plugins`].
///
/// Reads fall back to the plugin's `Default` implementation when the blob is
/// absent or unparseable; updates persist through [`SettingsStore::update`],
/// so subsystem sync hooks and window refresh run exactly like app-core
/// setting mutations.
pub struct PluginSettings<T: PluginSettingsDefinition>(std::marker::PhantomData<T>);

impl<T: PluginSettingsDefinition> PluginSettings<T> {
    /// Reads the plugin's current settings.
    pub fn get(cx: &App) -> T {
        SettingsStore::get(cx).plugin_settings::<T>()
    }

    /// Mutates the plugin's settings, persisting the result and refreshing
    /// all windows.
    pub fn update(cx: &mut App, mutate: impl FnOnce(&mut T)) -> anyhow::Result<()> {
        SettingsStore::update(cx, |settings| {
            let mut current = settings.plugin_settings::<T>();
            mutate(&mut current);
            settings.plugins.insert(
                T::PLUGIN_ID.to_string(),
                serde_json::to_value(&current).expect("plugin settings must serialize"),
            );
        })
    }
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

    /// Writes one settings value by dotted key path into a plugin's blob,
    /// persisting and refreshing all windows.
    pub fn set_plugin_value(
        cx: &mut App,
        plugin: &str,
        key: &str,
        value: serde_json::Value,
    ) -> anyhow::Result<()> {
        Self::update(cx, |settings| {
            let blob = settings
                .plugins
                .entry(plugin.to_string())
                .or_insert_with(|| serde_json::Value::Object(Default::default()));
            set_value_at_path(blob, key, value);
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
            Err(_) => default_settings_with_locale(detected_language_id),
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            default_settings_with_locale(detected_language_id)
        }
        Err(err) => {
            return Err(err).with_context(|| format!("failed to read '{}'", path.display()));
        }
    };
    save_app_settings_with_dirs(&settings, dirs)?;
    Ok(settings)
}

/// Default settings with the detected display language pre-selected.
fn default_settings_with_locale(language_id: &'static str) -> AppSettings {
    let mut core = CoreSettings::default();
    core.interface.language_id = language_id.to_string();
    let mut settings = AppSettings::default();
    settings.plugins.insert(
        CoreSettings::PLUGIN_ID.to_string(),
        serde_json::to_value(&core).expect("core settings must serialize"),
    );
    settings
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

/// Writes `value` into `blob` at the dotted `key` path, creating missing
/// intermediate objects.
fn set_value_at_path(blob: &mut serde_json::Value, key: &str, value: serde_json::Value) {
    let mut segments = key.split('.').collect::<Vec<_>>();
    let leaf = segments.pop().expect("settings key must not be empty");
    let mut current = blob;
    for segment in segments {
        if !current.is_object() {
            *current = serde_json::Value::Object(Default::default());
        }
        current = current
            .as_object_mut()
            .expect("just ensured object")
            .entry(segment.to_string())
            .or_insert_with(|| serde_json::Value::Object(Default::default()));
    }
    if !current.is_object() {
        *current = serde_json::Value::Object(Default::default());
    }
    current
        .as_object_mut()
        .expect("just ensured object")
        .insert(leaf.to_string(), value);
}

#[cfg(test)]
mod tests {
    use super::{CoreSettings, detected_language_id_from_locales};

    #[test]
    fn locale_settings_map_to_builtin_languages() {
        assert_eq!(detected_language_id_from_locales(["zh-CN"]), "zh-CN");
        assert_eq!(detected_language_id_from_locales(["zh-HK"]), "zh-CN");
        assert_eq!(detected_language_id_from_locales(["zh-Hant-TW"]), "zh-CN");
        assert_eq!(detected_language_id_from_locales(["zh_SG.UTF-8"]), "zh-CN");
        assert_eq!(detected_language_id_from_locales(["en-US"]), "en-US");
        assert_eq!(detected_language_id_from_locales(["en_GB.UTF-8"]), "en-US");
        assert_eq!(
            detected_language_id_from_locales(["fr-FR", "zh-CN"]),
            "zh-CN"
        );
        assert_eq!(
            detected_language_id_from_locales(Vec::<&str>::new()),
            "en-US"
        );
        assert_eq!(detected_language_id_from_locales(["fr-FR"]), "en-US");
        assert_eq!(detected_language_id_from_locales(["!!!"]), "en-US");
    }

    #[test]
    fn core_manifest_declarations_cover_core_settings() {
        let manifest: platform_contracts::PluginManifest =
            toml::from_str(crate::CORE_MANIFEST_TOML)
                .expect("bundled core manifest must be valid TOML");
        // Keybinding overrides and theme overrides are config-only
        // channels with custom settings UI rather than declaration rows.
        let problems = platform_contracts::verify_setting_declarations::<CoreSettings>(
            &manifest.settings,
            &[
                "keybindings",
                "theme.overrides",
                "theme.dimension_overrides",
                "theme.typography_overrides",
            ],
        );
        assert!(problems.is_empty(), "declaration mismatches: {problems:#?}");
    }
}
