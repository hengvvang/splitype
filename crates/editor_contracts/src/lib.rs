//! editor_contracts — unified contracts and domain vocabulary for the splitype editor ecosystem.

pub mod document;
pub mod export;
pub mod outline;
pub mod pane;
pub mod panel;
pub mod search;

pub use document::{
    DocumentHost, DocumentId, DocumentSnapshot, NavigationExecutionPlan, NavigationTarget, TabKind,
};
pub use export::{ExportError, ExportFormat};
pub use outline::{OutlineHost, OutlineHudState, OutlineNode};
pub use pane::{
    AutoscrollStrategy, PaneCapabilities, PaneDescriptor, PaneHost, PaneId, PaneKind,
    PaneOutlineHost, PaneRegistry, PaneRegistryError, PaneRenderContext, PaneView,
};
pub use panel::DocumentPanel;
pub use platform_contracts::{
    CommandContribution, CommandId, CommandRegistry, CommandRegistryError, ManifestCommand,
    PLUGIN_MANIFEST_VERSION, PluginCapabilities, PluginEntry, PluginId, PluginManifest,
    PluginManifestError, PluginRegistry, PluginRegistryError, PluginResources,
    PanelCapabilities, PanelDescriptor, PanelId, PanelKind,
    PanelRenderContext as PlatformPanelRenderContext, PanelView, SidebarPanel,
};
pub use search::{
    RawMatch, SearchActiveField, SearchHost, SearchIme, SearchInputSnapshot, SearchMatch,
    SearchPanelState, SearchQuery, SearchScope, SearchStateView, SearchTextInput,
    ceil_char_boundary, compute_preserve_case_replacement, floor_char_boundary,
};
