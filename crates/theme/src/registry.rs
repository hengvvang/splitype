//! Theme registry — every known theme family and the plugin token schema.

use std::collections::BTreeMap;

use gpui::Hsla;
use serde::{Deserialize, Serialize};

use config::dirs::SplitypeConfigDirs;
use config::jsonc::read_json_or_jsonc;
use config::settings::Appearance;

use super::content::{ThemeContent, ThemeFamilyContent, ThemeStyleContent, validate_family};
use super::theme::Theme;

/// One selectable theme exposed in menus and settings.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeCatalogEntry {
    /// Concrete theme id: `family.variant`.
    pub id: String,
    /// Variant display name.
    pub name: String,
    /// Family id the variant belongs to.
    pub family: String,
    /// Family display name.
    pub family_name: String,
    /// Theme author, if any.
    pub author: String,
    /// Whether the variant is a dark or light theme.
    pub appearance: Appearance,
}

/// One plugin-contributed extension token declaration.
///
/// `default` is the fallback color used until a theme or a settings override
/// sets the token. A `None` default means the consuming UI supplies its own
/// fallback (usually a core theme token), so the token stays in lock step
/// with the active theme until someone overrides it explicitly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenDeclaration {
    /// Stable token key (e.g. `splitype.explorer.accent`), namespaced under
    /// the contributing plugin's id.
    pub key: String,
    /// Fallback color, or `None` for consumer-supplied fallbacks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Hsla>,
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
}

/// All known theme families keyed by id, with priority `user > plugin >
/// builtin`, plus the plugin extension-token schema with per-plugin
/// ownership.
pub struct ThemeRegistry {
    builtin: BTreeMap<String, ThemeFamilyContent>,
    /// Plugin id → the families it contributes. Multiple plugins may
    /// contribute families with colliding ids; the lexicographically later
    /// plugin id wins during resolution.
    plugin: BTreeMap<String, Vec<ThemeFamilyContent>>,
    user: BTreeMap<String, ThemeFamilyContent>,
    /// Token key → declaration.
    token_schema: BTreeMap<String, TokenDeclaration>,
    /// Token key → owning plugin id.
    token_owners: BTreeMap<String, String>,
}

impl ThemeRegistry {
    /// Registry seeded with the built-in splitype family.
    pub fn with_builtins() -> Self {
        let mut registry = Self {
            builtin: BTreeMap::new(),
            plugin: BTreeMap::new(),
            user: BTreeMap::new(),
            token_schema: BTreeMap::new(),
            token_owners: BTreeMap::new(),
        };
        let family = splitype_builtin_family();
        registry.builtin.insert(family.id(), family);
        registry
    }

    /// Replaces one plugin's contributed families (validated all-or-nothing).
    pub fn insert_plugin(
        &mut self,
        plugin_id: &str,
        families: Vec<ThemeFamilyContent>,
    ) -> anyhow::Result<()> {
        for family in &families {
            validate_family(family)?;
        }
        self.plugin.insert(plugin_id.to_string(), families);
        Ok(())
    }

    /// Removes one plugin's contributed families.
    pub fn remove_plugin(&mut self, plugin_id: &str) {
        self.plugin.remove(plugin_id);
    }

    /// Inserts a user family, overwriting any previous one with its id.
    pub fn insert_user(&mut self, family: ThemeFamilyContent) -> anyhow::Result<()> {
        validate_family(&family)?;
        self.user.insert(family.id(), family);
        Ok(())
    }

    /// Removes a user family.
    pub fn remove_user(&mut self, family_id: &str) {
        self.user.remove(family_id);
    }

    /// The user-tier families with their ids — the themes a user imported
    /// and may remove.
    pub fn user_families(&self) -> Vec<(String, &ThemeFamilyContent)> {
        self.user
            .iter()
            .map(|(id, family)| (id.clone(), family))
            .collect()
    }

    /// Registers one plugin's extension tokens, overwriting any previous
    /// declaration (and ownership) with the same key.
    pub fn register_tokens(
        &mut self,
        plugin_id: &str,
        tokens: impl IntoIterator<Item = TokenDeclaration>,
    ) {
        for token in tokens {
            self.token_owners
                .insert(token.key.clone(), plugin_id.to_string());
            self.token_schema.insert(token.key.clone(), token);
        }
    }

    /// Removes every token declaration owned by the plugin.
    pub fn unregister_plugin_tokens(&mut self, plugin_id: &str) {
        self.token_schema.retain(|key, _| {
            self.token_owners
                .get(key)
                .is_none_or(|owner| owner != plugin_id)
        });
        self.token_owners.retain(|_, owner| owner != plugin_id);
    }

    /// The registered extension-token schema.
    pub fn token_schema(&self) -> &BTreeMap<String, TokenDeclaration> {
        &self.token_schema
    }

    /// Fallback colors of the extension tokens that declare a default.
    pub fn token_defaults(&self) -> BTreeMap<String, Hsla> {
        self.token_schema
            .iter()
            .filter_map(|(key, token)| token.default.map(|hsla| (key.clone(), hsla)))
            .collect()
    }

    /// The effective plugin families after cross-plugin id collisions, with
    /// the lexicographically later plugin id winning.
    fn plugin_families(&self) -> BTreeMap<String, &ThemeFamilyContent> {
        let mut merged: BTreeMap<String, &ThemeFamilyContent> = BTreeMap::new();
        for families in self.plugin.values() {
            for family in families {
                merged.insert(family.id(), family);
            }
        }
        merged
    }

    /// Highest-priority family with `family_id`, if any.
    pub fn family(&self, family_id: &str) -> Option<&ThemeFamilyContent> {
        self.user
            .get(family_id)
            .or_else(|| self.plugin_families().get(family_id).copied())
            .or_else(|| self.builtin.get(family_id))
    }

    /// Every family with its id, highest priority first: user, plugin, builtin.
    pub fn families(&self) -> Vec<(String, &ThemeFamilyContent)> {
        let plugin = self.plugin_families();
        let mut families = Vec::new();
        for (id, family) in &self.user {
            families.push((id.clone(), family));
        }
        for (id, family) in &plugin {
            if !self.user.contains_key(id) {
                families.push((id.clone(), *family));
            }
        }
        for (id, family) in &self.builtin {
            if !self.user.contains_key(id) && !plugin.contains_key(id) {
                families.push((id.clone(), family));
            }
        }
        families
    }

    /// Flat catalog of every selectable theme variant, highest priority first.
    pub fn catalog(&self) -> Vec<ThemeCatalogEntry> {
        let mut entries = Vec::new();
        for (family_id, family) in self.families() {
            for theme in &family.themes {
                entries.push(ThemeCatalogEntry {
                    id: format!("{family_id}.{}", theme.id()),
                    name: theme.name.clone(),
                    family: family_id.clone(),
                    family_name: family.name.clone(),
                    author: family.author.clone(),
                    appearance: theme.appearance,
                });
            }
        }
        entries
    }

    /// Loads every theme family file from the user themes directory,
    /// skipping (with a warning) files that fail to parse or validate.
    pub fn load_user_themes(&mut self, dirs: &SplitypeConfigDirs) -> anyhow::Result<()> {
        let themes_dir = dirs.themes_dir();
        if !themes_dir.exists() {
            return Ok(());
        }
        let mut paths: Vec<_> = std::fs::read_dir(&themes_dir)?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.is_file() && config::jsonc::is_supported_config_file(path))
            .collect();
        paths.sort();
        self.user.clear();
        for path in paths {
            match read_json_or_jsonc(&path)
                .and_then(|value| {
                    serde_json::from_value::<ThemeFamilyContent>(value).map_err(Into::into)
                })
                .and_then(|family| validate_family(&family).map(|()| family))
            {
                Ok(family) => {
                    self.user.insert(family.id(), family);
                }
                Err(err) => {
                    tracing::warn!(
                        path = %path.display(),
                        error = %err,
                        "skipping invalid theme family file"
                    );
                }
            }
        }
        Ok(())
    }
}

/// The built-in splitype family: full Dark and Light variants that flow
/// through the same resolution pipeline as user themes.
fn splitype_builtin_family() -> ThemeFamilyContent {
    ThemeFamilyContent {
        name: "Splitype".into(),
        author: String::new(),
        themes: vec![
            ThemeContent {
                name: "Dark".into(),
                appearance: Appearance::Dark,
                style: ThemeStyleContent::from_theme(&Theme::default_theme()),
            },
            ThemeContent {
                name: "Light".into(),
                appearance: Appearance::Light,
                style: ThemeStyleContent::from_theme(&Theme::light_theme()),
            },
        ],
    }
}
