//! Theme registry — every known theme family and the plugin token schema.

use std::collections::BTreeMap;

use gpui::Hsla;

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
#[derive(Debug, Clone, PartialEq)]
pub struct TokenDeclaration {
    /// Stable token key (e.g. `splitype.explorer.accent`).
    pub key: String,
    /// Fallback color used before any theme overrides it.
    pub default: Hsla,
    /// Human-readable description.
    pub description: String,
}

/// All known theme families keyed by id, with priority `user > plugin >
/// builtin`, plus the plugin extension-token schema.
pub struct ThemeRegistry {
    builtin: BTreeMap<String, ThemeFamilyContent>,
    plugin: BTreeMap<String, ThemeFamilyContent>,
    user: BTreeMap<String, ThemeFamilyContent>,
    token_schema: BTreeMap<String, TokenDeclaration>,
}

impl ThemeRegistry {
    /// Registry seeded with the built-in splitype family.
    pub fn with_builtins() -> Self {
        let mut registry = Self {
            builtin: BTreeMap::new(),
            plugin: BTreeMap::new(),
            user: BTreeMap::new(),
            token_schema: BTreeMap::new(),
        };
        let family = splitype_builtin_family();
        registry.builtin.insert(family.id(), family);
        registry
    }

    /// Inserts a built-in family, overwriting any previous one with its id.
    pub fn insert_builtin(&mut self, family: ThemeFamilyContent) -> anyhow::Result<()> {
        validate_family(&family)?;
        self.builtin.insert(family.id(), family);
        Ok(())
    }

    /// Inserts a plugin-contributed family, overwriting any previous one
    /// with its id.
    pub fn insert_plugin(&mut self, family: ThemeFamilyContent) -> anyhow::Result<()> {
        validate_family(&family)?;
        self.plugin.insert(family.id(), family);
        Ok(())
    }

    /// Removes a plugin-contributed family.
    pub fn remove_plugin(&mut self, family_id: &str) {
        self.plugin.remove(family_id);
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

    /// Highest-priority family with `family_id`, if any.
    pub fn family(&self, family_id: &str) -> Option<&ThemeFamilyContent> {
        self.user
            .get(family_id)
            .or_else(|| self.plugin.get(family_id))
            .or_else(|| self.builtin.get(family_id))
    }

    /// Every family with its id, highest priority first: user, plugin, builtin.
    pub fn families(&self) -> Vec<(String, &ThemeFamilyContent)> {
        let mut families = Vec::new();
        for (id, family) in &self.user {
            families.push((id.clone(), family));
        }
        for (id, family) in &self.plugin {
            if !self.user.contains_key(id) {
                families.push((id.clone(), family));
            }
        }
        for (id, family) in &self.builtin {
            if !self.user.contains_key(id) && !self.plugin.contains_key(id) {
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

    /// Registers plugin extension tokens, overwriting any previous
    /// declaration with the same key.
    pub fn register_tokens(&mut self, tokens: impl IntoIterator<Item = TokenDeclaration>) {
        for token in tokens {
            self.token_schema.insert(token.key.clone(), token);
        }
    }

    /// Removes token declarations by exact key.
    pub fn unregister_tokens(&mut self, keys: &[String]) {
        for key in keys {
            self.token_schema.remove(key);
        }
    }

    /// The registered extension-token schema.
    pub fn token_schema(&self) -> &BTreeMap<String, TokenDeclaration> {
        &self.token_schema
    }

    /// Fallback colors of every registered extension token.
    pub fn token_defaults(&self) -> BTreeMap<String, Hsla> {
        self.token_schema
            .values()
            .map(|token| (token.key.clone(), token.default))
            .collect()
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
