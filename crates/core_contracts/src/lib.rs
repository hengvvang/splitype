//! core_contracts — unified universal contracts, traits, and domain vocabulary
//! for the splitype editor family and application shell.

pub mod command;
pub mod document;
pub mod export;
pub mod outline;
pub mod pane;
pub mod panel;
pub mod plugin;
pub mod search;

pub use command::{CommandContribution, CommandId, CommandRegistry, CommandRegistryError};
pub use document::{
    DocumentHost, DocumentId, DocumentSnapshot, NavigationExecutionPlan, NavigationTarget, TabKind,
};
pub use export::{ExportError, ExportFormat};
pub use outline::{OutlineHost, OutlineHudState, OutlineNode};
pub use pane::{
    AutoscrollStrategy, PaneCapabilities, PaneDescriptor, PaneHost, PaneId, PaneKind,
    PaneOutlineHost, PaneRegistry, PaneRegistryError, PaneRenderContext, PaneView,
};
pub use panel::{
    DocumentPanel, PanelCapabilities, PanelDescriptor, PanelHost, PanelId, PanelKind,
    PanelRenderContext, PanelView, SidebarPanel,
};
pub use plugin::{
    ManifestCommand, PLUGIN_MANIFEST_VERSION, PluginCapabilities, PluginEntry, PluginId,
    PluginManifest, PluginManifestError, PluginRegistry, PluginRegistryError, PluginResources,
};
pub use search::{
    RawMatch, SearchActiveField, SearchHost, SearchIme, SearchInputSnapshot, SearchMatch,
    SearchPanelState, SearchQuery, SearchScope, SearchStateView, SearchTextInput,
    ceil_char_boundary, compute_preserve_case_replacement, floor_char_boundary,
};
