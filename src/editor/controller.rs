//! Top-level editor controller and window state.
//!
//! [`Editor`] owns window-level concerns such as view mode, save/close flow,
//! scroll state, and focus deferral. The runtime block tree itself lives in
//! [`Document`], which centralizes structural mutations and cached visible
//! order metadata. State is grouped into cohesive sub-records (`file`,
//! `focus`, `undo`, `scroll`, `tables`, `preview`, `references`) plus the
//! chrome and panel state defined in `super::chrome` / `super::panels`.

pub(crate) use std::time::{Duration, Instant};

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) use gpui::*;

pub(crate) use crate::editor::block_protocol::UndoCaptureKind;
pub(crate) use crate::editor::panels::{PreviewState, SourcePanelState};
pub(crate) use crate::editor::tree::block::Block;
pub(crate) use crate::editor::tree::document::Document;
pub(crate) use crate::editor::tree::footnotes::{
    FootnoteDefinitionBinding, FootnoteMap, FootnoteReferenceLocation, FootnoteResolvedOccurrence,
};
pub(crate) use crate::layout::state::{EditorTabList, ROOT_AREA_ID};
pub(crate) use crate::layout::types::{AreaId, AreaSplitMode, EditorAreaMode};
pub(crate) use crate::model::block::{BlockData, BlockId, BlockKind};
pub(crate) use crate::model::inline::text::RichText;
pub(crate) use crate::model::syntax::image::{
    parse_image_reference_definitions, ImageReferenceDefinitions,
};
pub(crate) use crate::model::syntax::link::{
    parse_link_reference_definitions, LinkReferenceDefinitions,
};
pub(crate) use crate::model::syntax::table::TableCellPosition;
pub(crate) use crate::model::syntax::table::{
    serialize_table_cell_markdown, TableAxisHighlight, TableAxisKind, TableAxisMarker,
    TableColumnAlignment, TableData,
};
pub(crate) use crate::windows::editor::chrome::WindowChrome;
pub(crate) use crate::windows::layout::WindowPanels;

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
    pub(crate) close_guard_installed: bool,
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
}

/// Native table cell bindings and axis selections.
#[derive(Default)]
pub(crate) struct TableRuntimes {
    pub(crate) cells: HashMap<EntityId, TableCellBinding>,
    pub(crate) axis_preview: Option<TableAxisSelection>,
    pub(crate) axis_selection: Option<TableAxisSelection>,
}

/// Scroll handle, row-footprint caches, and scrollbar interaction state.
pub(crate) struct ScrollState {
    pub(crate) handle: ScrollHandle,
    pub(crate) last_viewport_size: Option<Size<Pixels>>,
    /// Last frame's visible block ids, to detect structural edits so the height
    /// cache is refreshed only when the row/block mapping is unchanged.
    pub(crate) prev_visible_block_ids: Vec<EntityId>,
    /// Per-row footprint (height plus trailing gap), keyed by the row's first
    /// block. Scroll-invariant, unlike raw painted positions, so windowing from
    /// their running sum stays correct as the document scrolls. Filled as rows
    /// paint; unknown rows use a minimum-height estimate.
    pub(crate) row_stride_cache: HashMap<EntityId, f32>,
    /// Row range mounted last frame; only those rows shared one scroll offset, so
    /// their adjacent-top differences are valid footprints for the cache.
    pub(crate) prev_render_window: Option<(usize, usize)>,
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
            prev_visible_block_ids: Vec::new(),
            row_stride_cache: HashMap::new(),
            prev_render_window: None,
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
    /// Which view this tab is currently presenting.
    pub(crate) mode: EditorMode,
    pub(crate) file: FileState,
    pub(crate) focus: FocusState,
    pub(crate) selection: SelectionState,
    pub(crate) undo: UndoHistory,
    pub(crate) references: ReferenceRegistries,
    pub(crate) tables: TableRuntimes,
    pub(crate) preview: PreviewState,
    pub(crate) source_panel: SourcePanelState,
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
    /// Transient routing hint: the Editor area whose tab list `tab()` /
    /// `doc()` currently resolve to. Set by per-area render/event handlers
    /// and cleared by window-level entry points, which then resolve to the
    /// active editor.
    pub(crate) current_tab_area: Option<AreaId>,
    pub(crate) chrome: WindowChrome,
    pub(crate) panels: WindowPanels,
}

/// Runtime binding between a table block and one cell editor.
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
pub(crate) struct RenderWindow {
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

/// Source-mode selection snapshot stored with undo history.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct UndoSelectionSnapshot {
    pub(crate) range: std::ops::Range<usize>,
    pub(crate) reversed: bool,
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
    /// layout seeds one root Editor area with an empty tab bar.
    pub fn empty(_cx: &mut Context<Self>) -> Self {
        Self {
            current_tab_area: None,
            chrome: WindowChrome::default(),
            panels: WindowPanels::default(),
        }
        .with_seeded_root_editor()
    }

    /// Seed the root Editor area as the (empty) active editor.
    fn with_seeded_root_editor(mut self) -> Self {
        self.panels.layout.ensure_editor_session(ROOT_AREA_ID);
        self.panels.layout.activate_editor_area(ROOT_AREA_ID);
        self
    }

    /// True when the active editor has at least one document tab.
    pub(crate) fn has_active_tab(&self) -> bool {
        self.panels
            .layout
            .active_editor_session()
            .is_some_and(|session| !session.tab_list.tabs.is_empty())
    }

    pub fn from_markdown(
        cx: &mut Context<Self>,
        markdown: String,
        file_path: Option<PathBuf>,
    ) -> Self {
        let tab = Self::new_tab_from_markdown(cx, markdown, file_path);
        let mut editor = Self {
            current_tab_area: None,
            chrome: WindowChrome::default(),
            panels: WindowPanels::default(),
        };
        // Seed the root Editor area with the initial tab, migrating the
        // default welcome panel into its editing panel.
        editor
            .panels
            .layout
            .ensure_editor_session(ROOT_AREA_ID)
            .tab_list
            .tabs
            .push(tab);
        editor.panels.layout.enter_editing(ROOT_AREA_ID);
        editor.panels.layout.activate_editor_area(ROOT_AREA_ID);
        editor.rebuild_table_runtimes(cx);
        editor.rebuild_image_runtimes(cx);
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
            tables: TableRuntimes::default(),
            preview: PreviewState::default(),
            source_panel: SourcePanelState::default(),
            scroll: ScrollState::default(),
        }
    }

    /// Activates the tab at `index` in the given Editor area, restoring
    /// its focus and window chrome.
    pub(crate) fn activate_tab(&mut self, area_id: AreaId, index: usize, cx: &mut Context<Self>) {
        let list = &mut self.panels.layout.ensure_editor_session(area_id).tab_list;
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

    /// Opens a file in the given Editor area: activates its tab if already
    /// open, otherwise loads a new tab from disk.
    pub(crate) fn open_file_in_area(
        &mut self,
        area_id: AreaId,
        path: &std::path::Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let already_open = self
            .panels
            .layout
            .editor_session(area_id)
            .and_then(|session| {
                session
                    .tab_list
                    .tabs
                    .iter()
                    .position(|t| t.file.path.as_deref() == Some(path))
            });
        if let Some(index) = already_open {
            self.activate_tab(area_id, index, cx);
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
        let list = &mut self.panels.layout.ensure_editor_session(area_id).tab_list;
        let was_welcome = list.tabs.is_empty();
        list.tabs.push(Self::new_tab_from_markdown(
            cx,
            markdown,
            Some(path.to_path_buf()),
        ));
        let last = list.tabs.len() - 1;
        if was_welcome {
            // First tab: migrate the welcome panels into editing panels.
            self.panels.layout.enter_editing(area_id);
        }
        self.activate_tab(area_id, last, cx);
        crate::app::menus::record_recent_file_from_editor(path, cx);
    }

    /// Opens a file in the ACTIVE editor's tab bar. Returns `false` when no
    /// Editor area exists (the caller decides how to handle that case).
    pub(crate) fn open_file_in_active_editor(
        &mut self,
        path: &std::path::Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if let Some(area) = self.panels.layout.active_editor_area {
            self.open_file_in_area(area, path, window, cx);
            true
        } else {
            false
        }
    }

    /// Opens a fresh untitled tab in the given Editor area.
    pub(crate) fn new_untitled_tab(&mut self, area_id: AreaId, cx: &mut Context<Self>) {
        let list = &mut self.panels.layout.ensure_editor_session(area_id).tab_list;
        let was_welcome = list.tabs.is_empty();
        list.tabs
            .push(Self::new_tab_from_markdown(cx, String::new(), None));
        let last = list.tabs.len() - 1;
        if was_welcome {
            // First tab: migrate the welcome panels into editing panels.
            self.panels.layout.enter_editing(area_id);
        }
        self.activate_tab(area_id, last, cx);
    }

    /// Closes the tab at `index` in the given Editor area, activating a
    /// neighbor. Closing the last tab leaves the area back in the welcome
    /// state (no tabs).
    pub(crate) fn close_tab(&mut self, area_id: AreaId, index: usize, cx: &mut Context<Self>) {
        let Some(session) = self.panels.layout.editor_sessions.get_mut(&area_id) else {
            return;
        };
        let list = &mut session.tab_list;
        if index >= list.tabs.len() {
            return;
        }
        let was_active = index == list.active_tab;
        list.tabs.remove(index);
        if list.tabs.is_empty() {
            list.active_tab = 0;
            // Last tab: migrate the editing panels back into welcome
            // panels (they remember their kind for the next entry).
            self.panels.layout.exit_editing(area_id);
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
    /// The Editor area that `tab()` / `doc()` currently resolve to: the
    /// transient per-area render/event context if set, else the active
    /// editor.
    pub(crate) fn routed_tab_area(&self) -> Option<AreaId> {
        self.current_tab_area.or(self.panels.layout.active_editor_area)
    }

    /// Run `f` with the routing hint set to `area_id`, restoring the
    /// previous context afterwards. Per-area event handlers use this so
    /// their `tab()`/`doc()` access hits the owning editor area regardless
    /// of which editor is currently active.
    pub(crate) fn with_current_tab_area<R>(
        &mut self,
        area_id: AreaId,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let previous = self.current_tab_area;
        self.current_tab_area = Some(area_id);
        let result = f(self);
        self.current_tab_area = previous;
        result
    }

    /// The active document tab (of the routed editor area).
    pub(crate) fn tab(&self) -> &DocumentTab {
        let area = self
            .routed_tab_area()
            .expect("tab() requires an active editor area");
        let list = &self
            .panels
            .layout
            .editor_session(area)
            .expect("routed editor area has no editor session")
            .tab_list;
        &list.tabs[list.active_tab]
    }

    /// The active document tab, mutably.
    pub(crate) fn tab_mut(&mut self) -> &mut DocumentTab {
        let area = self
            .routed_tab_area()
            .expect("tab_mut() requires an active editor area");
        let list = &mut self
            .panels
            .layout
            .ensure_editor_session(area)
            .tab_list;
        let index = list.active_tab;
        &mut list.tabs[index]
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
    // Per-area tab access (independent editors)
    // ------------------------------------------------------------------

    /// True when the given Editor area has at least one document tab.
    pub(crate) fn area_has_tabs(&self, area_id: AreaId) -> bool {
        self.panels
            .layout
            .editor_session(area_id)
            .is_some_and(|session| !session.tab_list.tabs.is_empty())
    }

    /// The given Editor area's working mode: welcome (no tabs) or editing
    /// (at least one tab). The single source of truth for how an area's
    /// body and status bar render.
    pub(crate) fn area_mode(&self, area_id: AreaId) -> EditorAreaMode {
        self.panels.layout.editor_area_mode(area_id)
    }

    /// The given Editor area's tab list. Panics if the area has no editor
    /// session (rendering code must call `tab_list_mut_for` first).
    pub(crate) fn tab_list_for(&self, area_id: AreaId) -> &EditorTabList {
        &self
            .panels
            .layout
            .editor_session(area_id)
            .unwrap_or_else(|| panic!("no editor session for area {area_id}"))
            .tab_list
    }

    /// The given Editor area's tab list, created on demand.
    pub(crate) fn tab_list_mut_for(&mut self, area_id: AreaId) -> &mut EditorTabList {
        &mut self.panels.layout.ensure_editor_session(area_id).tab_list
    }

    /// The given Editor area's active tab.
    pub(crate) fn tab_for(&self, area_id: AreaId) -> &DocumentTab {
        let list = self.tab_list_for(area_id);
        &list.tabs[list.active_tab]
    }

    /// The given Editor area's active tab, mutably.
    pub(crate) fn tab_mut_for(&mut self, area_id: AreaId) -> &mut DocumentTab {
        let list = &mut self.panels.layout.ensure_editor_session(area_id).tab_list;
        let index = list.active_tab;
        &mut list.tabs[index]
    }

    /// The active editor's tab, if an active editor with tabs exists.
    pub(crate) fn active_editor_tab(&self) -> Option<&DocumentTab> {
        let session = self.panels.layout.active_editor_session()?;
        session.tab_list.tabs.get(session.tab_list.active_tab)
    }

    /// The active editor's serialized document text, if any.
    pub(crate) fn active_editor_serialized_text(&self, cx: &App) -> Option<String> {
        let tab = self.active_editor_tab()?;
        Some(if tab.mode == EditorMode::SourceCode {
            tab.document.to_raw_source(cx)
        } else {
            tab.document.to_markdown(cx)
        })
    }

    /// Serialize the given Editor area's active document.
    pub(crate) fn serialized_document_text_for(&self, area_id: AreaId, cx: &App) -> String {
        let tab = self.tab_for(area_id);
        if tab.mode == EditorMode::SourceCode {
            tab.document.to_raw_source(cx)
        } else {
            tab.document.to_markdown(cx)
        }
    }

    /// Split `area_id` with a same-kind sibling and seed the new Editor
    /// area per `mode`: [`AreaSplitMode::Copy`] deep-copies the source tab
    /// list (and the layout clones the inner panel layout);
    /// [`AreaSplitMode::Fresh`] leaves the new editor blank. Returns the
    /// new area's id.
    pub(crate) fn split_area(
        &mut self,
        area_id: AreaId,
        direction: crate::layout::Axis,
        ratio: f32,
        mode: AreaSplitMode,
        cx: &mut Context<Self>,
    ) -> Option<AreaId> {
        let new_id = self
            .panels
            .layout
            .split_window_area(area_id, direction, ratio, mode)?;
        if mode == AreaSplitMode::Copy {
            self.copy_editor_tab_list(area_id, new_id, cx);
        }
        Some(new_id)
    }

    /// Deep-copy the source Editor area's tab list into `dst`: every tab is
    /// re-materialized from its serialized document, so the two editors are
    /// fully independent (separate undo, focus, scroll, and dirty state).
    fn copy_editor_tab_list(
        &mut self,
        src: AreaId,
        dst: AreaId,
        cx: &mut Context<Self>,
    ) {
        let Some(source) = self
            .panels
            .layout
            .editor_session(src)
            .map(|session| &session.tab_list)
        else {
            // Source never got a session (e.g. a non-Editor area): the
            // copy is a blank editor.
            self.panels.layout.ensure_editor_session(dst);
            return;
        };
        if source.tabs.is_empty() {
            // Welcome-state editor: the copy is another welcome-state
            // editor — there is nothing to rebuild for it.
            self.panels.layout.ensure_editor_session(dst);
            return;
        }
        let mut copies: Vec<(String, Option<PathBuf>, EditorMode, bool)> = Vec::new();
        for tab in &source.tabs {
            let text = if tab.mode == EditorMode::SourceCode {
                tab.document.to_raw_source(cx)
            } else {
                tab.document.to_markdown(cx)
            };
            copies.push((text, tab.file.path.clone(), tab.mode, tab.file.dirty));
        }
        let active = source.active_tab;
        let mut list = EditorTabList {
            tabs: Vec::with_capacity(copies.len()),
            active_tab: 0,
        };
        for (text, path, mode, dirty) in copies {
            let mut tab = Self::new_tab_from_markdown(cx, text, path);
            tab.mode = mode;
            tab.file.dirty = dirty;
            list.tabs.push(tab);
        }
        list.active_tab = active.min(list.tabs.len().saturating_sub(1));
        let dst_list = &mut self.panels.layout.ensure_editor_session(dst).tab_list;
        *dst_list = list;

        // Rebuild the copied area's runtime registries under its routing
        // context, then restore the previous context.
        let previous = self.current_tab_area;
        self.current_tab_area = Some(dst);
        self.rebuild_table_runtimes(cx);
        self.rebuild_image_runtimes(cx);
        self.refresh_preview_blocks(cx);
        self.tab_mut().focus.pending = self.first_focusable_entity_id(cx);
        self.current_tab_area = previous;
    }

    /// First dirty tab across ALL editor areas, if any.
    pub(crate) fn first_dirty_tab(&self) -> Option<(AreaId, usize)> {
        for (area, session) in &self.panels.layout.editor_sessions {
            for (index, tab) in session.tab_list.tabs.iter().enumerate() {
                if tab.file.dirty {
                    return Some((*area, index));
                }
            }
        }
        None
    }
}
