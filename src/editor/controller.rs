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

pub(crate) use crate::app::shell::Shell;
pub(crate) use crate::app::window_panels::DEFAULT_EDITOR_PANEL_ID;
pub(crate) use crate::app::window_panels::WindowPanelKind;
pub(crate) use crate::editor::block_protocol::UndoCaptureKind;
pub(crate) use crate::editor::outline::state::OutlinePanelState;
pub(crate) use crate::editor::session::{EditorPaneKind, EditorSession, EditorTabList};
pub(crate) use crate::editor::tree::block::Block;
pub(crate) use crate::editor::tree::document::Document;
pub(crate) use crate::editor::tree::footnotes::{
    FootnoteDefinitionBinding, FootnoteMap, FootnoteReferenceLocation, FootnoteResolvedOccurrence,
};
pub(crate) use crate::editor::view::context_menu::ContextMenuState;
pub(crate) use crate::editor::view::dialogs::TableInsertDialogState;
pub(crate) use crate::editor::{PreviewState, SourceCodePaneState};
pub(crate) use crate::model::block::{BlockData, BlockId, BlockKind};
pub(crate) use crate::model::inline::text::BlockText;
pub(crate) use crate::model::syntax::image::{
    ImageReferenceDefinitions, parse_image_reference_definitions,
};
pub(crate) use crate::model::syntax::link::{
    LinkReferenceDefinitions, parse_link_reference_definitions,
};
pub(crate) use crate::model::syntax::table::TableCellPosition;
pub(crate) use crate::model::syntax::table::{
    TableAxisHighlight, TableAxisKind, TableAxisMarker, TableColumnAlignment, TableData,
    serialize_table_cell_markdown,
};
pub(crate) use crate::splitter::tree::NodeId;
pub(crate) use splitype_splitter::root::SplitterRoot;

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

/// Focus routing and deferred focus targets.
#[derive(Default)]
pub(crate) struct FocusState {
    pub(crate) pending: Option<EntityId>,
    pub(crate) active_entity: Option<EntityId>,
    pub(crate) pending_scroll_active_block_into_view: bool,
    pub(crate) pending_scroll_recheck_after_layout: bool,
}

/// Editor-level selection spanning rendered blocks.
#[derive(Default)]
pub(crate) struct SelectionState {
    pub(crate) cross_block: Option<CrossBlockSelection>,
    pub(crate) cross_block_drag: Option<CrossBlockDrag>,
    pub(crate) select_all_cycle: Option<RenderedSelectAllCycle>,
}

/// Undo/redo stacks, coalescing state, and stable source snapshots.
#[derive(Default)]
pub(crate) struct UndoHistory {
    pub(crate) undo_entries: Vec<HistoryEntry>,
    pub(crate) redo_entries: Vec<HistoryEntry>,
    pub(crate) pending_capture: Option<PendingUndoCapture>,
    pub(crate) last_selection_snapshot: UndoSelectionSnapshot,
    pub(crate) last_stable_source_text: String,
    pub(crate) restore_in_progress: bool,
}

/// Document-wide reference registries (images, links, footnotes).
#[derive(Default)]
pub(crate) struct ReferenceRegistries {
    pub(crate) image: Arc<ImageReferenceDefinitions>,
    pub(crate) link: Arc<LinkReferenceDefinitions>,
    pub(crate) footnotes: Arc<FootnoteMap>,
    /// Base directory the registries were last synced against; blocks
    /// re-resolve image sources whenever this changes.
    pub(crate) base_dir: Option<PathBuf>,
    /// Document structure version at the time every current block last
    /// received its reference context. A mismatch means blocks were added
    /// or replaced since, so the per-block sync cannot be skipped.
    pub(crate) synced_structure_version: u64,
    /// Blocks and table cells that could contribute reference definitions,
    /// footnote content, or standalone-image syntax, cached at the last full
    /// registry sync. A block edit outside this set cannot change the
    /// registries, so the per-keystroke rebuild is skipped.
    pub(crate) candidate_blocks: HashSet<EntityId>,
}

/// Native table cell bindings and axis selections.
#[derive(Default)]
pub(crate) struct TableGrids {
    pub(crate) cells: HashMap<EntityId, TableCellBinding>,
    pub(crate) axis_preview: Option<TableAxisSelection>,
    pub(crate) axis_selection: Option<TableAxisSelection>,
}

/// Scroll handle, row-footprint caches, and scrollbar interaction state.
pub(crate) struct ScrollState {
    pub(crate) handle: ScrollHandle,
    pub(crate) last_viewport_size: Option<Size<Pixels>>,
    /// Last frame.s mounted block ids, to detect structural edits so the height
    /// cache is refreshed only when the row/block mapping is unchanged.
    pub(crate) prev_block_ids: Vec<EntityId>,
    /// Per-row footprint (height plus trailing gap), keyed by the row's first
    /// block. Scroll-invariant, unlike raw painted positions, so windowing from
    /// their running sum stays correct as the document scrolls. Filled as rows
    /// paint; unknown rows use a minimum-height estimate.
    pub(crate) row_stride_cache: HashMap<EntityId, f32>,
    /// Row range mounted last frame; only those rows shared one scroll offset, so
    /// their adjacent-top differences are valid footprints for the cache.
    pub(crate) prev_row_band: Option<(usize, usize)>,
    pub(crate) scrollbar_hovered: bool,
    pub(crate) scrollbar_visible_until: Instant,
    pub(crate) scrollbar_fade_task: Option<Task<()>>,
    /// Forces a repaint shortly after a pending scroll-into-view that could
    /// not be satisfied yet (the target block has no measured bounds), so the
    /// scroll lands on the next frame instead of waiting for the cursor blink.
    pub(crate) scroll_recheck_task: Option<Task<()>>,
    pub(crate) scrollbar_drag: Option<ScrollbarDragSession>,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self {
            handle: ScrollHandle::new(),
            last_viewport_size: None,
            prev_block_ids: Vec::new(),
            row_stride_cache: HashMap::new(),
            prev_row_band: None,
            scrollbar_hovered: false,
            scrollbar_visible_until: Instant::now(),
            scrollbar_fade_task: None,
            scroll_recheck_task: None,
            scrollbar_drag: None,
        }
    }
}

/// One document tab: the document and all of its document-level state.
///
/// A tab whose `file.path` is `None` is an untitled temporary document.
/// Switching tabs swaps the whole context, so undo history, scroll
/// position, selection, and previews are preserved per file.
pub(crate) struct DocumentTab {
    pub(crate) document: Document,
    /// Bumped whenever the document text may have changed; derived views
    /// (preview, source panes) compare against this to skip re-syncing.
    pub(crate) document_revision: u64,
    /// Which view this tab is currently presenting.
    pub(crate) mode: EditorMode,
    pub(crate) file: FileState,
    pub(crate) focus: FocusState,
    pub(crate) selection: SelectionState,
    pub(crate) undo: UndoHistory,
    pub(crate) references: ReferenceRegistries,
    pub(crate) tables: TableGrids,
    pub(crate) preview: PreviewState,
    pub(crate) scroll: ScrollState,
}

/// Top-level controller that owns editor-wide state and delegates tree
/// mutations to [`Document`].
///
/// The editor subscribes to every [`BlockAction`](crate::editor::block_protocol::BlockAction)
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
    pub(crate) panel_id: NodeId,
    /// The Shell that owns this editor.s window panel. Used to request
    /// window-level operations (splitting an area creates a fresh Editor
    /// entity on the Shell; closing one removes it). None in tests that
    /// create Editor-rooted windows directly.
    pub(crate) shell: Option<WeakEntity<Shell>>,
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
    /// This editor's outline panel state (heading tree of its own active
    /// document). Synced during render from the active tab.
    pub(crate) outline: OutlinePanelState,
    /// Rendered-mode context menu currently open in the editor.
    pub(crate) context_menu: Option<ContextMenuState>,
    pub(crate) context_menu_submenu_close_task: Option<Task<()>>,
    /// Table insertion dialog opened from the context menu.
    pub(crate) table_insert_dialog: Option<TableInsertDialogState>,
    /// Timestamp of the last welcome-prompt click, used to detect a
    /// double-click across repaints. GPUI rebuilds elements (and their
    /// closures) every frame, so the timestamp must live in editor state
    /// rather than in a click-handler closure.
    pub(crate) welcome_last_click: Option<Instant>,
    /// Currently focused pane id — the status-bar action target.
    /// One Editor entity serves one area, so the area (panel) id alone
    /// identifies it.
    pub(crate) focused_pane: Option<usize>,
    /// Per-SourceCode-pane editing states, keyed by the pane id. Each
    /// source pane owns its own block entity so multiple source panels edit
    /// independently; see `SourceCodePaneState`.
    pub(crate) source_pane_states: HashMap<usize, SourceCodePaneState>,
}

/// Binding between a table block and one cell editor.
#[derive(Clone)]
pub(crate) struct TableCellBinding {
    pub(crate) table_block: Entity<Block>,
    pub(crate) cell: Entity<Block>,
    pub(crate) position: TableCellPosition,
}

/// Selected row or column in a rendered native table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TableAxisSelection {
    pub(crate) table_block_id: EntityId,
    pub(crate) kind: TableAxisKind,
    pub(crate) index: usize,
}

/// Pixel geometry for the custom editor scrollbar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ScrollbarGeometry {
    pub(crate) track_height: f32,
    pub(crate) thumb_height: f32,
    pub(crate) thumb_top: f32,
    pub(crate) max_scroll_y: f32,
}

/// Windowing result: the run of rows to mount, plus the top/bottom spacer
/// heights standing in for the culled rows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RowBand {
    pub(crate) run_start: usize,
    pub(crate) run_end: usize,
    pub(crate) top_h: f32,
    pub(crate) bottom_h: f32,
}

/// Active drag session for the custom scrollbar thumb.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ScrollbarDragSession {
    pub(crate) pointer_offset_y: f32,
    pub(crate) track_height: f32,
    pub(crate) thumb_height: f32,
    pub(crate) max_scroll_y: f32,
}

/// A block-local selection captured as a path through the block tree.
///
/// Undo restores rebuild every block entity, so the anchor addresses the
/// block structurally (root index + sibling index per level) instead of by
/// entity id. The range is the block's current (projected) content range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BlockSelectionAnchor {
    /// Root index followed by the sibling index of each child level.
    pub(crate) path: Vec<usize>,
    /// Current (projected) content range inside the anchored block.
    pub(crate) content_range: std::ops::Range<usize>,
}

/// Selection snapshot used by undo/redo to restore the caret.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct UndoSelectionSnapshot {
    /// Global source range. Only meaningful for cross-block selections and
    /// source-mode selections; block-local snapshots carry [`Self::block_anchor`]
    /// instead and leave this empty.
    pub(crate) range: std::ops::Range<usize>,
    pub(crate) reversed: bool,
    /// Block-local caret anchor, when the selection lives inside one block.
    pub(crate) block_anchor: Option<BlockSelectionAnchor>,
}

/// One undo history entry containing source text and selection state.
#[derive(Clone, Debug)]
pub(crate) struct HistoryEntry {
    pub(crate) source_text: String,
    pub(crate) selection: UndoSelectionSnapshot,
    pub(crate) timestamp: Instant,
    pub(crate) kind: UndoCaptureKind,
}

/// Deferred undo capture used to coalesce adjacent typing edits.
#[derive(Clone, Debug)]
pub(crate) struct PendingUndoCapture {
    pub(crate) snapshot: HistoryEntry,
}

/// Cross-block selection endpoint in visible block order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CrossBlockSelectionEndpoint {
    pub(crate) entity_id: EntityId,
    pub(crate) offset: usize,
}

/// Editor-level selection spanning two visible block endpoints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CrossBlockSelection {
    pub(crate) anchor: CrossBlockSelectionEndpoint,
    pub(crate) focus: CrossBlockSelectionEndpoint,
}

/// Drag state while creating or extending a cross-block selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CrossBlockDrag {
    pub(crate) anchor: CrossBlockSelectionEndpoint,
}

/// Short-lived Ctrl/Cmd+A press counter for rendered-mode selection upgrade.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RenderedSelectAllCycle {
    pub(crate) entity_id: EntityId,
    pub(crate) count: u8,
    pub(crate) last_pressed_at: Instant,
}

/// Mapping from one visible block's text range to canonical Markdown offsets.
#[derive(Clone)]
pub(crate) struct SourceTargetMapping {
    pub(crate) entity: Entity<Block>,
    pub(crate) full_source_range: std::ops::Range<usize>,
    pub(crate) content_to_source: Vec<usize>,
    pub(crate) source_to_content: Vec<usize>,
}

/// The two editing views the editor can present.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorMode {
    /// Rich rendered view where each block is styled by its semantic kind.
    Wysiwyg,
    /// Plain source view where the full Markdown document is edited as a
    /// single raw buffer.
    SourceCode,
}

/// The informational dialogs that can be shown from the Help menu.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum InfoDialogKind {
    /// Dialog describing update-check availability.
    CheckForUpdates,
    /// Dialog with app name and version information.
    About,
}

impl Editor {
    pub(crate) const HISTORY_LIMIT: usize = 200;
    pub(crate) const HISTORY_COALESCE_WINDOW: Duration = Duration::from_millis(1_000);
    pub(crate) const RENDERED_SELECT_ALL_CYCLE_WINDOW: Duration = Duration::from_millis(750);

    /// Creates an editor with no document tabs — the welcome state shown
    /// before any file is opened or an Untitled tab is started. The default
    /// layout seeds a left Explorer area and a right Editor area with an
    /// empty tab bar.
    pub(crate) fn empty(cx: &mut Context<Self>) -> Self {
        Self::empty_for_panel(DEFAULT_EDITOR_PANEL_ID, cx)
    }

    /// Creates an editor serving `panel_id` with no document tabs. The
    /// Shell seeds each Editor area with its own entity via this
    /// constructor.
    pub(crate) fn empty_for_panel(panel_id: NodeId, _cx: &mut Context<Self>) -> Self {
        Self::with_session(panel_id, EditorSession::welcome(), _cx)
    }

    /// Creates an editor serving `panel_id` with the given session (tab
    /// list + pane split root). The Shell uses this to materialize
    /// split-off and restored editor panel_contents; `shell` is wired in afterwards.
    pub(crate) fn with_session(
        panel_id: NodeId,
        session: EditorSession,
        _cx: &mut Context<Self>,
    ) -> Self {
        let this = Self {
            panel_id,
            shell: None,
            session,
            panel_rect: None,
            is_active_panel: false,
            is_maximized: false,
            leaf_count: 1,
            outline: OutlinePanelState::default(),
            context_menu: None,
            context_menu_submenu_close_task: None,
            table_insert_dialog: None,
            welcome_last_click: None,
            focused_pane: None,
            source_pane_states: HashMap::new(),
        };
        this
    }

    /// True when the active editor has at least one document tab.
    pub(crate) fn has_active_tab(&self) -> bool {
        !self.session.tab_list.tabs.is_empty()
    }

    pub fn from_markdown(
        cx: &mut Context<Self>,
        markdown: String,
        file_path: Option<PathBuf>,
    ) -> Self {
        let tab = Self::new_tab_from_markdown(cx, markdown, file_path);
        let mut editor = Self {
            panel_id: DEFAULT_EDITOR_PANEL_ID,
            shell: None,
            session: EditorSession::welcome(),
            panel_rect: None,
            is_active_panel: false,
            is_maximized: false,
            leaf_count: 1,
            outline: OutlinePanelState::default(),
            context_menu: None,
            context_menu_submenu_close_task: None,
            table_insert_dialog: None,
            welcome_last_click: None,
            focused_pane: None,
            source_pane_states: HashMap::new(),
        };
        editor.session.tab_list.tabs.push(tab);
        editor.rebuild_table_grids(cx);
        editor.rebuild_reference_registries(cx);
        editor.refresh_preview_blocks(cx);
        editor.tab_mut().focus.pending = editor.first_focusable_entity_id(cx);
        editor.tab_mut().focus.active_entity = editor.tab().focus.pending;
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
        let mut roots = Self::parse_document(cx, &normalized);
        if roots.is_empty() {
            roots.push(Self::new_block(cx, BlockData::paragraph(String::new())));
        }

        let mut document = Document::new(roots);
        document.rebuild_metadata_and_snapshot(cx);
        let pending_focus = document.first_root().map(|block| block.entity_id());

        DocumentTab {
            document,
            document_revision: 0,
            mode: EditorMode::Wysiwyg,
            file: FileState {
                path: file_path,
                ..FileState::default()
            },
            focus: FocusState {
                pending: pending_focus,
                active_entity: pending_focus,
                pending_scroll_active_block_into_view: true,
                pending_scroll_recheck_after_layout: true,
            },
            selection: SelectionState::default(),
            undo: UndoHistory {
                last_stable_source_text: normalized,
                ..UndoHistory::default()
            },
            references: ReferenceRegistries::default(),
            tables: TableGrids::default(),
            preview: PreviewState::default(),
            scroll: ScrollState::default(),
        }
    }

    /// Activates the tab at `index`, restoring its focus and window
    /// chrome.
    pub(crate) fn activate_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        let list = &mut self.session.tab_list;
        if index >= list.tabs.len() {
            return;
        }
        // Also reachable right after the first tab is pushed onto an empty
        // editor (welcome state) — notify so the new document renders.
        if index == list.active_tab {
            cx.notify();
            return;
        }
        list.active_tab = index;
        let tab = &mut list.tabs[index];
        if tab.focus.pending.is_none() {
            tab.focus.pending = tab.focus.active_entity;
        }
        tab.file.pending_window_title_refresh = true;
        tab.file.pending_window_edited = true;
        cx.notify();
    }

    /// Opens a file in this editor's tab list: activates its tab if
    /// already open, otherwise loads a new tab from disk.
    pub(crate) fn open_file_in_panel(
        &mut self,
        path: &std::path::Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let already_open = self
            .session
            .tab_list
            .tabs
            .iter()
            .position(|t| t.file.path.as_deref() == Some(path));
        if let Some(index) = already_open {
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
        let list = &mut self.session.tab_list;
        list.tabs.push(Self::new_tab_from_markdown(
            cx,
            markdown,
            Some(path.to_path_buf()),
        ));
        let last = list.tabs.len() - 1;
        self.activate_tab(last, cx);
        crate::app::menus::record_recent_file_from_editor(path, cx);
    }

    /// Opens a file in the ACTIVE editor's tab bar, routed through the
    /// Shell (which owns the area layout). Returns `false` when no active
    /// Editor area exists (the caller decides how to handle that case).
    pub(crate) fn open_file_in_active_editor(
        &mut self,
        path: &std::path::Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(shell) = self.shell.clone() else {
            return false;
        };
        shell
            .update(cx, |shell, cx| {
                shell.open_file_in_active_editor(path, window, cx)
            })
            .unwrap_or(false)
    }

    /// Opens a fresh untitled tab in this editor.
    pub(crate) fn new_untitled_tab(&mut self, cx: &mut Context<Self>) {
        let list = &mut self.session.tab_list;
        list.tabs
            .push(Self::new_tab_from_markdown(cx, String::new(), None));
        let last = list.tabs.len() - 1;
        self.activate_tab(last, cx);
    }

    /// Closes the tab at `index`, activating a neighbor. Closing the last
    /// tab leaves the editor back in the welcome state (no tabs).
    pub(crate) fn close_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        let list = &mut self.session.tab_list;
        if index >= list.tabs.len() {
            return;
        }
        let was_active = index == list.active_tab;
        list.tabs.remove(index);
        if list.tabs.is_empty() {
            list.active_tab = 0;
            // Last tab: back to the welcome mode. The pane tree keeps its
            // kinds, so the layout is restored unchanged when editing
            // resumes.
            cx.notify();
            return;
        }
        if was_active {
            list.active_tab = index.min(list.tabs.len() - 1);
            let tab = &mut list.tabs[list.active_tab];
            if tab.focus.pending.is_none() {
                tab.focus.pending = tab.focus.active_entity;
            }
            tab.file.pending_window_title_refresh = true;
            tab.file.pending_window_edited = true;
        } else if index < list.active_tab {
            list.active_tab -= 1;
        }
        cx.notify();
    }
}

impl Editor {
    /// Runs `action` on the Shell after the current update cycle ends.
    ///
    /// Shell layout operations (`activate_panel`, `split_panel`,
    /// `close_panel`, …) re-push state to every editor entity via
    /// `sync_panel_states`. When such an operation is triggered from this
    /// editor's own handler, the editor is already mid-update and the
    /// re-push would double-lease it (gpui panics on nested
    /// `Entity::update`). Deferring lets the current update finish before
    /// the Shell touches any editor; the pushed flags still land before
    /// the next frame renders.
    pub(crate) fn defer_shell_action(
        &self,
        cx: &mut Context<Self>,
        action: impl FnOnce(&mut Shell, &mut Context<Shell>) + 'static,
    ) {
        if let Some(shell) = self.shell.clone() {
            cx.defer(move |cx| {
                let _ = shell.update(cx, action);
            });
        }
    }

    /// Select the pane at `pane_id` as the focused pane AND transfer the
    /// keyboard edit focus to that pane's editing target: a source pane
    /// focuses its own block, a Wysiwyg pane resumes editing the shared
    /// document at the last position. Preview / Outline panes only update
    /// the bottombar focus.
    ///
    /// Two focus systems stay in sync here: `focused_pane`
    /// (bottombar target, set explicitly) and the gpui keyboard focus
    /// (input routing, moved to the pane's edit target). The keyboard
    /// focus is the single source of truth for *who edits*; the custom
    /// focus is its projection plus the explicit selection for panes
    /// without an edit target (Preview / Outline).
    pub(crate) fn focus_pane(
        &mut self,
        pane_id: NodeId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focused_pane = Some(pane_id);
        let panel_id = self.panel_id;
        self.defer_shell_action(cx, move |shell, cx| shell.activate_panel(panel_id, cx));
        if !self.has_active_tab() {
            // Welcome mode: no document to focus yet; the pane is still
            // selected so the pane body click only marks the panel active.
            cx.notify();
            return;
        }
        {
            let kind = self.session.root.tree.find_leaf_kind(pane_id);
            match kind {
                // The source panel's own block becomes the edit target.
                Some(EditorPaneKind::SourceCode) => {
                    self.sync_source_pane(pane_id, cx);
                    if let Some(block) = self
                        .source_pane_states
                        .get(&pane_id)
                        .and_then(|state| state.block.clone())
                    {
                        block.read(cx).focus_handle.focus(window);
                    }
                }
                // Resume editing the shared document at the last position
                // (falling back to the first block when it was rebuilt).
                Some(EditorPaneKind::Wysiwyg) => {
                    let target = self
                        .tab()
                        .focus
                        .active_entity
                        .filter(|id| self.focusable_entity_by_id(*id).is_some())
                        .or_else(|| self.first_focusable_entity_id(cx));
                    if let Some(id) = target {
                        if let Some(block) = self.focusable_entity_by_id(id) {
                            block.read(cx).focus_handle.focus(window);
                        }
                    }
                }
                _ => {}
            }
        }
        cx.notify();
    }

    /// The active document tab.
    pub(crate) fn tab(&self) -> &DocumentTab {
        let list = &self.session.tab_list;
        &list.tabs[list.active_tab]
    }

    /// The active document tab, mutably.
    pub(crate) fn tab_mut(&mut self) -> &mut DocumentTab {
        let index = self.session.tab_list.active_tab;
        &mut self.session.tab_list.tabs[index]
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
    pub(crate) fn has_tabs(&self) -> bool {
        !self.session.tab_list.tabs.is_empty()
    }

    /// This editor's tab list, mutably.
    pub(crate) fn tab_list_mut(&mut self) -> &mut EditorTabList<DocumentTab> {
        &mut self.session.tab_list
    }

    /// The active editor's tab, if the editor has tabs.
    pub(crate) fn active_editor_tab(&self) -> Option<&DocumentTab> {
        let list = &self.session.tab_list;
        list.tabs.get(list.active_tab)
    }

    /// The active editor's serialized document text, if any.
    pub(crate) fn active_editor_serialized_text(&self, cx: &App) -> Option<String> {
        let tab = self.active_editor_tab()?;
        Some(if tab.mode == EditorMode::SourceCode {
            tab.document.serialize_source_text(cx)
        } else {
            tab.document.serialize_markdown(cx)
        })
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

        let mut list = EditorTabList {
            tabs: Vec::with_capacity(self.session.tab_list.tabs.len()),
            active_tab: 0,
        };
        for tab in &self.session.tab_list.tabs {
            let text = if tab.mode == EditorMode::SourceCode {
                tab.document.serialize_source_text(cx)
            } else {
                tab.document.serialize_markdown(cx)
            };
            let mut copy = Self::new_tab_from_markdown(cx, text, tab.file.path.clone());
            copy.mode = tab.mode;
            copy.file.dirty = tab.file.dirty;
            list.tabs.push(copy);
        }
        list.active_tab = self
            .session
            .tab_list
            .active_tab
            .min(list.tabs.len().saturating_sub(1));
        EditorSession {
            tab_list: list,
            root,
        }
    }

    /// First dirty tab in this editor, if any. Window-wide aggregation
    /// (across every editor area) lives on the Shell.
    pub(crate) fn first_dirty_tab(&self) -> Option<(NodeId, usize)> {
        for (index, tab) in self.session.tab_list.tabs.iter().enumerate() {
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
