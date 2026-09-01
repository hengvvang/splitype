//! Plugin manifest — the versioned declaration a plugin ships to describe
//! itself: identity, entry point, and capability declarations.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::pane::PaneKind;
use crate::panel::PanelKind;
use crate::plugin::PluginId;

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
    pub panes: Vec<PaneKind>,
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
        Ok(())
    }
}
