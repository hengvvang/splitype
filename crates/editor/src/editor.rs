//! editor — the thin coordination layer of the editor family.
//!
//! This crate owns the *contract* types every editor mode (WYSIWYG,
//! Source Code, Preview) and every consumer (outline, search, export)
//! depends on — and nothing else, until the `Editor` entity converges
//! here: the pane-kind vocabulary, pane ids, the session primitives
//! (tab kinds, open modes), the outline node type, and the
//! [`EditorHost`] dependency-inversion seam to the window shell.
//!
//! Dependency direction: modes and consumers depend on `editor`;
//! `editor` depends on neither. Modes implement [`Pane`] (defined here
//! once `Editor` moves in) and register themselves through the pane
//! factory registry; the app composition root wires everything.

pub use gpui;

/// The pane kinds an Editor panel can host: the document views
/// inside its split tree. The tree holds only real views — the welcome
/// state is the area's mode, not a panel kind — so the split structure
/// survives tab open/close cycles unchanged and the remembered panel
/// layout needs no migration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorPaneKind {
    /// Raw Markdown source code editor.
    SourceCode,
    /// Visual block editor (WYSIWYG rendered view).
    Wysiwyg,
    /// Read-only rendered Markdown preview.
    Preview,
}

impl EditorPaneKind {
    #[inline]
    pub fn is_wysiwyg(&self) -> bool {
        matches!(self, Self::Wysiwyg)
    }

    #[inline]
    pub fn is_source_code(&self) -> bool {
        matches!(self, Self::SourceCode)
    }

    #[inline]
    pub fn is_preview(&self) -> bool {
        matches!(self, Self::Preview)
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::SourceCode => "Source Code",
            Self::Wysiwyg => "Wysiwyg",
            Self::Preview => "Preview",
        }
    }

    /// All editor pane types (status-bar dropdown options).
    pub fn all() -> &'static [EditorPaneKind] {
        &[Self::Wysiwyg, Self::Preview, Self::SourceCode]
    }
}

/// The strongly-typed identifier of one inner tiled editor pane
/// (WYSIWYG, Source Code, Preview).
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

/// A heading node in the outline HUD (pure data; owned by `editor` so both
/// the outline panel and the modes can name it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineNode {
    pub id: String,
    pub label: String,
    pub level: u8,
    pub block_index: usize,
    pub block_id: Option<gpui::EntityId>,
}

use std::path::Path;

use gpui::{App, Window};

/// Service contract between the editor family and the window shell
/// (dependency inversion seam).
///
/// The editor family never names the shell type: the `Editor` entity
/// holds an `Arc<dyn EditorHost>` and the app's composition root injects
/// a `ShellEditorHost` (defined next to `Shell`) when it spawns editor
/// entities. Every shell-side capability the editor needs goes through
/// this trait, so the editor crates depend on nothing above them in the
/// dependency graph and can be exercised with a no-op host in tests.
/// Window-scoped work that must run after an editor update finishes is
/// deferred by the editor; the host itself never re-enters the editor.
///
/// All methods take `&mut App` (never a shell context) so implementations
/// can be invoked from deferred app callbacks without naming the shell
/// type. Methods that need a `Window` receive it as an argument.
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

    /// Keep the explorer selection in sync after a document path change
    /// (the explorer is a sibling panel; the editor must not name it).
    fn sync_explorer_after_document_path_change(&self, cx: &mut App);
}
