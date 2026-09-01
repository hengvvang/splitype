//! platform_contracts — universal window shell, plugin, command, and panel contracts.

pub mod command;
pub mod panel;
pub mod plugin;

pub use command::{CommandContribution, CommandId, CommandRegistry, CommandRegistryError};
pub use panel::{
    PanelCapabilities, PanelDescriptor, PanelId, PanelKind, PanelRenderContext, PanelView,
    SidebarPanel,
};
pub use plugin::{
    ManifestCommand, PLUGIN_MANIFEST_VERSION, PluginCapabilities, PluginEntry, PluginId,
    PluginManifest, PluginManifestError, PluginRegistry, PluginRegistryError, PluginResources,
};
