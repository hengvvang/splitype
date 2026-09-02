//! platform_contracts — universal window shell, plugin, command, action, and panel contracts.

pub mod actions;
pub mod command;
pub mod document_id;
pub mod panel;
pub mod plugin;
pub mod settings;

pub use actions::{
    ClosePanel, Copy, Cut, DismissTransientUi, OpenPath, OpenPathInSplit, Paste, SplitPanel,
    ToggleKindDropdown, TogglePanelMaximized, UpdateOpenTabPaths,
};
pub use command::{CommandContribution, CommandId, CommandRegistry, CommandRegistryError};
pub use document_id::DocumentId;
pub use panel::{PanelDescriptor, PanelId, PanelKind, PanelRenderContext, PanelView};
pub use plugin::{
    ManifestCommand, PLUGIN_MANIFEST_VERSION, PluginCapabilities, PluginEntry, PluginId,
    PluginManifest, PluginManifestError, PluginRegistry, PluginRegistryError, PluginResources,
    ThemeFamilyDeclaration,
};
pub use settings::{SettingDeclaration, SettingKind, SettingOption, verify_setting_declarations};
