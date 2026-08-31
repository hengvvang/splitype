//! WYSIWYG pane state and domain types.

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use gpui::{Entity, EntityId, Pixels, Point};

use crate::model::block::{Block, footnotes::FootnoteMap};
use crate::input::history::delta::Transaction;
use crate::model::protocol::UndoCaptureKind;
use crate::markdown::block::image::ImageReferenceDefinitions;
use crate::markdown::block::link::LinkReferenceDefinitions;
use crate::markdown::block::table::{TableAxis, TableCellPosition};
use crate::markdown::inline::text::BlockText;
use crate::markdown::parse::{BlockData, BlockId};

pub use core_contracts::AutoscrollStrategy;

/// Focus routing and deferred focus targets.
#[derive(Default)]
pub struct FocusState {
    pub pending: Option<EntityId>,
    pub active_entity: Option<EntityId>,
}

/// Editor-level selection spanning rendered blocks.
#[derive(Default)]
pub struct SelectionState {
    pub cross_block: Option<CrossBlockSelection>,
    pub cross_block_drag: Option<CrossBlockDrag>,
    pub select_all_cycle: Option<WysiwygSelectAllCycle>,
}

impl SelectionState {
    /// Returns true if cross-block selection or drag is active.
    #[inline]
    pub const fn has_cross_block(&self) -> bool {
        self.cross_block.is_some() || self.cross_block_drag.is_some()
    }

    /// Clears cross-block selection and drag session.
    #[inline]
    pub fn clear_cross_block(&mut self) -> bool {
        let had_cross = self.cross_block.take().is_some();
        self.cross_block_drag = None;
        had_cross
    }

    /// Clears all active selection and multi-press cycle state.
    #[inline]
    pub fn clear_all(&mut self) {
        self.cross_block = None;
        self.cross_block_drag = None;
        self.select_all_cycle = None;
    }
}

pub static EMPTY_FOCUS_STATE: FocusState = FocusState {
    pending: None,
    active_entity: None,
};

pub static EMPTY_SELECTION_STATE: SelectionState = SelectionState {
    cross_block: None,
    cross_block_drag: None,
    select_all_cycle: None,
};

/// Undo/redo stacks, coalescing state, and delta transaction history.
#[derive(Default)]
pub struct UndoHistory {
    pub undo_entries: Vec<HistoryEntry>,
    pub redo_entries: Vec<HistoryEntry>,
    pub pending_capture: Option<PendingUndoCapture>,
    pub last_selection_snapshot: UndoSelectionSnapshot,
    pub restore_in_progress: bool,
}

/// Document-wide reference registries (images, links, footnotes).
#[derive(Default)]
pub struct ReferenceRegistries {
    pub image: Arc<ImageReferenceDefinitions>,
    pub link: Arc<LinkReferenceDefinitions>,
    pub footnotes: Arc<FootnoteMap>,
    /// Base directory the registries were last synced against; blocks
    /// re-resolve image sources whenever this changes.
    pub base_dir: Option<PathBuf>,
    /// Document structure version at the time every current block last
    /// received its reference context. A mismatch means blocks were added
    /// or replaced since, so the per-block sync cannot be skipped.
    pub synced_structure_version: u64,
    /// Blocks and table cells that could contribute reference definitions,
    /// footnote content, or standalone-image syntax, cached at the last full
    /// registry sync. A block edit outside this set cannot change the
    /// registries, so the per-keystroke rebuild is skipped.
    pub candidate_blocks: HashSet<EntityId>,
}

/// Native table cell bindings and axis selections.
#[derive(Default)]
pub struct TableGrids {
    pub cells: HashMap<EntityId, TableCellBinding>,
    pub axis_preview: Option<TableAxisSelection>,
    pub axis_selection: Option<TableAxisSelection>,
}

/// A block-local selection captured as a path through the block tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockSelectionAnchor {
    /// Root index followed by the sibling index of each child level.
    pub path: Vec<usize>,
    /// Current (projected) content range inside the anchored block.
    pub content_range: Range<usize>,
}

/// Selection snapshot used by undo/redo to restore the caret.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UndoSelectionSnapshot {
    /// Global source range. Only meaningful for cross-block selections and
    /// source-mode selections; block-local snapshots carry [`Self::block_anchor`]
    /// instead and leave this empty.
    pub range: Range<usize>,
    pub reversed: bool,
    /// Block-local caret anchor, when the selection lives inside one block.
    pub block_anchor: Option<BlockSelectionAnchor>,
}

/// One undo history entry containing transactional deltas and selection state.
#[derive(Clone, Debug)]
pub struct HistoryEntry {
    pub transaction: Transaction,
    pub selection_before: UndoSelectionSnapshot,
    pub selection_after: UndoSelectionSnapshot,
    pub timestamp: Instant,
    pub kind: UndoCaptureKind,
}

/// Deferred undo capture used to coalesce adjacent typing edits.
#[derive(Clone, Debug)]
pub struct PendingUndoCapture {
    pub snapshot: HistoryEntry,
    pub target_block_id: Option<BlockId>,
    pub initial_text: Option<BlockText>,
    pub initial_roots: Option<Vec<BlockData>>,
}

/// Cross-block selection endpoint in visible block order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrossBlockSelectionEndpoint {
    pub entity_id: EntityId,
    pub offset: usize,
}

/// Editor-level selection spanning two visible block endpoints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrossBlockSelection {
    pub anchor: CrossBlockSelectionEndpoint,
    pub focus: CrossBlockSelectionEndpoint,
}

/// Drag state while creating or extending a cross-block selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrossBlockDrag {
    pub anchor: CrossBlockSelectionEndpoint,
}

/// Short-lived Ctrl/Cmd+A press counter for rendered-mode selection upgrade.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WysiwygSelectAllCycle {
    pub entity_id: EntityId,
    pub count: u8,
    pub last_pressed_at: Instant,
}

/// Mapping from one visible block's text range to canonical Markdown offsets.
#[derive(Clone)]
pub struct SourceTargetMapping {
    pub entity: Entity<Block>,
    pub full_source_range: Range<usize>,
    pub content_to_source: Vec<usize>,
    pub source_to_content: Vec<usize>,
}

impl SourceTargetMapping {
    pub fn content_to_source_offset(&self, content_offset: usize) -> usize {
        let max_content = self.content_to_source.len().saturating_sub(1);
        self.content_to_source[content_offset.min(max_content)]
    }

    pub fn source_to_content_offset(&self, source_offset: usize) -> usize {
        let max_source = self.source_to_content.len().saturating_sub(1);
        self.source_to_content[source_offset.min(max_source)]
    }
}

/// Unified description of the active selection in the editor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorSelection {
    /// No selection active.
    None,
    /// Selection confined within a single focused block.
    IntraBlock {
        block_id: EntityId,
        range: Range<usize>,
        reversed: bool,
    },
    /// Selection spanning multiple distinct blocks.
    CrossBlock(CrossBlockSelection),
    /// Selection of a table row or column.
    TableAxis(TableAxisSelection),
}

/// One bound native table cell: the owning table block plus the cell
/// entity at a grid position.
#[derive(Clone)]
pub struct TableCellBinding {
    pub table_block: Entity<Block>,
    pub cell: Entity<Block>,
    pub position: TableCellPosition,
}

/// Selected row or column in a rendered native table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableAxisSelection {
    pub table_block_id: EntityId,
    pub kind: TableAxis,
    pub index: usize,
}

/// State for the interactive Table Size Matrix Picker popup.
#[derive(Clone, Debug)]
pub struct TableSizePickerState {
    pub table_block_id: EntityId,
    pub position: Point<Pixels>,
    pub current_rows: usize,
    pub current_cols: usize,
    pub hovered_rows: Option<usize>,
    pub hovered_cols: Option<usize>,
}


