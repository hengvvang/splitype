//! editor_contracts — contracts and domain vocabulary of the document
//! editing ecosystem: the editor panel role, the pane SPI, and the
//! document/search/outline contracts shared with pane plugins.
//!
//! Platform-level contracts (panels, plugins, commands, shell actions, and
//! the cross-cutting `DocumentId`) live in `platform_contracts`, which has
//! zero knowledge of document editing. This crate extends that vocabulary
//! one-way: `DocumentPanel` refines `PanelView`, so `editor_contracts`
//! depends on `platform_contracts` while the reverse never happens.
//! Document plugins import each type from its owning crate (`DocumentId` is
//! re-exported here for convenience but owned by `platform_contracts`).

pub mod document;
pub mod edit;
pub mod export;
pub mod highlight;
pub mod outline;
pub mod pane;
pub mod panel;
pub mod search;

pub use document::{DocumentHost, DocumentId, DocumentSnapshot, TabKind};
pub use edit::{CursorHint, EditTransaction};
pub use export::ExportFormat;
pub use highlight::{CodeHighlightClass, CodeHighlightSpan, HighlightSnapshot};
pub use outline::{OutlineHost, OutlineHudState, OutlineNode};
pub use pane::{
    PaneCapabilities, PaneDescriptor, PaneHost, PaneId, PaneKind, PaneOutlineHost, PaneRegistry,
    PaneRegistryError, PaneRenderContext, PaneView,
};
pub use panel::DocumentPanel;
pub use rope::Rope;
pub use search::{
    SearchActiveField, SearchHost, SearchIme, SearchInputSnapshot, SearchMatch, SearchPanelState,
    SearchQuery, SearchScope, SearchStateView, SearchTextInput, compute_preserve_case_replacement,
};
