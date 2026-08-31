//! Plugin manifest registry — the discovery record of every loaded plugin.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use thiserror::Error;

use crate::pane::PaneKind;
use crate::panel::PanelKind;
use crate::plugin::{PluginId, PluginManifest};

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum PluginRegistryError {
    #[error("plugin '{0}' is already registered")]
    DuplicatePlugin(PluginId),
    #[error("plugin registry lock is poisoned")]
    Poisoned,
}

/// Registry of discovered plugin manifests, keyed by plugin id.
///
/// Holds metadata and declared capabilities only — the live pane/panel
/// descriptors stay in their own registries. The shell consults this to map
/// kinds back to their owning plugins (for display and resource resolution).
#[derive(Default)]
pub struct PluginRegistry {
    manifests: HashMap<PluginId, Arc<PluginManifest>>,
    order: Vec<PluginId>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn global() -> &'static Mutex<Self> {
        static REGISTRY: LazyLock<Mutex<PluginRegistry>> =
            LazyLock::new(|| Mutex::new(PluginRegistry::new()));
        &REGISTRY
    }

    pub fn register(&mut self, manifest: PluginManifest) -> Result<(), PluginRegistryError> {
        let id = manifest.plugin.clone();
        if self.manifests.contains_key(&id) {
            return Err(PluginRegistryError::DuplicatePlugin(id));
        }
        self.order.push(id.clone());
        self.manifests.insert(id, Arc::new(manifest));
        Ok(())
    }

    pub fn register_global(manifest: PluginManifest) -> Result<(), PluginRegistryError> {
        Self::global()
            .lock()
            .map_err(|_| PluginRegistryError::Poisoned)?
            .register(manifest)
    }

    pub fn get(&self, id: &PluginId) -> Option<Arc<PluginManifest>> {
        self.manifests.get(id).cloned()
    }

    pub fn registered(id: PluginId) -> Result<Option<Arc<PluginManifest>>, PluginRegistryError> {
        Ok(Self::global()
            .lock()
            .map_err(|_| PluginRegistryError::Poisoned)?
            .get(&id))
    }

    pub fn all(&self) -> Vec<Arc<PluginManifest>> {
        self.order
            .iter()
            .filter_map(|id| self.manifests.get(id).cloned())
            .collect()
    }

    pub fn registered_manifests() -> Result<Vec<Arc<PluginManifest>>, PluginRegistryError> {
        Ok(Self::global()
            .lock()
            .map_err(|_| PluginRegistryError::Poisoned)?
            .all())
    }

    /// The plugin whose manifest declares `kind` as a pane capability.
    pub fn pane_kind_owner(&self, kind: &PaneKind) -> Option<PluginId> {
        self.manifests
            .values()
            .find(|manifest| manifest.capabilities.panes.contains(kind))
            .map(|manifest| manifest.plugin.clone())
    }

    /// The plugin whose manifest declares `kind` as a panel capability.
    pub fn panel_kind_owner(&self, kind: &PanelKind) -> Option<PluginId> {
        self.manifests
            .values()
            .find(|manifest| manifest.capabilities.panels.contains(kind))
            .map(|manifest| manifest.plugin.clone())
    }

    /// Global query for the plugin whose manifest declares `kind` as a panel
    /// capability.
    pub fn panel_kind_owner_global(
        kind: PanelKind,
    ) -> Result<Option<PluginId>, PluginRegistryError> {
        Ok(Self::global()
            .lock()
            .map_err(|_| PluginRegistryError::Poisoned)?
            .panel_kind_owner(&kind))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(id: &'static str) -> PluginManifest {
        PluginManifest {
            manifest_version: crate::plugin::PLUGIN_MANIFEST_VERSION,
            plugin: PluginId::from_static(id),
            name: id.into(),
            version: "0.1.0".into(),
            description: None,
            entry: crate::plugin::PluginEntry::InProcess {
                registration: "test".into(),
            },
            capabilities: crate::plugin::PluginCapabilities::default(),
            resources: crate::plugin::PluginResources::default(),
            commands: Vec::new(),
        }
    }

    #[test]
    fn duplicate_plugin_ids_are_rejected() {
        let mut registry = PluginRegistry::new();
        registry.register(manifest("com.test.one")).unwrap();
        assert_eq!(
            registry.register(manifest("com.test.one")),
            Err(PluginRegistryError::DuplicatePlugin(PluginId::from_static(
                "com.test.one"
            )))
        );
        assert_eq!(registry.all().len(), 1);
    }
}
