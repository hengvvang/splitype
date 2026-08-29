//! editor_wysiwyg — core mode 1: the complete Markdown editing world
//! (WYSIWYG rendered view).
//!
//! Owns the runtime block tree ([`document`]), its history deltas, the
//! text projection layer, native-table grid data, text shaping/measuring
//! helpers, and — as the mode crates converge — the pane state, input
//! routing, rendering and presentation. Stage 3h folds the dissolved
//! `markdown`/`sum_tree`/`latex`/`mermaid`/`export` crates (and the
//! WYSIWYG half of `syntax`) in here.
//!
//! The pane state implements [`editor::Pane`]; nothing in this crate
//! depends on the `Editor` entity or on `editor_source_code` (dual-core
//! zero sharing, D14-B).

pub mod actions;
pub mod code_language;
pub mod highlight;
pub mod document;
pub mod markdown;
pub mod tree;
pub mod latex;
pub mod mermaid;
pub mod export;
pub mod history;
pub mod pane;
pub mod presentation;
pub mod paste_plain;
pub mod render;
pub mod projection;
pub mod table_grid;
pub mod text_layout;
pub mod table_measure;

pub mod state;

pub use pane::WysiwygPaneState;

pub use state::{
    AutoscrollStrategy, BlockSelectionAnchor, CrossBlockDrag, CrossBlockSelection,
    CrossBlockSelectionEndpoint, EditorSelection, FocusState, HistoryEntry, PendingUndoCapture,
    ReferenceRegistries, SelectionState, SourceTargetMapping, TableAxisSelection,
    TableCellBinding, TableGrids, TableSizePickerState, UndoHistory, UndoSelectionSnapshot,
    WysiwygSelectAllCycle, EMPTY_FOCUS_STATE, EMPTY_SELECTION_STATE,
};
