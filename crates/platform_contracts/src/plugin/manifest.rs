//! Plugin manifest — the versioned declaration a plugin ships to describe
//! itself: identity, entry point, and capability declarations.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::panel::PanelKind;
use crate::plugin::PluginId;
use crate::settings::{SettingDeclaration, SettingKind};

/// Current manifest schema version. Bump on breaking changes; loaders must
/// reject manifests they do not understand.
pub const PLUGIN_MANIFEST_VERSION: u32 = 1;

/// A plugin manifest, as parsed from a plugin's `plugin.toml`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginManifest {
    pub manifest_version: u32,
    /// Reverse-domain plugin id, e.g. `com.vendor.product`.
    pub plugin: PluginId,
    /// Human-readable plugin name.
    pub name: String,
    /// Human-readable plugin version (display only; not used for semver).
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    /// How the plugin's code is provided and activated.
    pub entry: PluginEntry,
    #[serde(default)]
    pub capabilities: PluginCapabilities,
    #[serde(default)]
    pub resources: PluginResources,
    /// Commands the plugin exposes to menus and shortcuts.
    #[serde(default)]
    pub commands: Vec<ManifestCommand>,
    /// Settings schema the plugin contributes to the settings UI.
    #[serde(default)]
    pub settings: Vec<SettingDeclaration>,
    /// Theme families the plugin contributes, in the same JSONC family
    /// format as user theme files.
    #[serde(default)]
    pub themes: Vec<ThemeFamilyDeclaration>,
    /// Extension color tokens the plugin registers for theming.
    #[serde(default)]
    pub theme_tokens: Vec<theme::TokenDeclaration>,
}

/// One theme family contributed by a plugin, written in the same JSONC
/// family format as user theme files.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThemeFamilyDeclaration {
    /// The family document as a JSONC string.
    pub json: String,
}

/// One command contribution declared by a manifest.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ManifestCommand {
    /// Plugin-local command id; the full id is `<plugin-id>.<id>`.
    pub id: String,
    /// Menu skeleton location (e.g. `file`, `file.export`). Absent for
    /// keybinding-only commands.
    #[serde(default)]
    pub menu: Option<String>,
    /// Default shortcuts as gpui keystroke strings. Empty for menu-only
    /// commands.
    #[serde(default)]
    pub shortcuts: Vec<String>,
    /// Optional gpui keybinding context (e.g. `BlockEditor`); the binding
    /// only fires while a focus handle with that context is focused.
    #[serde(default)]
    pub context: Option<String>,
}

/// How a plugin provides its code.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PluginEntry {
    /// Statically linked into the application; `registration` names the
    /// composition-root factory that produces this plugin's descriptors.
    InProcess { registration: String },
}

/// The pane and panel kinds a plugin claims to provide.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PluginCapabilities {
    #[serde(default)]
    pub panes: Vec<String>,
    #[serde(default)]
    pub panels: Vec<PanelKind>,
}

/// Plugin-owned resources exposed through the `plugin://` asset namespace.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PluginResources {
    /// Asset-catalog root of the plugin's bundled icons; a request for
    /// `plugin://<id>/<path>` resolves to `<icon_root>/<path>`.
    #[serde(default)]
    pub icon_root: Option<String>,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum PluginManifestError {
    #[error("unsupported manifest schema version {0}")]
    UnsupportedVersion(u32),
    #[error("plugin id '{0}' must be a reverse-domain name (at least two dot-separated segments)")]
    InvalidPluginId(String),
    #[error("plugin name must not be empty")]
    EmptyName,
    #[error("in-process entry registration key must not be empty")]
    EmptyRegistration,
    #[error("setting '{0}' must declare a non-empty dotted key")]
    InvalidSettingKey(String),
    #[error("setting '{0}' must declare a non-empty title")]
    EmptySettingTitle(String),
    #[error("setting '{0}' is declared more than once")]
    DuplicateSettingKey(String),
    #[error("setting '{0}' has a default that does not match its declared kind")]
    SettingDefaultMismatch(String),
    #[error("enum setting '{0}' must declare at least one option")]
    EnumWithoutOptions(String),
    #[error("number setting '{0}' declares bounds or options only valid on other kinds")]
    InvalidSettingBounds(String),
    #[error("theme token '{0}' must be namespaced under the plugin id")]
    InvalidThemeTokenKey(String),
    #[error("theme token '{0}' is declared more than once")]
    DuplicateThemeTokenKey(String),
    #[error("theme family #{0} is invalid: {1}")]
    InvalidThemeFamily(usize, String),
}

impl PluginManifest {
    /// Validates the structural invariants of a manifest.
    pub fn validate(&self) -> Result<(), PluginManifestError> {
        if self.manifest_version != PLUGIN_MANIFEST_VERSION {
            return Err(PluginManifestError::UnsupportedVersion(
                self.manifest_version,
            ));
        }
        if !self.plugin.is_namespaced() {
            return Err(PluginManifestError::InvalidPluginId(
                self.plugin.to_string(),
            ));
        }
        if self.name.trim().is_empty() {
            return Err(PluginManifestError::EmptyName);
        }
        // In-process is the only entry kind today; future transports add
        // their own validation arms here.
        let PluginEntry::InProcess { registration } = &self.entry;
        if registration.trim().is_empty() {
            return Err(PluginManifestError::EmptyRegistration);
        }
        self.validate_settings()?;
        self.validate_theme_contributions()?;
        Ok(())
    }

    fn validate_theme_contributions(&self) -> Result<(), PluginManifestError> {
        let prefix = format!("{}.", self.plugin);
        let mut seen = std::collections::BTreeSet::new();
        for token in &self.theme_tokens {
            if !token.key.starts_with(&prefix) || token.key.len() == prefix.len() {
                return Err(PluginManifestError::InvalidThemeTokenKey(token.key.clone()));
            }
            if !seen.insert(token.key.clone()) {
                return Err(PluginManifestError::DuplicateThemeTokenKey(
                    token.key.clone(),
                ));
            }
        }
        for (index, declaration) in self.themes.iter().enumerate() {
            let parsed = config::jsonc::parse_jsonc_value(&declaration.json).and_then(|value| {
                serde_json::from_value::<theme::ThemeFamilyContent>(value).map_err(Into::into)
            });
            if let Err(err) = parsed.and_then(|family| theme::validate_family(&family)) {
                return Err(PluginManifestError::InvalidThemeFamily(
                    index,
                    err.to_string(),
                ));
            }
        }
        Ok(())
    }

    fn validate_settings(&self) -> Result<(), PluginManifestError> {
        for declaration in &self.settings {
            let id = format!("{}.{}", self.plugin, declaration.key);
            if declaration.key.trim().is_empty() || declaration.key.starts_with('.') {
                return Err(PluginManifestError::InvalidSettingKey(id));
            }
            if declaration.title.trim().is_empty() {
                return Err(PluginManifestError::EmptySettingTitle(id));
            }
            if self
                .settings
                .iter()
                .filter(|other| other.key == declaration.key)
                .count()
                > 1
            {
                return Err(PluginManifestError::DuplicateSettingKey(id));
            }
            if !declaration.accepts(&declaration.default) {
                return Err(PluginManifestError::SettingDefaultMismatch(id));
            }
            match &declaration.kind {
                SettingKind::Enum => {
                    if declaration.options.is_empty() {
                        return Err(PluginManifestError::EnumWithoutOptions(id));
                    }
                }
                SettingKind::Number => {}
                _ => {
                    if declaration.min.is_some()
                        || declaration.max.is_some()
                        || declaration.step.is_some()
                        || declaration.unit.is_some()
                        || !declaration.options.is_empty()
                    {
                        return Err(PluginManifestError::InvalidSettingBounds(id));
                    }
                }
            }
        }
        Ok(())
    }
}
