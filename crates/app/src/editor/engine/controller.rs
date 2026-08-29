//! Top-level editor controller.
//!
//! [`Editor`] aggregates the editor's own state: the runtime block tree
//! (`Document`), view mode, scroll state, focus deferral, undo, and the
//! editor's panes (preview, outline, source-code pane states). State is
//! grouped into cohesive sub-records (`file`, `focus`, `undo`, `scroll`,
//! `tables`,
//! `preview`, `references`, `menu_bar`, `overlays`) plus the session
//! aggregate defined in `super::session_ops` / `super::session`.

pub(crate) use std::time::{Duration, Instant};

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) use gpui::*;

pub(crate) use editor_core::EditorHost;
pub(crate) use workspace::DEFAULT_EDITOR_PANEL_ID;
pub(crate) use workspace::WindowPanelKind;
pub(crate) use editor_wysiwyg::document::protocol::UndoCaptureKind;
pub(crate) use editor_outline::OutlineHudState;
pub use crate::editor::engine::session::{
    EditorPaneKind, EditorSession, EditorTabList, OpenFileMode, TabKind,
};
pub(crate) use editor_wysiwyg::document::block::Block;
pub(crate) use editor_wysiwyg::document::Document;
pub(crate) use editor_wysiwyg::document::block::footnotes::FootnoteMap;
pub(crate) use editor_wysiwyg::state::{
    AutoscrollStrategy, BlockSelectionAnchor, CrossBlockDrag, CrossBlockSelection,
    CrossBlockSelectionEndpoint, EditorSelection, FocusState, HistoryEntry, PendingUndoCapture,
    ReferenceRegistries, SelectionState, SourceTargetMapping, TableAxisSelection,
    TableCellBinding, TableGrids, TableSizePickerState, UndoHistory, UndoSelectionSnapshot,
    WysiwygSelectAllCycle, EMPTY_FOCUS_STATE, EMPTY_SELECTION_STATE,
};
pub(crate) use crate::editor::panes::document_pane::context_menu::{ContextMenuState, FootnoteTooltipState};
pub(crate) use crate::editor::panes::document_pane::dialogs::TableInsertDialogState;
pub(crate) use editor_wysiwyg::markdown::block::image::parse_image_reference_definitions;
pub(crate) use editor_wysiwyg::markdown::block::link::parse_link_reference_definitions;
pub(crate) use editor_wysiwyg::markdown::block::table::TableCellPosition;
pub(crate) use editor_wysiwyg::markdown::block::table::{
    TableColumnAlignment, serialize_table_cell_markdown,
};
pub(crate) use editor_wysiwyg::markdown::inline::text::BlockText;
pub(crate) use editor_wysiwyg::markdown::parse::{BlockData, BlockKind};
pub(crate) use splitter::root::SplitterRoot;
pub use workspace::PanelId;

/// The strongly-typed identifier representing an inner tiled editor pane
/// (WYSIWYG, SourceCode, Preview). Contract type owned by the `editor`
/// crate.
pub use editor_core::PaneId;

/// Link navigation request deferred until a `Window` is available.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PendingOpenLink {
    pub(crate) prompt_target: String,
    pub(crate) open_target: String,
}

// ── Grouped editor state ───────────────────────────────────────────────────

/// File lifecycle: path, dirty tracking, save/close and drop-replace flows.
#[derive(Default)]
pub(crate) struct FileState {
    pub(crate) path: Option<PathBuf>,
    pub(crate) dirty: bool,
    pub(crate) pending_save: bool,
    pub(crate) pending_save_as: bool,
    pub(crate) pending_open_link: Option<PendingOpenLink>,
    pub(crate) pending_window_edited: bool,
    pub(crate) pending_window_title_refresh: bool,
    pub(crate) show_unsaved_changes_dialog: bool,
    pub(crate) pending_close_after_save: bool,
    pub(crate) close_dialog_restore_focus: Option<EntityId>,
    pub(crate) pending_drop_replace_path: Option<PathBuf>,
    pub(crate) show_drop_replace_dialog: bool,
    pub(crate) pending_drop_replace_after_save: bool,
    pub(crate) drop_replace_restore_focus: Option<EntityId>,
}

/// Scroll handle, layout anchoring, and autoscroll interaction state.
pub(crate) struct ScrollState {
    pub(crate) handle: ScrollHandle,
    pub(crate) pending_autoscroll: Option<AutoscrollStrategy>,
    pub(crate) last_viewport_size: Option<Size<Pixels>>,
    pub(crate) scrollbar_hovered: bool,
    pub(crate) scrollbar_visible_until: Instant,
    pub(crate) scrollbar_fade_task: Option<Task<()>>,
    pub(crate) scrollbar_drag: Option<ScrollbarDragSession>,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self {
            handle: ScrollHandle::new(),
            pending_autoscroll: None,
            last_viewport_size: None,
            scrollbar_hovered: false,
            scrollbar_visible_until: Instant::now(),
            scrollbar_fade_task: None,
            scrollbar_drag: None,
        }
    }
}

/// One document tab: the document and all of its document-level state.
///
/// A tab whose `file.path` is `None` is an untitled temporary document.
/// Switching tabs swaps the whole context, so undo history and per-pane
/// view states are preserved per file.
pub(crate) struct DocumentTab {
    pub(crate) document: Document,
    /// Bumped whenever the document text may have changed; derived views
    /// (preview, source panes) compare against this to skip re-syncing.
    pub(crate) document_revision: u64,
    pub(crate) file: FileState,
    pub(crate) kind: TabKind,
    pub(crate) undo: UndoHistory,
    pub(crate) references: ReferenceRegistries,
    pub(crate) tables: TableGrids,
    /// Per-pane view states, keyed by pane id. Every pane is fully
    /// independent — its own scroll position, focus target, selection, and
    /// preview — while all panes render the same shared `document`. Pane
    /// states travel with the tab, so each tab remembers where every pane
    /// was.
    pub(crate) panes: HashMap<PaneId, PaneState>,
    /// Cached (revision, word_count) to avoid full serialization on every status bar frame.
    pub(crate) cached_word_count: Option<(u64, usize)>,
}

/// The independent view state of one pane inside an editor area.
///
/// Panes share the tab's document (the single source of truth) but nothing
/// else: each Wysiwyg pane scrolls and focuses independently, each Source
/// pane owns its own raw text buffer, and each Preview pane keeps its own
/// rendered AST. This keeps the model simple — there is no state to
/// synchronize between panes because there is no shared view state.
pub(crate) struct PaneState {
    pub(crate) scroll: ScrollState,
    pub(crate) pane: Box<dyn editor_core::Pane>,
}


#[allow(dead_code)]
impl PaneState {
    pub(crate) fn new(kind: EditorPaneKind) -> Self {
        Self {
            scroll: ScrollState::default(),
            pane: new_pane_for_kind(kind),
        }
    }

    #[inline]
    pub(crate) fn kind(&self) -> EditorPaneKind {
        self.pane.kind()
    }

    pub(crate) fn ensure_kind(&mut self, kind: EditorPaneKind) {
        if self.kind() == kind {
            return;
        }
        self.pane = new_pane_for_kind(kind);
    }

    #[inline]
    pub(crate) fn as_wysiwyg(&self) -> Option<&editor_wysiwyg::WysiwygPaneState> {
        self.pane.as_any().downcast_ref()
    }

    #[inline]
    pub(crate) fn as_wysiwyg_mut(&mut self) -> Option<&mut editor_wysiwyg::WysiwygPaneState> {
        self.pane.as_any_mut().downcast_mut()
    }

    #[inline]
    pub(crate) fn as_source_code(&self) -> Option<&editor_source_code::SourceCodeState> {
        self.pane.as_any().downcast_ref()
    }

    #[inline]
    pub(crate) fn as_source_code_mut(
        &mut self,
    ) -> Option<&mut editor_source_code::SourceCodeState> {
        self.pane.as_any_mut().downcast_mut()
    }

    #[inline]
    pub(crate) fn as_preview(&self) -> Option<&crate::editor::panes::preview::PreviewState> {
        self.pane.as_any().downcast_ref()
    }

    #[inline]
    pub(crate) fn as_preview_mut(
        &mut self,
    ) -> Option<&mut crate::editor::panes::preview::PreviewState> {
        self.pane.as_any_mut().downcast_mut()
    }

    #[inline]
    pub(crate) fn selection(&self) -> Option<&SelectionState> {
        self.as_wysiwyg().map(|w| &w.selection)
    }

    #[inline]
    pub(crate) fn selection_mut(&mut self) -> Option<&mut SelectionState> {
        self.as_wysiwyg_mut().map(|w| &mut w.selection)
    }

    #[inline]
    pub(crate) fn focus(&self) -> Option<&FocusState> {
        self.as_wysiwyg().map(|w| &w.focus)
    }

    #[inline]
    pub(crate) fn focus_mut(&mut self) -> Option<&mut FocusState> {
        self.as_wysiwyg_mut().map(|w| &mut w.focus)
    }
}

/// Creates a fresh pane state for `kind` through the app-wide pane
/// factory registry (the composition root registers one factory per
/// mode kind at startup).
pub(crate) fn new_pane_for_kind(kind: EditorPaneKind) -> Box<dyn editor_core::Pane> {
    editor_core::PaneFactoryRegistry::global()
        .lock()
        .unwrap()
        .create(kind)
}

/// Top-level controller that owns editor-wide state and delegates tree
/// mutations to [`Document`].
///
/// The editor subscribes to every [`BlockEvent`](editor_wysiwyg::document::protocol::BlockEvent)
/// emitted by child blocks. Structural changes are handled centrally so focus,
/// scrolling, dirty tracking, and serialization stay synchronized. Documents
/// live in [`DocumentTab`]s, grouped per Editor area in the window layout:
/// every Editor area owns an independent tab bar, and window-level operations
/// (menus, chrome, explorer routing) target the ACTIVE editor — the last
/// Editor area that received focus.
pub struct Editor {
    /// The outer window panel this editor renders inside. An Editor entity
    /// serves exactly one area (Shell owns the area layout); window-level
    /// state such as the layout tree and sidebar panels lives on the Shell.
    pub(crate) panel_id: PanelId,
    /// The window-shell service this editor talks to. Used to request
    /// window-level operations (splitting an area creates a fresh Editor
    /// entity on the shell; closing one removes it). `None` in tests that
    /// create Editor-rooted windows directly.
    pub(crate) host: Option<Arc<dyn EditorHost>>,
    /// This editor panel's session: its document tabs and pane split
    /// root. One Editor entity owns exactly one session.
    pub(crate) session: EditorSession,
    /// The area's rectangle in window coordinates, pushed by the Shell on
    /// every layout change (the Shell owns the outer layout tree). Used by
    /// pane rendering and drag gestures to translate pointer
    /// positions into the area's local space.
    pub(crate) panel_rect: Option<Bounds<Pixels>>,
    /// Whether this area is the window's active editor (the target for
    /// explorer file opens). Pushed by the Shell alongside `panel_rect`.
    pub(crate) is_active_panel: bool,
    /// Whether this area's tile is maximized in the outer layout. Pushed
    /// by the Shell alongside `panel_rect`.
    pub(crate) is_maximized: bool,
    /// How many panel_contents the window's outer layout currently has; the top bar
    /// hides the maximize/close controls when only one area exists. Pushed
    /// by the Shell alongside `panel_rect`.
    pub(crate) leaf_count: usize,
    /// This editor's floating outline HUD state (heading list of its own active
    /// document). Synced during render from the active tab.
    pub(crate) outline: OutlineHudState,
    /// Rendered-mode context menu currently open in the editor.
    pub(crate) context_menu: Option<ContextMenuState>,
    pub(crate) context_menu_submenu_close_task: Option<Task<()>>,
    pub(crate) context_menu_submenu_close_token: usize,
    /// Footnote content tooltip shown while the pointer hovers a footnote
    /// reference or definition header.
    pub(crate) footnote_tooltip: Option<FootnoteTooltipState>,
    /// Table insertion dialog opened from the context menu.
    pub(crate) table_insert_dialog: Option<TableInsertDialogState>,
    /// Table size matrix picker opened from the bottom-right corner dot.
    pub(crate) table_size_picker: Option<TableSizePickerState>,
    /// Timestamp of the last welcome-prompt click, used to detect a
    /// double-click across repaints. GPUI rebuilds elements (and their
    /// closures) every frame, so the timestamp must live in editor state
    /// rather than in a click-handler closure.
    pub(crate) welcome_last_click: Option<Instant>,
    /// Currently focused pane id — the status-bar action target and the
    /// routing target for events without a pane context (block events,
    /// keyboard commands). One Editor entity serves one area, so the area
    /// (panel) id alone identifies it.
    pub(crate) focused_pane_id: Option<PaneId>,
    /// In-buffer and workspace search and replace state.
    pub(crate) search: crate::editor::search::SearchPanelState,
    /// Block entities this editor currently subscribes to (block events
    /// drive undo capture, preview refresh and structural requests).
    /// Blocks created outside `Editor::new_block` (e.g. by undo restore
    /// deltas in the WYSIWYG crate) are subscribed lazily via
    /// `subscribe_document_blocks` after the structure change.
    pub(crate) subscribed_blocks: HashSet<EntityId>,
}

/// Pixel geometry for the custom editor scrollbar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ScrollbarGeometry {
    pub(crate) track_height: f32,
    pub(crate) thumb_height: f32,
    pub(crate) thumb_top: f32,
    pub(crate) max_scroll_y: f32,
}

/// Active drag session for the custom scrollbar thumb.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ScrollbarDragSession {
    pub(crate) pointer_offset_y: f32,
    pub(crate) track_height: f32,
    pub(crate) thumb_height: f32,
    pub(crate) max_scroll_y: f32,
}


/// The informational dialogs that can be shown from the Help menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InfoDialogKind {
    /// Dialog describing update-check availability.
    CheckForUpdates,
    /// Dialog with app name and version information.
    About,
}

impl DocumentTab {
    #[inline]
    pub fn is_transient(&self) -> bool {
        self.kind == TabKind::Transient
    }

    #[inline]
    pub fn persist(&mut self) {
        self.kind = TabKind::Persistent;
    }
}

impl Editor {
    pub(crate) const HISTORY_LIMIT: usize = 200;
    pub(crate) const HISTORY_COALESCE_WINDOW: Duration = Duration::from_millis(1_000);
    pub(crate) const WYSIWYG_SELECT_ALL_CYCLE_WINDOW: Duration = Duration::from_millis(750);

    /// Creates an editor with no document tabs — the welcome state shown
    /// before any file is opened or an Untitled tab is started. The default
    /// layout seeds a left Explorer area and a right Editor area with an
    /// empty tab bar.
    pub(crate) fn empty(cx: &mut Context<Self>) -> Self {
        Self::empty_for_panel(PanelId(DEFAULT_EDITOR_PANEL_ID), cx)
    }

    /// Creates an editor serving `panel_id` with no document tabs. The
    /// Shell seeds each Editor area with its own entity via this
    /// constructor.
    pub(crate) fn empty_for_panel(panel_id: impl Into<PanelId>, _cx: &mut Context<Self>) -> Self {
        Self::with_session(panel_id, EditorSession::welcome(), _cx)
    }

    /// Creates an editor serving `panel_id` with the given session (tab
    /// list + pane split root). The Shell uses this to materialize
    /// split-off and restored editor panel_contents; `host` is wired in afterwards.
    pub(crate) fn with_session(
        panel_id: impl Into<PanelId>,
        session: EditorSession,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            panel_id: panel_id.into(),
            host: None,
            session,
            panel_rect: None,
            is_active_panel: false,
            is_maximized: false,
            leaf_count: 1,
            outline: OutlineHudState::default(),
            context_menu: None,
            context_menu_submenu_close_task: None,
            context_menu_submenu_close_token: 0,
            footnote_tooltip: None,
            table_insert_dialog: None,
            table_size_picker: None,
            welcome_last_click: None,
            focused_pane_id: None,
            search: crate::editor::search::SearchPanelState::new(cx),
            subscribed_blocks: HashSet::new(),
        }
    }

    /// True when the active editor has at least one document tab.
    pub(crate) fn has_active_tab(&self) -> bool {
        self.session.has_tabs()
    }

    pub fn from_markdown(
        cx: &mut Context<Self>,
        markdown: String,
        file_path: Option<PathBuf>,
    ) -> Self {
        let tab = Self::new_tab_from_markdown(cx, markdown, file_path);
        let mut editor = Self {
            panel_id: PanelId(DEFAULT_EDITOR_PANEL_ID),
            host: None,
            session: EditorSession::welcome(),
            panel_rect: None,
            is_active_panel: false,
            is_maximized: false,
            leaf_count: 1,
            outline: OutlineHudState::default(),
            context_menu: None,
            context_menu_submenu_close_task: None,
            context_menu_submenu_close_token: 0,
            footnote_tooltip: None,
            table_insert_dialog: None,
            table_size_picker: None,
            welcome_last_click: None,
            focused_pane_id: None,
            search: crate::editor::search::SearchPanelState::new(cx),
            subscribed_blocks: HashSet::new(),
        };
        editor.session.push_tab(tab);
        editor.rebuild_table_grids(cx);
        editor.rebuild_reference_registries(cx);
        let pane_id = editor.active_pane_id();
        editor.refresh_preview_blocks(pane_id, cx);
        let pending_focus = editor.first_focusable_entity_id(cx);
        let pane = editor.pane_state(pane_id);
        if let Some(w) = pane.as_wysiwyg_mut() {
            w.focus.pending = pending_focus;
            w.focus.active_entity = pending_focus;
        }
        editor.refresh_stable_document_snapshot(cx);
        editor
    }
}

impl Editor {
    /// Builds a document tab from raw Markdown and an optional file path.
    /// `file_path == None` produces an untitled temporary document.
    pub(crate) fn new_tab_from_markdown(
        cx: &mut Context<Self>,
        markdown: String,
        file_path: Option<PathBuf>,
    ) -> DocumentTab {
        let normalized = markdown.replace("\r\n", "\n").replace('\r', "\n");
        let mut roots = Self::parse_wysiwyg_document(cx, &normalized);
        if roots.is_empty() {
            roots.push(cx.new(|cx| {
                editor_wysiwyg::document::block::Block::with_data(
                    cx,
                    BlockData::paragraph(String::new()),
                )
            }));
        }

        let mut document = Document::new(roots);
        document.rebuild_metadata_and_snapshot(cx);

        let panes = HashMap::new();

        DocumentTab {
            document,
            document_revision: 0,
            file: FileState {
                path: file_path,
                ..FileState::default()
            },
            kind: TabKind::Persistent,
            undo: UndoHistory::default(),
            references: ReferenceRegistries::default(),
            tables: TableGrids::default(),
            panes,
            cached_word_count: None,
        }
    }

    /// Activates the tab at `index`, restoring its focus and window
    /// chrome.
    pub(crate) fn activate_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.session.tab_count() {
            return;
        }
        // Also reachable right after the first tab is pushed onto an empty
        // editor (welcome state) — notify so the new document renders.
        if index == self.session.active_tab_index() {
            self.rebuild_table_grids(cx);
            self.rebuild_reference_registries(cx);
            self.subscribe_document_blocks(cx);
            let pane_id = self.active_pane_id();
            self.refresh_preview_blocks(pane_id, cx);
            self.refresh_stable_document_snapshot(cx);
            cx.notify();
            return;
        }
        self.session.set_active_tab(index);
        let pane = self.active_pane_state();
        if let Some(w) = pane.as_wysiwyg_mut() {
            if w.focus.pending.is_none() {
                w.focus.pending = w.focus.active_entity;
            }
        }
        if let Some(tab) = self.session.tab_mut(index) {
            tab.file.pending_window_title_refresh = true;
            tab.file.pending_window_edited = true;
        }
        self.rebuild_table_grids(cx);
        self.rebuild_reference_registries(cx);
        let pane_id = self.active_pane_id();
        self.refresh_preview_blocks(pane_id, cx);
        self.refresh_stable_document_snapshot(cx);
        if self.search.visible {
            self.execute_search(cx);
        }
        cx.notify();
    }

    /// Opens a file in this editor's tab list: activates its tab if
    /// already open, otherwise loads a new tab from disk.
    ///
    /// If `mode == OpenFileMode::Transient`, reuses/replaces any existing
    /// non-dirty transient tab in this editor pane.
    pub(crate) fn open_file_in_panel(
        &mut self,
        path: &std::path::Path,
        mode: OpenFileMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let already_open = self
            .session
            .tab_list
            .iter()
            .position(|t| t.file.path.as_deref() == Some(path));
        if let Some(index) = already_open {
            if mode == OpenFileMode::Persistent {
                if let Some(tab) = self.session.tab_mut(index) {
                    tab.persist();
                }
            }
            self.activate_tab(index, cx);
            return;
        }

        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(_) => {
                self.show_drop_open_failed_prompt(
                    format!("failed to read '{}'", path.display()),
                    window,
                    cx,
                );
                return;
            }
        };
        let markdown = String::from_utf8_lossy(&bytes).to_string();
        let mut tab = Self::new_tab_from_markdown(
            cx,
            markdown,
            Some(path.to_path_buf()),
        );
        tab.kind = match mode {
            OpenFileMode::Transient => TabKind::Transient,
            OpenFileMode::Persistent => TabKind::Persistent,
        };

        if mode == OpenFileMode::Transient {
            let clean_transient_idx = self
                .session
                .tab_list
                .iter()
                .position(|t| t.is_transient() && !t.file.dirty);
            if let Some(idx) = clean_transient_idx {
                self.session.tab_list.replace(idx, tab);
                self.activate_tab(idx, cx);
                crate::app::menus::record_recent_file_from_editor(path, cx);
                return;
            }
        }

        let last = self.session.push_tab(tab);
        self.activate_tab(last, cx);
        crate::app::menus::record_recent_file_from_editor(path, cx);
    }

    /// Opens a file in the ACTIVE editor's tab bar, routed through the
    /// Shell (which owns the area layout). Returns `false` when no active
    /// Editor area exists (the caller decides how to handle that case).
    pub(crate) fn open_file_in_active_editor(
        &mut self,
        path: &std::path::Path,
        mode: OpenFileMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(host) = self.host.clone() else {
            return false;
        };
        host.open_file_in_active_editor(path, mode, window, cx)
    }

    /// Opens a fresh untitled tab in this editor.
    pub(crate) fn new_untitled_tab(&mut self, cx: &mut Context<Self>) {
        let last = self
            .session
            .tab_list
            .push(Self::new_tab_from_markdown(cx, String::new(), None));
        self.activate_tab(last, cx);
    }

    /// Requests to close the tab at `index`. If dirty, prompts for confirmation;
    /// otherwise closes immediately.
    pub(crate) fn request_close_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(tab) = self.session.tab(index) else {
            return;
        };
        if tab.file.dirty {
            let panel_id = self.panel_id;
            self.activate_tab(index, cx);
            self.defer_host_action(cx, move |host, cx| {
                host.prompt_close_tab(panel_id, index, cx);
            });
            return;
        }
        self.close_tab(index, cx);
    }

    /// Closes the tab at `index`, activating a neighbor. Closing the last
    /// tab leaves the editor back in the welcome state (no tabs).
    pub(crate) fn close_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.session.close_tab(index).is_none() {
            return;
        }
        if !self.session.has_tabs() {
            // Last tab: back to the welcome mode. The pane tree keeps its
            // kinds, so the layout is restored unchanged when editing
            // resumes.
            self.clear_search_highlights_from_document(cx);
            self.search.matches.clear();
            self.search.active_match_index = None;
            cx.notify();
            return;
        }
        let pane = self.active_pane_state();
        if let Some(w) = pane.as_wysiwyg_mut() {
            if w.focus.pending.is_none() {
                w.focus.pending = w.focus.active_entity;
            }
        }
        if let Some(tab) = self.session.active_tab_mut() {
            tab.file.pending_window_title_refresh = true;
            tab.file.pending_window_edited = true;
        }
        if self.search.visible {
            self.execute_search(cx);
        }
        cx.notify();
    }
}

impl Editor {
    // ------------------------------------------------------------------
    // Pane view state access
    // ------------------------------------------------------------------

    /// The active pane id: the explicitly focused pane, or the first pane
    /// of the layout tree before any focus was set. Events without a pane
    /// context (block events, keyboard commands) route here, because the
    /// window keyboard focus sits in exactly one pane at a time.
    pub(crate) fn active_pane_id(&self) -> PaneId {
        if let Some(pane_id) = self.focused_pane_id {
            return pane_id;
        }
        PaneId(self.session.root.tree.first_leaf_id().unwrap_or(0))
    }

    /// The active pane's view state, creating it lazily.
    pub(crate) fn active_pane_state(&mut self) -> &mut PaneState {
        let pane_id = self.active_pane_id();
        self.pane_state(pane_id)
    }

    /// The view state of the pane with `pane_id`, creating it lazily.
    pub(crate) fn pane_state(&mut self, pane_id: PaneId) -> &mut PaneState {
        let kind = self.pane_kind(pane_id).unwrap_or(EditorPaneKind::Wysiwyg);
        let tab = self.tab_mut();
        let state = tab.panes.entry(pane_id).or_insert_with(|| PaneState::new(kind));
        state.ensure_kind(kind);
        state
    }

    /// The view state of the pane with `pane_id`, creating it lazily if an active tab exists.
    pub(crate) fn pane_state_mut(&mut self, pane_id: PaneId) -> Option<&mut PaneState> {
        let kind = self.pane_kind(pane_id).unwrap_or(EditorPaneKind::Wysiwyg);
        let tab = self.session.active_tab_mut()?;
        let state = tab.panes.entry(pane_id).or_insert_with(|| PaneState::new(kind));
        state.ensure_kind(kind);
        Some(state)
    }

    /// The view state of the pane with `pane_id`, if it exists.
    pub(crate) fn pane_state_ref(&self, pane_id: PaneId) -> Option<&PaneState> {
        let tab = self.active_tab()?;
        tab.panes.get(&pane_id)
    }

    /// The active pane's focus state — the routing target for events
    /// without a pane context.
    pub(crate) fn active_pane_focus(&self) -> &FocusState {
        self.pane_state_ref(self.active_pane_id())
            .and_then(|p| p.as_wysiwyg())
            .map(|w| &w.focus)
            .unwrap_or(&EMPTY_FOCUS_STATE)
    }

    #[allow(dead_code)]
    pub(crate) fn active_pane_focus_mut(&mut self) -> Option<&mut FocusState> {
        let pane_id = self.active_pane_id();
        self.pane_state_mut(pane_id)
            .and_then(|p| p.as_wysiwyg_mut())
            .map(|w| &mut w.focus)
    }

    /// The active pane's selection state.
    pub(crate) fn active_pane_selection(&self) -> &SelectionState {
        self.pane_state_ref(self.active_pane_id())
            .and_then(|p| p.as_wysiwyg())
            .map(|w| &w.selection)
            .unwrap_or(&EMPTY_SELECTION_STATE)
    }

    #[allow(dead_code)]
    pub(crate) fn active_pane_selection_mut(&mut self) -> Option<&mut SelectionState> {
        let pane_id = self.active_pane_id();
        self.pane_state_mut(pane_id)
            .and_then(|p| p.as_wysiwyg_mut())
            .map(|w| &mut w.selection)
    }

    /// The active pane's scroll state.
    pub(crate) fn active_pane_scroll(&self) -> &ScrollState {
        &self
            .pane_state_ref(self.active_pane_id())
            .or_else(|| self.tab().panes.values().next())
            .expect("tab always has at least one pane state")
            .scroll
    }
}

impl Editor {
    /// Defers a window-shell service call until this editor update ends.
    ///
    /// Shell layout operations (`activate_panel`, `split_panel`,
    /// `close_panel`, …) re-push state to every editor entity via
    /// `sync_panel_states`. When such an operation is triggered from this
    /// editor's own handler, the editor is already mid-update and the
    /// re-push would double-lease it (gpui panics on nested
    /// `Entity::update`). Deferring lets the current update finish before
    /// the shell touches any editor; the pushed flags still land before
    /// the next frame renders.
    pub(crate) fn defer_host_action(
        &self,
        cx: &mut Context<Self>,
        action: impl FnOnce(&dyn EditorHost, &mut App) + 'static,
    ) {
        if let Some(host) = self.host.clone() {
            cx.defer(move |cx| {
                action(host.as_ref(), cx);
            });
        }
    }

    /// Select the pane at `pane_id` as the focused pane AND transfer the
    /// keyboard edit focus to that pane's editing target: a source pane
    /// focuses its own block, a Wysiwyg pane resumes editing the shared
    /// document at the last position. Preview pane only updates
    /// the bottombar focus.
    ///
    /// Two focus systems stay in sync here: `focused_pane_id`
    /// (bottombar target, set explicitly) and the gpui keyboard focus
    /// (input routing, moved to the pane's edit target). The keyboard
    /// focus is the single source of truth for *who edits*; the custom
    /// focus is its projection plus the explicit selection for panes
    /// without an edit target (Preview).
    pub(crate) fn focus_pane(
        &mut self,
        pane_id: impl Into<PaneId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pane_id = pane_id.into();
        self.focused_pane_id = Some(pane_id);
        self.session.root.activate_leaf(pane_id.0);
        self.session.root.clear_dropdowns();
        let panel_id = self.panel_id;
        self.defer_host_action(cx, move |host, cx| host.activate_panel(panel_id, cx));
        if !self.has_active_tab() {
            // Welcome mode: no document to focus yet; the pane is still
            // selected so the pane body click only marks the panel active.
            cx.notify();
            return;
        }
        {
            let kind = self.session.root.tree.find_leaf_kind(pane_id.0);
            match kind {
                // The source panel's own buffer becomes the edit target.
                Some(EditorPaneKind::SourceCode) => {
                    self.sync_source_pane(pane_id, cx);
                    if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                        if source.focus_handle.is_none() {
                            source.focus_handle = Some(cx.focus_handle());
                        }
                        if let Some(ref handle) = source.focus_handle {
                            handle.focus(window, cx);
                        }
                    }
                }
                // Resume editing the shared document at the last position
                // (falling back to the first block when it was rebuilt).
                Some(EditorPaneKind::Wysiwyg) => {
                    let target = self
                        .pane_state_ref(pane_id)
                        .and_then(|state| state.as_wysiwyg())
                        .and_then(|wysiwyg| wysiwyg.focus.active_entity)
                        .filter(|id| self.focusable_entity_by_id(*id).is_some())
                        .or_else(|| self.first_focusable_entity_id(cx));
                    if let Some(id) = target {
                        if let Some(block) = self.focusable_entity_by_id(id) {
                            let focus_handle = block.read(cx).focus_handle.clone();
                            focus_handle.focus(window, cx);
                        }
                    }
                }
                _ => {}
            }
        }
        cx.notify();
    }

    /// Query the pane kind of a specific pane.
    #[inline]
    pub fn pane_kind(&self, pane_id: PaneId) -> Option<EditorPaneKind> {
        self.session.root.tree.find_leaf(pane_id.0).map(|l| l.kind)
    }

    /// The active pane's kind.
    #[inline]
    pub fn active_pane_kind(&self) -> EditorPaneKind {
        self.pane_kind(self.active_pane_id())
            .unwrap_or(EditorPaneKind::Wysiwyg)
    }

    /// True if the active pane is in WYSIWYG mode.
    #[inline]
    pub fn is_wysiwyg(&self) -> bool {
        self.active_pane_kind().is_wysiwyg()
    }

    /// True if the active pane is in Source Code mode.
    #[inline]
    pub fn is_source_code(&self) -> bool {
        self.active_pane_kind().is_source_code()
    }

    /// True if the active pane is in Preview mode.
    #[inline]
    pub fn is_preview(&self) -> bool {
        self.active_pane_kind().is_preview()
    }

    #[inline]
    pub(crate) fn active_tab(&self) -> Option<&DocumentTab> {
        self.session.active_tab()
    }

    #[inline]
    pub(crate) fn active_tab_mut(&mut self) -> Option<&mut DocumentTab> {
        self.session.active_tab_mut()
    }

    #[inline]
    pub(crate) fn active_doc(&self) -> Option<&Document> {
        self.session.active_tab().map(|t| &t.document)
    }

    #[inline]
    pub(crate) fn tab(&self) -> &DocumentTab {
        self.session
            .active_tab()
            .expect("active tab requested on empty editor")
    }

    /// The active document tab, mutably.
    #[inline]
    pub(crate) fn tab_mut(&mut self) -> &mut DocumentTab {
        self.session
            .active_tab_mut()
            .expect("active tab mut requested on empty editor")
    }

    /// The active tab's document.
    pub(crate) fn doc(&self) -> &Document {
        &self.tab().document
    }

    /// The active tab's document, mutably.
    pub(crate) fn doc_mut(&mut self) -> &mut Document {
        &mut self.tab_mut().document
    }

    // ------------------------------------------------------------------
    // Tab access (one Editor entity serves one panel)
    // ------------------------------------------------------------------

    /// True when this editor has at least one document tab.
    #[inline]
    pub(crate) fn has_tabs(&self) -> bool {
        self.session.has_tabs()
    }

    /// This editor's tab list, mutably.
    pub(crate) fn tab_list_mut(&mut self) -> &mut EditorTabList<DocumentTab> {
        &mut self.session.tab_list
    }

    /// Split `panel_id` with a same-kind sibling and seed the new Editor
    /// area per `copy_content`: `true` deep-copies the source session
    /// (tab list re-materialized from its serialized document, pane
    /// layout cloned with fresh local ids); `false` leaves the new editor
    /// blank. Returns the new area's id.
    /// Deep-copy this editor's session for a fresh sibling area: the inner
    /// panel tree gets fresh ids from the area-local space (root leaf 1
    /// plus a fresh local pool), and every tab is re-materialized from its
    /// serialized document so the two editors are fully independent
    /// (separate undo, focus, scroll, and dirty state).
    pub(crate) fn clone_session(&self, cx: &mut Context<Self>) -> EditorSession {
        let mut root = SplitterRoot::single_leaf(1, EditorPaneKind::SourceCode);
        let mut next_id = 1;
        root.tree = self.session.root.tree.clone_with_new_ids(&mut next_id);
        root.next_node_id = next_id;

        let mut list = EditorTabList::new();
        for tab in self.session.tabs() {
            let text = tab.document.serialize_markdown(cx);
            let mut copy = Self::new_tab_from_markdown(cx, text, tab.file.path.clone());
            copy.file.dirty = tab.file.dirty;
            copy.kind = tab.kind;
            list.push(copy);
        }
        list.set_active_tab(self.session.active_tab_index());
        EditorSession {
            tab_list: list,
            root,
        }
    }

    /// First dirty tab in this editor, if any. Window-wide aggregation
    /// (across every editor area) lives on the Shell.
    pub(crate) fn first_dirty_tab(&self) -> Option<(PanelId, usize)> {
        for (index, tab) in self.session.tabs().enumerate() {
            if tab.file.dirty {
                return Some((self.panel_id, index));
            }
        }
        None
    }

    // ------------------------------------------------------------------
    // Drag-policy content steps (invoked by the host after a policy's
    // tree operation)
    // ------------------------------------------------------------------
}
