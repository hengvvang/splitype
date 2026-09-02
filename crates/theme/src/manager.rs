//! Theme manager — the gpui global owning the registry and the resolved
//! current theme.
//!
//! The manager never picks themes on its own: [`ThemeSettingsContent`] in
//! the settings store is the single source of truth. Settings writes flow
//! through `SettingsStore` sync hooks into [`ThemeManager::apply_settings`],
//! which re-resolves only when the settings snapshot actually changed.

use std::path::Path;
use std::sync::Arc;

use anyhow::Context as _;
use gpui::{App, BorrowAppContext, Global};

use config::dirs::SplitypeConfigDirs;
use config::jsonc::read_json_or_jsonc;
use config::settings::{Appearance, CoreSettings, PluginSettings, ThemeSettingsContent};

use super::content::{ThemeFamilyContent, validate_family};
use super::registry::{ThemeCatalogEntry, ThemeRegistry, TokenDeclaration};
use super::resolve::resolve_theme;
use super::theme::Theme;

/// The built-in family id.
pub const BUILTIN_THEME_FAMILY_ID: &str = "splitype";

/// A theme family file imported into the user's theme library.
pub struct ImportedTheme {
    pub family_id: String,
    pub theme_id: String,
    pub appearance: Appearance,
}

/// Global singleton that holds the resolved current [`Theme`].
///
/// Registered via [`Global`] so every component can access it through
/// `cx.global::<ThemeManager>().current()` without passing props.
pub struct ThemeManager {
    registry: ThemeRegistry,
    current: Arc<Theme>,
    current_theme_id: String,
    settings_snapshot: ThemeSettingsContent,
}

impl Global for ThemeManager {}

impl Default for ThemeManager {
    fn default() -> Self {
        Self {
            registry: ThemeRegistry::with_builtins(),
            current: Arc::new(Theme::default_theme()),
            current_theme_id: format!("{BUILTIN_THEME_FAMILY_ID}.dark"),
            settings_snapshot: ThemeSettingsContent::default(),
        }
    }
}

impl ThemeManager {
    /// Installs the manager, loading user theme families and applying the
    /// settings store's theme settings. `SettingsStore` must be initialized
    /// first.
    pub fn init(cx: &mut App) {
        let mut manager = Self::default();
        if let Ok(dirs) = SplitypeConfigDirs::from_system()
            && let Err(err) = manager.registry.load_user_themes(&dirs)
        {
            tracing::warn!(error = %err, "failed to load user theme families");
        }
        let settings = PluginSettings::<CoreSettings>::get(cx).theme;
        let _ = manager.apply_settings(&settings);
        cx.set_global(manager);
    }

    /// Registers the settings sync hook that keeps the active theme in lock
    /// step with the settings store. Call once during application bootstrap.
    pub fn register_settings_sync_hook() {
        config::settings::SettingsStore::register_sync_hook(|cx, settings| {
            let theme_settings = settings.plugin_settings::<CoreSettings>().theme;
            cx.update_global::<ThemeManager, _>(|manager, _cx| {
                manager.apply_settings(&theme_settings);
            });
        });
    }

    /// Returns the currently active theme.
    pub fn current(&self) -> &Theme {
        &self.current
    }

    /// Returns an `Arc` clone of the currently active theme — O(1), no
    /// per-field copy. Use this in hot render paths instead of cloning the
    /// whole `Theme` struct.
    pub fn current_arc(&self) -> Arc<Theme> {
        self.current.clone()
    }

    /// Concrete id (`family.variant`) of the currently active theme.
    pub fn current_theme_id(&self) -> &str {
        &self.current_theme_id
    }

    /// The theme registry backing this manager.
    pub fn registry(&self) -> &ThemeRegistry {
        &self.registry
    }

    /// Every selectable theme exposed in menus and settings.
    pub fn available_themes(&self) -> Vec<ThemeCatalogEntry> {
        self.registry.catalog()
    }

    /// Appearance of a concrete theme id, for menu icons.
    pub fn appearance_of(&self, theme_id: &str) -> Option<Appearance> {
        self.registry
            .catalog()
            .into_iter()
            .find(|entry| entry.id == theme_id)
            .map(|entry| entry.appearance)
    }

    /// Re-resolves the active theme from the given settings snapshot,
    /// returning whether the snapshot changed. On resolution failure the
    /// previous theme is kept and the error is logged.
    pub fn apply_settings(&mut self, settings: &ThemeSettingsContent) -> bool {
        if settings == &self.settings_snapshot {
            return false;
        }
        self.settings_snapshot = settings.clone();
        self.resolve_current();
        true
    }

    /// Registers one plugin's theme contributions — theme families (JSONC
    /// documents in the same format as user theme files) and extension token
    /// declarations — then re-resolves the active theme so contributions take
    /// effect even when the settings snapshot did not change. Family parsing
    /// is all-or-nothing: a single invalid document registers nothing.
    pub fn register_plugin_contributions(
        &mut self,
        plugin_id: &str,
        family_jsoncs: &[String],
        tokens: &[TokenDeclaration],
    ) -> anyhow::Result<()> {
        let mut families = Vec::with_capacity(family_jsoncs.len());
        for jsonc in family_jsoncs {
            let value = config::jsonc::parse_jsonc_value(jsonc)?;
            let family: ThemeFamilyContent = serde_json::from_value(value)?;
            validate_family(&family)?;
            families.push(family);
        }
        self.registry.insert_plugin(plugin_id, families)?;
        self.registry
            .register_tokens(plugin_id, tokens.iter().cloned());
        self.resolve_current();
        Ok(())
    }

    /// Removes one plugin's theme contributions and re-resolves. When the
    /// re-resolution fails (e.g. the settings still select the removed
    /// family), the previously resolved theme keeps rendering and a warning
    /// is logged.
    pub fn unregister_plugin_contributions(&mut self, plugin_id: &str) {
        self.registry.remove_plugin(plugin_id);
        self.registry.unregister_plugin_tokens(plugin_id);
        self.resolve_current();
    }

    /// Resolves the active theme from the last applied settings snapshot,
    /// regardless of whether it changed (used after registry mutations).
    fn resolve_current(&mut self) {
        let settings = self.settings_snapshot.clone();
        match resolve_theme(
            &self.registry,
            &settings.family,
            settings.appearance,
            &settings.overrides,
        ) {
            Ok(resolved) => {
                self.current = resolved.theme;
                self.current_theme_id = resolved.id;
            }
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    family = %settings.family,
                    "failed to resolve theme settings; keeping the current theme"
                );
            }
        }
    }

    /// Imports a theme family file into the user theme library: validates,
    /// persists a normalized copy, registers it, and resolves it once to
    /// surface any base-chain or override errors at import time.
    pub fn import_theme_file(&mut self, path: impl AsRef<Path>) -> anyhow::Result<ImportedTheme> {
        let dirs = SplitypeConfigDirs::from_system()?;
        self.import_theme_file_with_dirs(path, &dirs)
    }

    fn import_theme_file_with_dirs(
        &mut self,
        path: impl AsRef<Path>,
        dirs: &SplitypeConfigDirs,
    ) -> anyhow::Result<ImportedTheme> {
        let path = path.as_ref();
        let value = read_json_or_jsonc(path)?;
        let family: ThemeFamilyContent = serde_json::from_value(value)
            .with_context(|| format!("invalid theme file '{}'", path.display()))?;
        validate_family(&family)?;

        let family_id = family.id();
        let variant = family
            .themes
            .iter()
            .find(|theme| theme.appearance == self.settings_snapshot.appearance)
            .or_else(|| family.themes.first())
            .expect("family validated to have at least one theme");
        let theme_id = format!("{family_id}.{}", variant.id());
        let appearance = variant.appearance;

        self.registry.insert_user(family.clone())?;
        if let Err(err) = resolve_theme(
            &self.registry,
            &theme_id,
            appearance,
            &self.settings_snapshot.overrides,
        ) {
            self.registry.remove_user(&family_id);
            return Err(err).with_context(|| {
                format!("theme family '{family_id}' does not resolve from base '{theme_id}'")
            });
        }

        let themes_dir = dirs.themes_dir();
        std::fs::create_dir_all(&themes_dir)?;
        std::fs::write(
            themes_dir.join(format!("{family_id}.json")),
            serde_json::to_string_pretty(&family)?,
        )?;

        Ok(ImportedTheme {
            family_id,
            theme_id,
            appearance,
        })
    }
}

/// Selects a concrete theme (`family.variant`) and records the choice in
/// the settings store; the settings sync hook applies it live.
pub fn apply_theme_selection(cx: &mut App, theme_id: &str) -> anyhow::Result<()> {
    let selection = cx.update_global::<ThemeManager, _>(|manager, _cx| {
        manager
            .available_themes()
            .into_iter()
            .find(|entry| entry.id == theme_id)
            .map(|entry| (entry.family, entry.appearance))
    });
    let Some((family, appearance)) = selection else {
        anyhow::bail!("unknown theme '{theme_id}'");
    };
    PluginSettings::<CoreSettings>::update(cx, |settings| {
        settings.theme.family = family;
        settings.theme.appearance = appearance;
    })?;
    Ok(())
}

/// Imports a theme family file and selects it through the settings store.
pub fn import_theme_config_and_select(
    cx: &mut App,
    path: impl AsRef<Path>,
) -> anyhow::Result<String> {
    let imported =
        cx.update_global::<ThemeManager, _>(|manager, _cx| manager.import_theme_file(path))?;
    PluginSettings::<CoreSettings>::update(cx, |settings| {
        settings.theme.family = imported.family_id;
        settings.theme.appearance = imported.appearance;
    })?;
    Ok(imported.theme_id)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::content::ThemeStyleContent;
    use crate::registry::ThemeRegistry;
    use crate::resolve::resolve_theme;
    use config::dirs::SplitypeConfigDirs;
    use gpui::rgba;

    #[test]
    fn switches_builtin_themes_via_settings() {
        let mut manager = ThemeManager::default();
        assert_eq!(manager.current_theme_id(), "splitype.dark");
        assert_eq!(manager.current().name, "Dark");
        assert_eq!(manager.current().appearance, Appearance::Dark);

        let settings = ThemeSettingsContent {
            appearance: Appearance::Light,
            ..Default::default()
        };
        assert!(manager.apply_settings(&settings));
        assert_eq!(manager.current_theme_id(), "splitype.light");
        assert_eq!(manager.current().name, "Light");
        assert_eq!(
            manager.current().colors.editor_background,
            Theme::light_theme().colors.editor_background
        );

        // The same snapshot re-applies without re-resolving.
        assert!(!manager.apply_settings(&settings));

        // An unknown family keeps the current theme.
        let broken = ThemeSettingsContent {
            family: "missing".into(),
            ..Default::default()
        };
        assert!(manager.apply_settings(&broken));
        assert_eq!(manager.current_theme_id(), "splitype.light");
    }

    #[test]
    fn imports_family_file_and_persists_normalized_json() {
        let root = std::env::temp_dir().join(format!("splitype-theme-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("temp root should be created");
        let source = root.join("theme.jsonc");
        std::fs::write(
            &source,
            r##"{
                // A user theme family with one variant.
                "name": "Night Writer",
                "author": "Ada",
                "themes": [
                    {
                        "name": "Dark",
                        "appearance": "dark",
                        "style": {
                            "base": "splitype.Dark",
                            "colors": {
                                "focus_accent": "#8b5cf6"
                            },
                            "dimensions": {
                                "block_gap": 12.0
                            }
                        }
                    }
                ]
            }"##,
        )
        .expect("theme family config should be written");

        let dirs = SplitypeConfigDirs::from_root(&root);
        let mut manager = ThemeManager::default();
        let imported = manager
            .import_theme_file_with_dirs(&source, &dirs)
            .expect("theme family config should import");

        assert_eq!(imported.family_id, "night_writer");
        assert_eq!(imported.theme_id, "night_writer.dark");
        assert_eq!(imported.appearance, Appearance::Dark);

        // Base inheritance plus the patch.
        let resolved = resolve_theme(
            &manager.registry,
            "night_writer.dark",
            Appearance::Dark,
            &BTreeMap::new(),
        )
        .expect("imported theme should resolve");
        assert_eq!(resolved.theme.name, "Dark");
        assert_eq!(resolved.theme.colors.focus_accent, rgba(0x8b5cf6ff).into());
        assert_eq!(resolved.theme.dimensions.block_gap, 12.0);
        assert_eq!(
            resolved.theme.colors.editor_background,
            Theme::default_theme().colors.editor_background
        );
        assert_eq!(resolved.theme.dimensions.menu_text_size, 11.0);

        // The normalized copy is persisted in the family format.
        let normalized = std::fs::read_to_string(dirs.themes_dir().join("night_writer.json"))
            .expect("normalized theme family config should exist");
        assert!(normalized.contains("\"name\": \"Night Writer\""));
        assert!(normalized.contains("\"base\": \"splitype.Dark\""));
        assert!(normalized.contains("\"focus_accent\": \"#8b5cf6ff\""));

        // A fresh manager reloads the family from disk.
        let mut reloaded = ThemeManager::default();
        reloaded
            .registry
            .load_user_themes(&dirs)
            .expect("saved family should reload");
        assert!(reloaded.registry.family("night_writer").is_some());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn applies_settings_color_overrides_above_everything() {
        let mut manager = ThemeManager::default();
        let mut settings = ThemeSettingsContent::default();
        settings
            .overrides
            .insert("colors.editor_background".into(), rgba(0x123456ff).into());
        assert!(manager.apply_settings(&settings));
        assert_eq!(
            manager.current().colors.editor_background,
            rgba(0x123456ff).into()
        );

        // An unknown override key fails resolution and keeps the theme.
        let mut broken = ThemeSettingsContent::default();
        broken
            .overrides
            .insert("colors.nope".into(), rgba(0xffffffff).into());
        assert!(manager.apply_settings(&broken));
        assert_eq!(
            manager.current().colors.editor_background,
            rgba(0x123456ff).into()
        );
    }

    #[test]
    fn rejects_base_cycles_and_unknown_file_keys() {
        let mut registry = ThemeRegistry::with_builtins();
        let family = ThemeFamilyContent {
            name: "Cycle".into(),
            author: String::new(),
            themes: vec![
                crate::ThemeContent {
                    name: "A".into(),
                    appearance: Appearance::Dark,
                    style: ThemeStyleContent {
                        base: Some("Cycle.B".into()),
                        ..Default::default()
                    },
                },
                crate::ThemeContent {
                    name: "B".into(),
                    appearance: Appearance::Dark,
                    style: ThemeStyleContent {
                        base: Some("Cycle.A".into()),
                        ..Default::default()
                    },
                },
            ],
        };
        registry
            .insert_user(family)
            .expect("family should validate");
        let err = resolve_theme(&registry, "cycle.a", Appearance::Dark, &BTreeMap::new())
            .expect_err("base cycle must fail resolution");
        assert!(err.to_string().contains("cycle"), "error was: {err}");

        // Unknown keys in theme files are hard errors.
        let raw = r##"{ "name": "X", "themes": [{ "name": "Dark", "style": { "colors": { "nope": "#fff" } } }] }"##;
        assert!(serde_json::from_str::<ThemeFamilyContent>(raw).is_err());

        // Import-time resolution failure removes the family again.
        let mut manager = ThemeManager::default();
        let root = std::env::temp_dir().join(format!("splitype-cycle-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("temp root should be created");
        let source = root.join("broken.json");
        std::fs::write(
            &source,
            r#"{ "name": "Broken", "themes": [{ "name": "Dark", "style": { "base": "missing.Base" } }] }"#,
        )
        .expect("broken theme file should be written");
        let dirs = SplitypeConfigDirs::from_root(&root);
        assert!(manager.import_theme_file_with_dirs(&source, &dirs).is_err());
        assert!(manager.registry.family("broken").is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn registers_plugin_contributions_and_reapplies() {
        let mut manager = ThemeManager::default();
        let family_jsonc = r##"{
            "name": "Acme",
            "author": "Acme Inc",
            "themes": [
                {
                    "name": "Dark",
                    "appearance": "dark",
                    "style": {
                        "base": "splitype",
                        "colors": { "focus_accent": "#8b5cf6" }
                    }
                }
            ]
        }"##;
        manager
            .register_plugin_contributions(
                "com.acme",
                &[family_jsonc.to_string()],
                &[TokenDeclaration {
                    key: "com.acme.brand".into(),
                    default: Some(rgba(0x8b5cf6ff).into()),
                    description: "Brand color".into(),
                }],
            )
            .expect("plugin contributions should register");

        // The token default resolves into the active theme immediately.
        assert_eq!(
            manager.current().token("com.acme.brand"),
            Some(rgba(0x8b5cf6ff).into())
        );
        assert!(
            manager
                .available_themes()
                .iter()
                .any(|entry| entry.id == "acme.dark")
        );

        // Settings can select the plugin-contributed family.
        let settings = ThemeSettingsContent {
            family: "acme".into(),
            ..Default::default()
        };
        assert!(manager.apply_settings(&settings));
        assert_eq!(manager.current_theme_id(), "acme.dark");
        assert_eq!(
            manager.current().colors.focus_accent,
            rgba(0x8b5cf6ff).into()
        );

        // A settings override above the plugin family.
        let mut overridden = ThemeSettingsContent {
            family: "acme".into(),
            ..Default::default()
        };
        overridden
            .overrides
            .insert("com.acme.brand".into(), rgba(0x112233ff).into());
        assert!(manager.apply_settings(&overridden));
        assert_eq!(
            manager.current().token("com.acme.brand"),
            Some(rgba(0x112233ff).into())
        );

        // Unregistering removes the family and the token; the kept theme
        // keeps rendering until the next settings change.
        manager.unregister_plugin_contributions("com.acme");
        assert!(manager.registry.family("acme").is_none());
        assert!(manager.registry.token_schema().is_empty());
        assert!(
            !manager
                .available_themes()
                .iter()
                .any(|entry| entry.id == "acme.dark")
        );
        assert!(
            resolve_theme(
                &manager.registry,
                "acme.dark",
                Appearance::Dark,
                &BTreeMap::new()
            )
            .is_err()
        );
    }

    #[test]
    fn extension_tokens_without_default_fall_back_to_consumers() {
        let mut manager = ThemeManager::default();
        manager
            .register_plugin_contributions(
                "com.acme",
                &[],
                &[TokenDeclaration {
                    key: "com.acme.accent".into(),
                    default: None,
                    description: String::new(),
                }],
            )
            .expect("token should register");

        // No default: the consumer supplies its own fallback.
        assert_eq!(manager.current().token("com.acme.accent"), None);

        // A settings override brings the token to life.
        let mut settings = ThemeSettingsContent::default();
        settings
            .overrides
            .insert("com.acme.accent".into(), rgba(0xaabbccff).into());
        assert!(manager.apply_settings(&settings));
        assert_eq!(
            manager.current().token("com.acme.accent"),
            Some(rgba(0xaabbccff).into())
        );

        // Unknown extension keys in theme files are hard errors.
        let mut extension = BTreeMap::new();
        extension.insert("com.unknown.token".into(), rgba(0xffffffff).into());
        let family = ThemeFamilyContent {
            name: "Bad".into(),
            author: String::new(),
            themes: vec![crate::ThemeContent {
                name: "Dark".into(),
                appearance: Appearance::Dark,
                style: ThemeStyleContent {
                    extension: Some(extension),
                    ..Default::default()
                },
            }],
        };
        manager
            .registry
            .insert_user(family)
            .expect("family should validate");
        assert!(
            resolve_theme(
                &manager.registry,
                "bad.dark",
                Appearance::Dark,
                &BTreeMap::new()
            )
            .is_err()
        );
    }
}
