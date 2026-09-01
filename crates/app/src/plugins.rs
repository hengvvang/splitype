//! Bundled in-process plugin catalog — the composition-root link between
//! plugin manifests, their statically linked descriptor factories, and their
//! role adapters.
//!
//! This module is the ONLY place allowed to map a manifest registration key
//! to concrete descriptor constructors and role adapters. Everything
//! downstream (registries, shell, resources) treats plugins through their
//! manifests and contracts.

use std::borrow::Cow;
use std::sync::Arc;

use editor_contracts::PaneDescriptor;
use platform_contracts::{PanelDescriptor, PanelKind, PluginId, PluginManifest, PluginRegistry};

use crate::routing::{DocumentRouting, ExplorerHooks};

/// Descriptor factories and role adapters produced by one in-process
/// registration.
struct PluginRegistration {
    pane_descriptors: Vec<(Arc<dyn PaneDescriptor>, bool)>,
    panel_descriptors: Vec<(Arc<dyn PanelDescriptor>, bool)>,
    /// Document-role adapter plus its kind and whether it is the preferred
    /// document panel of the default window layout.
    document_routing: Option<(PanelKind, DocumentRouting, bool)>,
    explorer_hooks: Option<ExplorerHooks>,
}

/// Resolves a manifest registration key to the plugin's descriptor factories.
fn descriptors_for(registration: &str) -> Option<PluginRegistration> {
    let registration = match registration {
        "wysiwyg" => PluginRegistration {
            pane_descriptors: vec![(Arc::new(wysiwyg::WysiwygDescriptor::new()), true)],
            panel_descriptors: Vec::new(),
            document_routing: None,
            explorer_hooks: None,
        },
        "source-code" => PluginRegistration {
            pane_descriptors: vec![(Arc::new(source_code::SourceCodeDescriptor::new()), false)],
            panel_descriptors: Vec::new(),
            document_routing: None,
            explorer_hooks: None,
        },
        "preview" => PluginRegistration {
            pane_descriptors: vec![(Arc::new(preview::PreviewDescriptor::new()), false)],
            panel_descriptors: Vec::new(),
            document_routing: None,
            explorer_hooks: None,
        },
        "editor" => PluginRegistration {
            pane_descriptors: Vec::new(),
            panel_descriptors: vec![(Arc::new(editor::EditorPanelDescriptor::new()), true)],
            document_routing: Some((
                PanelKind::from_static(editor::PANEL_KIND),
                DocumentRouting {
                    as_document: editor::document_role,
                    as_document_mut: editor::document_role_mut,
                },
                true,
            )),
            explorer_hooks: None,
        },
        "explorer" => PluginRegistration {
            pane_descriptors: Vec::new(),
            panel_descriptors: vec![(Arc::new(explorer::ExplorerPanelDescriptor::new()), false)],
            document_routing: None,
            explorer_hooks: Some(ExplorerHooks {
                kind: PanelKind::from_static(explorer::PANEL_KIND),
                set_active_document_path: explorer::set_active_document_path,
                on_document_path_changed: explorer::on_document_path_changed,
                toggle_tree: explorer::toggle_tree,
                close_folder_scope: explorer::close_folder_scope,
            }),
        },
        "settings" => PluginRegistration {
            pane_descriptors: Vec::new(),
            panel_descriptors: vec![(Arc::new(settings::SettingsPanelDescriptor::new()), false)],
            document_routing: None,
            explorer_hooks: None,
        },
        // The core plugin contributes commands only; no descriptors.
        "core" => PluginRegistration {
            pane_descriptors: Vec::new(),
            panel_descriptors: Vec::new(),
            document_routing: None,
            explorer_hooks: None,
        },
        _ => return None,
    };
    Some(registration)
}

/// Bundled manifest sources, compiled into the binary.
const BUNDLED_MANIFESTS: &[&str] = &[
    include_str!("../../../assets/plugins/splitype.core.toml"),
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
            commands,
            ..
        } = manifest.clone();
        let platform_contracts::PluginEntry::InProcess { registration } = entry;
        let factory = descriptors_for(&registration)
            .unwrap_or_else(|| panic!("no descriptor factory for registration '{registration}'"));

        for (descriptor, is_default) in &factory.pane_descriptors {
            assert!(
                capabilities
                    .panes
                    .iter()
                    .any(|p| p == descriptor.kind().as_str()),
                "pane kind '{}' registered by '{plugin}' is not declared in its manifest",
                descriptor.kind()
            );
            editor_contracts::PaneRegistry::register_global(descriptor.clone(), *is_default)
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
        if let Some((kind, routing, is_primary)) = &factory.document_routing {
            crate::routing::register_document_routing(kind.clone(), *routing, *is_primary);
        }
        if let Some(hooks) = &factory.explorer_hooks {
            crate::routing::register_explorer_hooks(hooks.clone());
        }
        for command in &commands {
            let full_id = format!("{}.{}", plugin, command.id);
            assert!(
                crate::commands::binding_for(plugin.as_str(), &command.id).is_some(),
                "command '{full_id}' declared by '{plugin}' has no composition-root binding"
            );
            platform_contracts::CommandRegistry::register_global(
                platform_contracts::CommandContribution {
                    id: platform_contracts::CommandId::new(full_id),
                    menu: command.menu.clone().map(std::sync::Arc::from),
                    shortcuts: command
                        .shortcuts
                        .iter()
                        .map(|shortcut| std::sync::Arc::from(shortcut.as_str()))
                        .collect(),
                    context: command.context.clone().map(std::sync::Arc::from),
                },
            )
            .expect("bundled command ids must be unique");
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

/// Discovers user-installed plugin manifests under the config `plugins/`
/// directory and records their metadata.
///
/// Code transports for user-installed plugins are not implemented yet, so
/// their entry points cannot be activated; the metadata is still recorded so
/// the shell can name a plugin when its kinds go missing.
pub(crate) fn discover_user_plugins() {
    let Ok(dirs) = config::dirs::SplitypeConfigDirs::from_system() else {
        return;
    };
    discover_user_plugins_in(&dirs.plugins_dir());
}

/// Scans `dir` for `*.toml` plugin manifests, recording valid metadata.
/// Invalid, duplicate, or unreadable manifests are reported and skipped.
pub(crate) fn discover_user_plugins_in(dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return; // no plugins directory yet
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }
        let source = match std::fs::read_to_string(&path) {
            Ok(source) => source,
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "failed to read plugin manifest");
                continue;
            }
        };
        let manifest: PluginManifest = match toml::from_str(&source) {
            Ok(manifest) => manifest,
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "plugin manifest is not valid TOML");
                continue;
            }
        };
        if let Err(error) = manifest.validate() {
            tracing::warn!(path = %path.display(), %error, "plugin manifest failed validation");
            continue;
        }
        let plugin = manifest.plugin.clone();
        if PluginRegistry::registered(plugin.clone())
            .ok()
            .flatten()
            .is_some()
        {
            tracing::warn!(path = %path.display(), plugin = %plugin, "plugin id is already registered");
            continue;
        }
        tracing::warn!(
            path = %path.display(),
            plugin = %plugin,
            "plugin metadata recorded; code transports for user-installed plugins are not implemented"
        );
        if let Err(error) = PluginRegistry::register_global(manifest) {
            tracing::warn!(%error, "failed to record plugin metadata");
        }
    }
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

    #[test]
    fn every_manifest_command_has_a_binding() {
        for source in BUNDLED_MANIFESTS {
            let manifest: PluginManifest =
                toml::from_str(source).expect("manifest must be valid TOML");
            for command in &manifest.commands {
                assert!(
                    crate::commands::binding_for(manifest.plugin.as_str(), &command.id).is_some(),
                    "command '{}.{}' has no composition-root binding",
                    manifest.plugin,
                    command.id
                );
            }
        }
    }

    #[test]
    fn user_plugin_manifests_are_discovered_from_a_directory() {
        let root = std::env::temp_dir().join(format!(
            "splitype-plugin-discovery-test-{}",
            std::process::id()
        ));
        let dirs = config::dirs::SplitypeConfigDirs::from_root(&root);
        let plugins_dir = dirs.plugins_dir();
        std::fs::create_dir_all(&plugins_dir).expect("create plugins dir");
        let manifest_path = plugins_dir.join("com.example.test.toml");
        std::fs::write(
            &manifest_path,
            r#"
manifest_version = 1
plugin = "com.example.test"
name = "Example Test"
version = "0.1.0"

[entry]
kind = "in_process"
registration = "example-test"

[capabilities]
panels = ["com.example.test.panel"]
"#,
        )
        .expect("write manifest");

        discover_user_plugins_in(&plugins_dir);

        let recorded = PluginRegistry::registered(PluginId::new("com.example.test"))
            .expect("registry readable")
            .expect("manifest recorded");
        assert_eq!(recorded.name, "Example Test");

        std::fs::remove_dir_all(&root).ok();
    }
}
