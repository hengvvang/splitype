//! Top-level editor controller and window state.
//!
//! [`Editor`] owns window-level concerns such as view mode, save/close flow,
//! scroll state, and focus deferral. The runtime block tree itself lives in
//! [`Document`], which centralizes structural mutations and cached visible
//! order metadata.

pub(crate) use std::time::{Duration, Instant};

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) use gpui::*;

pub(crate) use crate::core::extensions::footnote_def::{
    FootnoteDefinitionBinding, FootnoteMap, FootnoteReferenceLocation, FootnoteResolvedOccurrence,
};
pub(crate) use crate::core::extensions::image_ref::{
    ImageReferenceDefinitions, parse_image_reference_definitions,
};
pub(crate) use crate::core::extensions::table::{
    TableAxisHighlight, TableAxisKind, TableAxisMarker, TableColumnAlignment, TableData,
    serialize_table_cell_markdown,
};
pub(crate) use crate::core::extensions::table_cell::TableCellPosition;
pub(crate) use crate::core::text::link_ref::{
    LinkReferenceDefinitions, parse_link_reference_definitions,
};
pub(crate) use crate::core::text::rich_text::RichText;
pub(crate) use crate::engine::block_types::{BlockData, BlockType, UndoCaptureKind};
pub(crate) use crate::engine::document::Document;
pub(crate) use crate::ui::blocks::block_view::Block;
pub(crate) use crate::ui::blocks::table_block::TableGrid;
pub(crate) use crate::ui::components::context_menu::{ContextMenuState, TableInsertDialogState};
pub(crate) use crate::ui::components::status_bar::StatusBarState;
pub(crate) use crate::ui::views::explorer_view::Explorer;
pub(crate) use crate::ui::views::layout::Layout;

/// Link navigation request deferred until a `Window` is available.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PendingOpenLink {
    pub(crate) prompt_target: String,
    pub(crate) open_target: String,
}

/// Top-level controller that owns editor-wide state and delegates tree
/// mutations to [`Document`].
///
/// The editor subscribes to every [`BlockAction`](crate::engine::block_types::BlockAction)
/// emitted by child blocks. Structural changes are handled centrally so focus,
/// scrolling, dirty tracking, and serialization stay synchronized.
pub struct Editor {
    pub(crate) document: Document,
    pub(crate) table_cells: HashMap<EntityId, TableCellBinding>,
    /// Which view the editor is currently presenting.
    pub(crate) view_mode: EditMode,
    /// Deferred focus target applied during render when a [`Window`] is
    /// available.
    pub(crate) pending_focus: Option<EntityId>,
    pub(crate) active_entity_id: Option<EntityId>,
    pub(crate) pending_scroll_active_block_into_view: bool,
    pub(crate) pending_scroll_recheck_after_layout: bool,
    pub(crate) pending_save: bool,
    pub(crate) pending_save_as: bool,
    pub(crate) pending_open_link: Option<PendingOpenLink>,
    pub(crate) pending_window_edited: bool,
    pub(crate) pending_window_title_refresh: bool,
    pub(crate) document_dirty: bool,
    pub(crate) file_path: Option<PathBuf>,
    pub(crate) scroll_handle: ScrollHandle,
    pub(crate) last_scroll_viewport_size: Option<Size<Pixels>>,
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
    pub(crate) close_guard_installed: bool,
    pub(crate) show_unsaved_changes_dialog: bool,
    /// When true, the window will close after the next successful save.
    pub(crate) pending_close_after_save: bool,
    /// Focus target to restore when the close dialog is dismissed.
    pub(crate) close_dialog_restore_focus: Option<EntityId>,
    pub(crate) pending_drop_replace_path: Option<PathBuf>,
    pub(crate) show_drop_replace_dialog: bool,
    pub(crate) pending_drop_replace_after_save: bool,
    pub(crate) drop_replace_restore_focus: Option<EntityId>,
    /// Optional informational dialog shown from the Help menu.
    pub(crate) info_dialog: Option<InfoDialogKind>,
    /// True while an online update check is running in the background.
    pub(crate) update_check_in_progress: bool,
    pub(crate) workspace: Explorer,
    pub(crate) area_layout: Layout,
    pub(crate) status_bar: StatusBarState,
    pub(crate) context_menu: Option<ContextMenuState>,
    pub(crate) table_insert_dialog: Option<TableInsertDialogState>,
    pub(crate) context_menu_submenu_close_task: Option<Task<()>>,
    pub(crate) table_axis_preview: Option<TableAxisSelection>,
    pub(crate) table_axis_selection: Option<TableAxisSelection>,
    pub(crate) cross_block_selection: Option<CrossBlockSelection>,
    pub(crate) cross_block_drag: Option<CrossBlockDrag>,
    pub(crate) rendered_select_all_cycle: Option<RenderedSelectAllCycle>,
    /// Open top-level menu in the in-window fallback menu bar.
    pub(crate) menu_bar_open: Option<usize>,
    pub(crate) menu_bar_expanded: bool,
    /// Open child submenu inside the in-window fallback menu panel.
    pub(crate) menu_submenu_open: Option<usize>,
    pub(crate) menu_bar_hovered: bool,
    pub(crate) menu_panel_hovered: bool,
    pub(crate) menu_submenu_panel_hovered: bool,
    /// Hover state for the invisible bridge spanning the gap between the menu
    /// panel and an open submenu. Tracked separately from
    /// `menu_submenu_panel_hovered` so the handoff between the two regions
    /// cannot clobber a single shared flag and tear the menu down.
    pub(crate) menu_submenu_bridge_hovered: bool,
    pub(crate) menu_close_task: Option<Task<()>>,
    pub(crate) scrollbar_hovered: bool,
    pub(crate) scrollbar_visible_until: Instant,
    pub(crate) scrollbar_fade_task: Option<Task<()>>,
    /// Forces a repaint shortly after a pending scroll-into-view that could
    /// not be satisfied yet (the target block has no measured bounds), so the
    /// scroll lands on the next frame instead of waiting for the cursor blink.
    pub(crate) scroll_recheck_task: Option<Task<()>>,
    pub(crate) scrollbar_drag: Option<ScrollbarDragSession>,
    pub(crate) undo_history: Vec<HistoryEntry>,
    pub(crate) redo_history: Vec<HistoryEntry>,
    pub(crate) pending_undo_capture: Option<PendingUndoCapture>,
    pub(crate) last_selection_snapshot: UndoSelectionSnapshot,
    pub(crate) last_stable_source_text: String,
    pub(crate) history_restore_in_progress: bool,
    pub(crate) image_reference_definitions: Arc<ImageReferenceDefinitions>,
    pub(crate) link_reference_definitions: Arc<LinkReferenceDefinitions>,
    pub(crate) footnote_registry: Arc<FootnoteMap>,
    /// Cached preview blocks rebuilt from source when document changes.
    pub(crate) preview_blocks: Vec<Entity<Block>>,
    pub(crate) preview_source_hash: u64,
    /// Interactive source editor for the Source channel.  Edits sync
    /// to the document via the BlockAction::Changed handler.
    pub(crate) source_panel_block: Option<Entity<Block>>,
    /// Hash of the document text that source_panel_block was built from.
    pub(crate) source_panel_doc_hash: u64,
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
#[derive(Clone, Debug, PartialEq, Eq)]
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
pub enum EditMode {
    /// Rich rendered view where each block is styled by its semantic kind.
    Wysiwyg,
    /// Plain source view where the full Markdown document is edited as a
    /// single raw buffer.
    Source,
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

    pub fn from_markdown(
        cx: &mut Context<Self>,
        markdown: String,
        file_path: Option<PathBuf>,
    ) -> Self {
        let normalized = markdown.replace("\r\n", "\n").replace('\r', "\n");
        let mut roots = Self::parse_document(cx, &normalized);
        if roots.is_empty() {
            roots.push(Self::new_block(cx, BlockData::paragraph(String::new())));
        }

        let mut document = Document::new(roots);
        document.rebuild_metadata_and_snapshot(cx);
        let pending_focus = document.first_root().map(|block| block.entity_id());

        let mut editor = Self {
            document,
            table_cells: HashMap::new(),
            view_mode: EditMode::Wysiwyg,
            pending_focus,
            active_entity_id: pending_focus,
            pending_scroll_active_block_into_view: true,
            pending_scroll_recheck_after_layout: true,
            pending_save: false,
            pending_save_as: false,
            pending_open_link: None,
            pending_window_edited: false,
            pending_window_title_refresh: false,
            document_dirty: false,
            file_path,
            scroll_handle: ScrollHandle::new(),
            last_scroll_viewport_size: None,
            prev_visible_block_ids: Vec::new(),
            row_stride_cache: HashMap::new(),
            prev_render_window: None,
            close_guard_installed: false,
            show_unsaved_changes_dialog: false,
            pending_close_after_save: false,
            close_dialog_restore_focus: None,
            pending_drop_replace_path: None,
            show_drop_replace_dialog: false,
            pending_drop_replace_after_save: false,
            drop_replace_restore_focus: None,
            info_dialog: None,
            update_check_in_progress: false,
            workspace: Explorer::default(),
            area_layout: Layout::default(),
            status_bar: StatusBarState::default(),
            context_menu: None,
            table_insert_dialog: None,
            context_menu_submenu_close_task: None,
            table_axis_preview: None,
            table_axis_selection: None,
            cross_block_selection: None,
            cross_block_drag: None,
            rendered_select_all_cycle: None,
            menu_bar_open: None,
            menu_bar_expanded: false,
            menu_submenu_open: None,
            menu_bar_hovered: false,
            menu_panel_hovered: false,
            menu_submenu_panel_hovered: false,
            menu_submenu_bridge_hovered: false,
            menu_close_task: None,
            scrollbar_hovered: false,
            scrollbar_visible_until: Instant::now(),
            scrollbar_fade_task: None,
            scroll_recheck_task: None,
            scrollbar_drag: None,
            undo_history: Vec::new(),
            redo_history: Vec::new(),
            pending_undo_capture: None,
            last_selection_snapshot: Self::empty_selection_snapshot(),
            last_stable_source_text: normalized,
            history_restore_in_progress: false,
            image_reference_definitions: Arc::default(),
            link_reference_definitions: Arc::default(),
            footnote_registry: Arc::default(),
            preview_blocks: Vec::new(),
            preview_source_hash: 0,
            source_panel_block: None,
            source_panel_doc_hash: 0,
        };
        editor.rebuild_table_runtimes(cx);
        editor.rebuild_image_runtimes(cx);
        editor.refresh_preview_blocks(cx);
        editor.pending_focus = editor.first_focusable_entity_id(cx);
        editor.active_entity_id = editor.pending_focus;
        editor.refresh_stable_document_snapshot(cx);
        editor
    }
}
