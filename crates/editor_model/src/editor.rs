//! editor_model — the editor family's contract and vocabulary layer.
//!
//! This crate owns the *contract* types every editor mode (WYSIWYG,
//! Source Code, Preview, and custom plugins) and every consumer (outline, search, export)
//! depends on — and nothing else: the pane-kind identifier, pane ids,
//! the session primitives (tab kinds, open modes), the outline node
//! type, the `PaneView` plugin trait, the `PaneHost` reverse seam, and the
//! [`EditorHost`] dependency-inversion seam to the window shell.
//!
//! Dependency direction: modes and consumers depend on `editor_model`;
//! `editor_model` depends on neither. The `Editor` entity lives in the
//! app composition root (ADR-01) and talks to the modes only through
//! these contracts and the reverse seams.

pub use gpui;

mod autoscroll;
mod pane_host;
mod pane_registry;
mod pane_type;
mod pane_view;

pub use autoscroll::AutoscrollStrategy;
pub use pane_host::{PaneHost, PaneRenderContext};
pub use pane_registry::{PaneDescriptor, PaneRegistry};
pub use pane_type::PaneKindId;
pub use pane_view::PaneView;

/// Backward compatibility alias for PaneKindId.
pub type EditorPaneKind = PaneKindId;

/// The strongly-typed identifier of one inner tiled editor pane.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PaneId(pub splitter::tree::NodeId);

impl From<splitter::tree::NodeId> for PaneId {
    #[inline]
    fn from(id: splitter::tree::NodeId) -> Self {
        Self(id)
    }
}

impl From<PaneId> for splitter::tree::NodeId {
    #[inline]
    fn from(id: PaneId) -> Self {
        id.0
    }
}

impl From<PaneId> for gpui::ElementId {
    #[inline]
    fn from(id: PaneId) -> Self {
        id.0.into()
    }
}

impl std::fmt::Display for PaneId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Lifecycle retention kind of a document tab in an editor pane.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TabKind {
    /// Transient temporary tab: replaced in-place when another file is clicked.
    #[default]
    Transient,
    /// Persistent resident tab: pinned to the tab bar until explicitly closed.
    Persistent,
}

/// Requested mode when opening a file into an editor pane.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum OpenFileMode {
    /// Open as transient tab (replaces existing clean transient tab if present).
    #[default]
    Transient,
    /// Open as persistent tab (or promotes existing tab to persistent).
    Persistent,
}

use std::path::Path;

use gpui::{App, Window};

/// Service contract between the editor family and the window shell
/// (dependency inversion seam).
pub trait EditorHost: Send + Sync + 'static {
    /// Bring the window panel `panel_id` to the foreground.
    fn activate_panel(&self, panel_id: workspace::PanelId, cx: &mut App);

    /// Toggle the window panel's kind dropdown (top bar control).
    fn toggle_panel_dropdown(&self, panel_id: workspace::PanelId, cx: &mut App);

    /// Split the window panel into two editor panels along `axis`.
    fn split_panel(
        &self,
        panel_id: workspace::PanelId,
        axis: splitter::SplitAxis,
        ratio: f32,
        copy_content: bool,
        cx: &mut App,
    );

    /// Maximize or restore the window panel.
    fn toggle_panel_maximize(&self, panel_id: workspace::PanelId, cx: &mut App);

    /// Request closing the window panel (runs the shell's dirty check).
    fn request_close_panel(&self, panel_id: workspace::PanelId, cx: &mut App);

    /// Prompt the shell's unsaved-changes dialog for one tab.
    fn prompt_close_tab(&self, panel_id: workspace::PanelId, index: usize, cx: &mut App);

    /// Open `path` in the active editor tab of the shell.
    fn open_file_in_active_editor(
        &self,
        path: &Path,
        mode: OpenFileMode,
        window: &mut Window,
        cx: &mut App,
    ) -> bool;

    /// Dismiss the shell's info dialog (drop-replace flow).
    fn hide_info_dialog(&self, cx: &mut App);

    /// Close window-level layout dropdowns opened by the shell.
    fn clear_outer_dropdowns(&self, cx: &mut App);

    /// Keep the explorer selection in sync after a document path change.
    fn sync_explorer_after_document_path_change(&self, cx: &mut App);

    /// Record a recently opened document path.
    fn record_recent_file(&self, path: &Path, cx: &mut App);
}

/// Minimal document view the editor modes may read.
pub trait EditorDocument {
    /// Serialize the active document to markdown.
    fn serialize_markdown(&self, cx: &App) -> String;
}
