//! Bundled in-process plugin catalog — the composition-root link between
//! plugin manifests and their statically linked descriptor factories.
//!
//! This module is the ONLY place allowed to map a manifest registration key
//! to concrete descriptor constructors. Everything downstream (registries,
//! shell, resources) treats plugins through their manifests and contracts.

use std::borrow::Cow;
use std::sync::Arc;

use core_contracts::{PaneDescriptor, PanelDescriptor, PluginId, PluginManifest, PluginRegistry};

/// Descriptor factories produced by one in-process registration.
struct PluginRegistration {
    pane_descriptors: Vec<(Arc<dyn PaneDescriptor>, bool)>,
    panel_descriptors: Vec<(Arc<dyn PanelDescriptor>, bool)>,
}

/// Resolves a manifest registration key to the plugin's descriptor factories.
fn descriptors_for(registration: &str) -> Option<PluginRegistration> {
    let registration = match registration {
        "wysiwyg" => PluginRegistration {
            pane_descriptors: vec![(Arc::new(pane_wysiwyg::WysiwygDescriptor::new()), true)],
            panel_descriptors: Vec::new(),
        },
        "source-code" => PluginRegistration {
            pane_descriptors: vec![(
                Arc::new(pane_source_code::SourceCodeDescriptor::new()),
                false,
            )],
            panel_descriptors: Vec::new(),
        },
        "preview" => PluginRegistration {
            pane_descriptors: vec![(Arc::new(pane_preview::PreviewDescriptor::new()), false)],
            panel_descriptors: Vec::new(),
        },
        "editor" => PluginRegistration {
            pane_descriptors: Vec::new(),
            panel_descriptors: vec![(Arc::new(editor::EditorPanelDescriptor::new()), true)],
        },
        "explorer" => PluginRegistration {
            pane_descriptors: Vec::new(),
            panel_descriptors: vec![(Arc::new(explorer::ExplorerPanelDescriptor::new()), false)],
        },
        "settings" => PluginRegistration {
            pane_descriptors: Vec::new(),
            panel_descriptors: vec![(Arc::new(settings::SettingsPanelDescriptor::new()), false)],
        },
        _ => return None,
    };
    Some(registration)
}

/// Bundled manifest sources, compiled into the binary.
const BUNDLED_MANIFESTS: &[&str] = &[
    include_str!("../../../assets/plugins/splitype.editor.toml"),
    include_str!("../../../assets/plugins/splitype.explorer.toml"),
    include_str!("../../../assets/plugins/splitype.settings.toml"),
    include_str!("../../../assets/plugins/splitype.wysiwyg.toml"),
    include_str!("../../../assets/plugins/splitype.source-code.toml"),
    include_str!("../../../assets/plugins/splitype.preview.toml"),
];

/// Discovers and activates every bundled in-process plugin.
///
/// Parses and validates each manifest, registers it in the plugin registry,
/// and registers its descriptors in the pane/panel registries — verifying
/// that every registered kind is declared by the manifest (and vice versa).
pub(crate) fn init_plugins() {
    for source in BUNDLED_MANIFESTS {
        let manifest: PluginManifest =
            toml::from_str(source).expect("bundled plugin manifest must be valid TOML");
        manifest
            .validate()
            .expect("bundled plugin manifest must satisfy its schema");
        let PluginManifest {
            plugin,
            entry,
            capabilities,
            ..
        } = manifest.clone();
        let core_contracts::PluginEntry::InProcess { registration } = entry;
        let factory = descriptors_for(&registration)
            .unwrap_or_else(|| panic!("no descriptor factory for registration '{registration}'"));

        for (descriptor, is_default) in &factory.pane_descriptors {
            assert!(
                capabilities.panes.contains(&descriptor.kind()),
                "pane kind '{}' registered by '{plugin}' is not declared in its manifest",
                descriptor.kind()
            );
            core_contracts::PaneRegistry::register_global(descriptor.clone(), *is_default)
                .expect("bundled pane kinds must be unique");
        }
        for (descriptor, is_default) in &factory.panel_descriptors {
            assert!(
                capabilities.panels.contains(&descriptor.kind()),
                "panel kind '{}' registered by '{plugin}' is not declared in its manifest",
                descriptor.kind()
            );
            window::PanelRegistry::register_global(descriptor.clone(), *is_default)
                .expect("bundled panel kinds must be unique");
        }
        PluginRegistry::register_global(manifest).expect("bundled plugin ids must be unique");
    }
}

/// Resolves a `plugin://<plugin-id>/<path>` resource URL through the owning
/// plugin's manifest: the request maps onto the plugin's declared
/// `resources.icon_root` inside the application asset catalog.
pub(crate) fn resolve_plugin_resource(url: &str) -> Option<Cow<'static, [u8]>> {
    let (plugin_id, resource) = url.split_once('/')?;
    let manifest = PluginRegistry::registered(PluginId::new(plugin_id)).ok()??;
    let icon_root = manifest.resources.icon_root.as_deref()?;
    crate::assets::icon_bytes(&format!("{icon_root}/{resource}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bundled_manifest_parses_and_validates() {
        for source in BUNDLED_MANIFESTS {
            let manifest: PluginManifest =
                toml::from_str(source).expect("manifest must be valid TOML");
            manifest
                .validate()
                .expect("manifest must satisfy its schema");
        }
    }
}
