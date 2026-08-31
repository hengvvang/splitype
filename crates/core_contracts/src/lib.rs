//! core_contracts — unified universal contracts, traits, and domain vocabulary
//! for the splitype editor family and application shell.

pub mod document;
pub mod export;
pub mod outline;
pub mod pane;
pub mod search;

pub use document::{
    EditorHost, NavigationExecutionPlan, NavigationTarget, TabKind,
};
pub use export::{ExportError, ExportFormat};
pub use outline::{
    OutlineHeading, OutlineHost, OutlineHudState, OutlineNode, render_floating_outline_hud,
};
pub use pane::{
    AutoscrollStrategy, PaneDescriptor, PaneHost, PaneId, PaneKind, PaneOutlineHost,
    PaneRegistry, PaneRenderContext, PaneView,
};
pub use search::{
    ceil_char_boundary, compute_preserve_case_replacement, floor_char_boundary, RawMatch,
    SearchActiveField, SearchHost, SearchIme, SearchInputElement, SearchInputSnapshot,
    SearchMatch, SearchPanelState, SearchQuery, SearchScope, SearchStateView, SearchTextInput,
    render_search_panel_overlay,
};
