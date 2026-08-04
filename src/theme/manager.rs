//! Theme manager for loading, switching, and applying themes.

use std::path::Path;
use std::sync::Arc;

use gpui::{App, Global};

use crate::infra::config::dirs::VelotypeConfigDirs;
use crate::infra::config::jsonc::{read_json_or_jsonc, sanitize_config_file_stem};

use super::theme::{
    BUILTIN_THEME_VELOTYPE_ID, BUILTIN_THEME_VELOTYPE_LIGHT_ID, BUILTIN_THEME_VELOTYPE_LIGHT_NAME,
    BUILTIN_THEME_VELOTYPE_NAME, CUSTOM_THEME_ID, CustomThemeEntry, Theme, ThemeCatalogEntry,
    builtin_theme_catalog, custom_theme_from_value, custom_theme_from_value_with_default_base,
};

/// Global singleton that holds the current [`Theme`].
///
/// Registered via [`Global`] so every component can access it through
/// `cx.global::<ThemeManager>().current()` without passing props.
pub struct ThemeManager {
    current: Arc<Theme>,
    current_theme_id: String,
    custom_themes: Vec<CustomThemeEntry>,
    theme_catalog: Vec<ThemeCatalogEntry>,
}

impl Global for ThemeManager {}

impl Default for ThemeManager {
    fn default() -> Self {
        Self {
            current: Arc::new(Theme::default_theme()),
            current_theme_id: BUILTIN_THEME_VELOTYPE_ID.into(),
            custom_themes: Vec::new(),
            theme_catalog: builtin_theme_catalog(),
        }
    }
}

#[allow(unused)]
impl ThemeManager {
    /// Installs the configured theme into GPUI's global state.
    pub fn init(cx: &mut App) {
        let theme_id = crate::infra::config::settings::read_app_settings()
            .map(|preferences| preferences.default_theme_id)
            .unwrap_or_else(|_| BUILTIN_THEME_VELOTYPE_ID.into());
        Self::init_with_theme_id(cx, &theme_id);
    }

    /// Installs a specific theme into GPUI's global state.
    pub fn init_with_theme_id(cx: &mut App, theme_id: &str) {
        let mut manager = Self::default();
        if let Ok(dirs) = VelotypeConfigDirs::from_system()
            && let Err(err) = manager.load_custom_themes_from_dirs(&dirs)
        {
            eprintln!("failed to load custom themes: {err}");
        }
        let _ = manager.set_theme_by_id(theme_id);
        cx.set_global(manager);
    }

    /// Returns the currently active theme.
    pub fn current(&self) -> &Theme {
        &self.current
    }

    /// Returns an `Arc` clone of the currently active theme — O(1), no
    /// per-field copy. Use this in hot render paths instead of cloning the
    /// whole `Theme` struct (which has ~200 fields and a `String` name).
    pub fn current_arc(&self) -> Arc<Theme> {
        self.current.clone()
    }

    /// Returns the identifier of the currently active theme.
    pub fn current_theme_id(&self) -> &str {
        &self.current_theme_id
    }

    /// Returns all built-in and imported themes exposed in the native menu.
    pub fn available_themes(&self) -> &[ThemeCatalogEntry] {
        &self.theme_catalog
    }

    /// Loads and activates a theme from a file.
    pub fn load_file(&mut self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let theme = Theme::from_file(path)?;
        self.current_theme_id = self.theme_id_for_loaded_theme(&theme);
        self.current = Arc::new(theme);
        Ok(())
    }

    /// Loads and activates a theme from JSON text.
    pub fn load_json(&mut self, json: &str) -> anyhow::Result<()> {
        let theme = Theme::from_json(json)?;
        self.current_theme_id = self.theme_id_for_loaded_theme(&theme);
        self.current = Arc::new(theme);
        Ok(())
    }

    /// Replaces the active theme with a fully constructed value.
    pub fn set_theme(&mut self, theme: Theme) {
        self.current_theme_id = self.theme_id_for_loaded_theme(&theme);
        self.current = Arc::new(theme);
    }

    /// Restores the built-in default theme.
    pub fn reset(&mut self) {
        self.current = Arc::new(Theme::default_theme());
        self.current_theme_id = BUILTIN_THEME_VELOTYPE_ID.into();
    }

    /// Activates a theme by identifier.
    pub fn set_theme_by_id(&mut self, theme_id: &str) -> bool {
        match theme_id {
            id if id == BUILTIN_THEME_VELOTYPE_ID => {
                self.current = Arc::new(Theme::default_theme());
                self.current_theme_id = BUILTIN_THEME_VELOTYPE_ID.into();
                true
            }
            id if id == BUILTIN_THEME_VELOTYPE_LIGHT_ID => {
                self.current = Arc::new(Theme::light_theme());
                self.current_theme_id = BUILTIN_THEME_VELOTYPE_LIGHT_ID.into();
                true
            }
            id => {
                let Some(entry) = self.custom_themes.iter().find(|entry| entry.id == id) else {
                    return false;
                };
                self.current = Arc::new(entry.theme.clone());
                self.current_theme_id = entry.id.clone();
                true
            }
        }
    }

    /// Imports a user theme pack, persists a normalized copy, and activates it.
    pub fn import_theme_config(&mut self, path: impl AsRef<Path>) -> anyhow::Result<String> {
        let dirs = VelotypeConfigDirs::from_system()?;
        self.import_theme_config_with_dirs(path, &dirs)
    }

    fn import_theme_config_with_dirs(
        &mut self,
        path: impl AsRef<Path>,
        dirs: &VelotypeConfigDirs,
    ) -> anyhow::Result<String> {
        let raw = read_json_or_jsonc(path.as_ref())?;
        let default_base_theme_id = self.theme_import_base_theme_id();
        let (entry, normalized) =
            custom_theme_from_value_with_default_base(raw, default_base_theme_id.as_str())?;
        let file_name = format!(
            "{}_{}.json",
            sanitize_config_file_stem(&entry.name),
            sanitize_config_file_stem(&entry.creator)
        );
        let themes_dir = dirs.themes_dir();
        std::fs::create_dir_all(&themes_dir)?;
        std::fs::write(
            themes_dir.join(file_name),
            serde_json::to_string_pretty(&normalized)?,
        )?;
        let imported_id = entry.id.clone();
        self.upsert_custom_theme(entry);
        self.set_theme_by_id(&imported_id);
        Ok(imported_id)
    }

    fn load_custom_themes_from_dirs(&mut self, dirs: &VelotypeConfigDirs) -> anyhow::Result<()> {
        let themes_dir = dirs.themes_dir();
        if !themes_dir.exists() {
            return Ok(());
        }

        let mut loaded = Vec::new();
        for entry in std::fs::read_dir(&themes_dir)? {
            let path = entry?.path();
            if path.is_file() {
                match read_json_or_jsonc(&path)
                    .and_then(|value| custom_theme_from_value(value).map(|(entry, _)| entry))
                {
                    Ok(entry) => loaded.push(entry),
                    Err(err) => {
                        eprintln!("skipping custom theme config '{}': {err}", path.display())
                    }
                }
            }
        }
        loaded.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then(left.creator.cmp(&right.creator))
        });
        for entry in loaded {
            self.upsert_custom_theme(entry);
        }
        Ok(())
    }

    fn upsert_custom_theme(&mut self, entry: CustomThemeEntry) {
        if let Some(existing) = self
            .custom_themes
            .iter_mut()
            .find(|existing| existing.id == entry.id)
        {
            *existing = entry;
        } else {
            self.custom_themes.push(entry);
        }
        self.rebuild_theme_catalog();
    }

    fn rebuild_theme_catalog(&mut self) {
        let mut catalog = builtin_theme_catalog();
        catalog.extend(self.custom_themes.iter().map(|entry| ThemeCatalogEntry {
            id: entry.id.clone(),
            name: format!("{} - {}", entry.name, entry.creator),
        }));
        self.theme_catalog = catalog;
    }

    fn theme_id_for_loaded_theme(&self, theme: &Theme) -> String {
        if theme.name == BUILTIN_THEME_VELOTYPE_NAME {
            BUILTIN_THEME_VELOTYPE_ID.into()
        } else if theme.name == BUILTIN_THEME_VELOTYPE_LIGHT_NAME {
            BUILTIN_THEME_VELOTYPE_LIGHT_ID.into()
        } else {
            CUSTOM_THEME_ID.into()
        }
    }

    fn theme_import_base_theme_id(&self) -> String {
        match self.current_theme_id.as_str() {
            BUILTIN_THEME_VELOTYPE_LIGHT_ID => BUILTIN_THEME_VELOTYPE_LIGHT_ID.into(),
            BUILTIN_THEME_VELOTYPE_ID => BUILTIN_THEME_VELOTYPE_ID.into(),
            id => self
                .custom_themes
                .iter()
                .find(|entry| entry.id == id)
                .map(|entry| entry.base_theme_id.clone())
                .unwrap_or_else(|| BUILTIN_THEME_VELOTYPE_ID.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ThemeManager;
    use crate::infra::config::dirs::VelotypeConfigDirs;
    use crate::theme::theme::Theme;
    use gpui::rgba;

    #[test]
    fn imports_partial_jsonc_theme_and_persists_normalized_json() {
        let root = std::env::temp_dir().join(format!("velotype-theme-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("temp root should be created");
        let source = root.join("theme.jsonc");
        std::fs::write(
            &source,
            r#"{
                // Required metadata.
                "name": "Night Writer",
                "creator": "Ada",
                "description": "",
                "theme": {
                    "dimensions": {
                        "block_gap": 12.0,
                        "menu_text_size": null
                    },
                    "placeholders": {
                        "empty_editing": ""
                    }
                }
            }"#,
        )
        .expect("theme config should be written");

        let dirs = VelotypeConfigDirs::from_root(&root);
        let mut manager = ThemeManager::default();
        let imported_id = manager
            .import_theme_config_with_dirs(&source, &dirs)
            .expect("theme config should import");

        assert_eq!(manager.current_theme_id(), imported_id);
        assert_eq!(manager.current().name, "Night Writer");
        assert_eq!(
            manager.current().colors.editor_background,
            Theme::default_theme().colors.editor_background
        );
        assert_eq!(manager.current().dimensions.block_gap, 12.0);
        assert_eq!(manager.current().dimensions.menu_text_size, 12.0);
        assert!(
            manager
                .available_themes()
                .iter()
                .any(|entry| { entry.id == imported_id && entry.name == "Night Writer - Ada" })
        );

        let normalized = std::fs::read_to_string(dirs.themes_dir().join("Night_Writer_Ada.json"))
            .expect("normalized theme config should exist");
        assert!(normalized.contains("\"name\": \"Night Writer\""));
        assert!(normalized.contains("\"creator\": \"Ada\""));
        assert!(normalized.contains("\"base_theme_id\": \"velotype\""));
        assert!(normalized.contains("\"block_gap\": 12.0"));
        assert!(!normalized.contains("menu_text_size"));
        assert!(!normalized.contains("empty_editing"));
        assert!(!normalized.contains("description"));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn importing_without_base_uses_current_builtin_theme_as_base() {
        let root =
            std::env::temp_dir().join(format!("velotype-light-theme-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("temp root should be created");
        let source = root.join("theme.jsonc");
        std::fs::write(
            &source,
            r#"{
                "name": "Light Radius",
                "creator": "Ada",
                "theme": {
                    "dimensions": {
                        "menu_panel_radius": 14.0
                    }
                }
            }"#,
        )
        .expect("theme config should be written");

        let dirs = VelotypeConfigDirs::from_root(&root);
        let mut manager = ThemeManager::default();
        assert!(manager.set_theme_by_id("velotype-light"));
        let imported_id = manager
            .import_theme_config_with_dirs(&source, &dirs)
            .expect("theme config should import");

        assert_eq!(manager.current_theme_id(), imported_id);
        assert_eq!(
            manager.current().colors.editor_background,
            Theme::light_theme().colors.editor_background
        );
        assert_eq!(manager.current().dimensions.menu_panel_radius, 14.0);

        let normalized = std::fs::read_to_string(dirs.themes_dir().join("Light_Radius_Ada.json"))
            .expect("normalized theme config should exist");
        assert!(normalized.contains("\"base_theme_id\": \"velotype-light\""));

        let mut reloaded = ThemeManager::default();
        reloaded
            .load_custom_themes_from_dirs(&dirs)
            .expect("saved theme should reload");
        assert!(reloaded.set_theme_by_id(&imported_id));
        assert_eq!(
            reloaded.current().colors.editor_background,
            Theme::light_theme().colors.editor_background
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn theme_manager_switches_builtin_themes() {
        let mut manager = ThemeManager::default();
        assert_eq!(manager.current_theme_id(), "velotype");
        assert_eq!(manager.current().name, "Velotype");
        assert_eq!(
            manager
                .available_themes()
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Velotype", "Velotype Light"]
        );

        assert!(manager.set_theme_by_id("velotype-light"));
        assert_eq!(manager.current_theme_id(), "velotype-light");
        assert_eq!(manager.current().name, "Velotype Light");
        assert_eq!(
            manager.current().colors.editor_background,
            rgba(0xf7f8fbff).into()
        );

        assert!(manager.set_theme_by_id("velotype"));
        assert_eq!(manager.current_theme_id(), "velotype");
        assert_eq!(manager.current().name, "Velotype");
        assert!(!manager.set_theme_by_id("missing"));
    }
}
