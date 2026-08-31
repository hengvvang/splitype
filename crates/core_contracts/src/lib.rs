//! core_contracts — unified universal contracts, traits, and domain vocabulary
//! for the splitype editor family and application shell.

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
pub use outline::{
    OutlineHeading, OutlineHost, OutlineHudState, OutlineNode, render_floating_outline_hud,
};
pub use pane::{
    AutoscrollStrategy, PaneCapabilities, PaneDescriptor, PaneHost, PaneId, PaneKind,
    PaneOutlineHost, PaneRegistry, PaneRegistryError, PaneRenderContext, PaneView,
};
pub use panel::{
    DocumentPanel, PanelCapabilities, PanelDescriptor, PanelHost, PanelId, PanelKind,
    PanelRenderContext, PanelView,
};
pub use search::{
    RawMatch, SearchActiveField, SearchHost, SearchIme, SearchInputElement, SearchInputSnapshot,
    SearchMatch, SearchPanelState, SearchQuery, SearchScope, SearchStateView, SearchTextInput,
    ceil_char_boundary, compute_preserve_case_replacement, floor_char_boundary,
    render_search_panel_overlay,
};
