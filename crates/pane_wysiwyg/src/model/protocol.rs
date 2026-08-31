//! Events emitted by a block to its parent editor, plus undo capture policy
//! and clipboard image payloads.
//!
//! The Editor subscribes to these events on every block via
//! `cx.subscribe(&block, Self::on_block_event)`.

use std::ops::Range;
use std::path::PathBuf;

use gpui::{Image, Pixels, Point, SharedString};

use crate::markdown::block::table::TableAxis;
use crate::markdown::inline::text::BlockText;

/// Image payload extracted from GPUI's clipboard abstraction.
///
/// File-manager copies are usually represented as local paths, while bitmap
/// copies from image editors or browsers arrive as encoded image bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PastedImageSource {
    ClipboardImage(Image),
    LocalPath(PathBuf),
}

/// Events emitted by a block to its parent editor when structural
/// changes or focus transfers are needed that the block cannot handle alone.
#[derive(Debug, Clone)]
pub enum BlockEvent {
    /// Capture the current document state before an upcoming mutation.
    PrepareUndo {
        kind: UndoCaptureKind,
        target_block_id: Option<crate::markdown::parse::BlockId>,
        initial_text: Option<crate::markdown::inline::text::BlockText>,
    },
    /// The block's content or kind changed; the editor should mark the
    /// document dirty and optionally scroll to keep the block visible.
    Changed,
    /// The user pressed Enter; a new block should be created after this
    /// one with the given trailing text.
    RequestNewline {
        trailing: BlockText,
        source_already_mutated: bool,
    },
    /// The user pressed Enter at offset 0 of a block; an empty paragraph should be inserted above this block.
    RequestNewlineAbove,
    /// The user pressed Enter on a callout header; the editor should ensure
    /// the callout owns a body entry and move focus into it.
    RequestEnterCalloutBody,
    /// The user requested a quote-group break at the current quote depth.
    /// The editor should insert a new empty quote group at the current depth,
    /// with whatever separator structure is required by Markdown at that level.
    RequestQuoteBreak,
    /// The user requested to exit the current callout into a plain text block.
    /// The editor should insert the separator structure needed to end the
    /// surrounding quote group, then focus a plain paragraph entry below it.
    RequestCalloutBreak,
    /// The user pressed Backspace at the start of this block; its entire
    /// content should be appended to the previous block.
    RequestMergeIntoPrevious { content: BlockText },
    /// A multi-line paste was detected; the editor must split the pasted
    /// lines into separate blocks and re-attach the leading/trailing text
    /// to the correct positions.
    RequestPasteMultiline {
        leading: BlockText,
        lines: Vec<String>,
        trailing: BlockText,
        split_physical_lines: bool,
    },
    /// An image-like clipboard payload was pasted. The editor resolves
    /// storage preferences and inserts either an image block or image text.
    RequestPasteImage {
        leading: BlockText,
        source: PastedImageSource,
        trailing: BlockText,
    },
    /// Replace the current editor-level cross-block selection with text
    /// submitted through the focused block input handler.
    RequestReplaceCrossBlockSelection {
        text: String,
        selected_range_relative: Option<Range<usize>>,
        mark_inserted_text: bool,
        undo_kind: UndoCaptureKind,
    },
    /// Ctrl/Cmd+A was pressed in rendered editing. The editor decides whether
    /// this press selects the focused block or upgrades to all rendered blocks.
    RequestRenderedSelectAll,
    /// Tab pressed in list context; increase the current block's nesting when
    /// the previous visible block can adopt it.
    RequestIndent,
    /// Shift-Tab pressed in list context; lift the current block out one level.
    RequestOutdent,
    /// Backspace on a nested list item should remove its marker first,
    /// degrading it into a direct list-child paragraph at the same depth.
    RequestDowngradeNestedListItemToChildParagraph,
    /// Toggle the checked state of a task-list item.
    RequestToggleTaskChecked,
    /// Prompt to open the clicked inline link destination.
    /// `prompt_target` preserves the raw syntax target shown to the user,
    /// while `open_target` is the resolved destination actually opened.
    RequestOpenLink {
        prompt_target: String,
        open_target: String,
    },
    /// Jump from a rendered footnote reference to the corresponding
    /// in-place footnote definition block.
    RequestJumpToFootnoteDefinition { id: String },
    /// Jump from an in-place footnote definition back to its first reference.
    RequestJumpToFootnoteBackref { id: String },
    /// Show or hide the footnote content tooltip. `content` carries the
    /// definition text when the hovered element is a definition header;
    /// reference hovers leave it empty and the editor resolves it from the
    /// footnote binding (no tooltip when the definition is missing).
    RequestFootnoteTooltip {
        id: String,
        content: Option<SharedString>,
        position: Point<Pixels>,
        show: bool,
    },
    /// Move focus horizontally across native table cells.
    RequestTableCellMoveHorizontal { delta: i32 },
    /// Move focus vertically across native table cells.
    RequestTableCellMoveVertical { delta: i32 },
    /// Append one empty column to a native table.
    RequestAppendTableColumn,
    /// Append one empty body row to a native table.
    RequestAppendTableRow,
    /// A native table axis handle was entered or left by the pointer.
    /// `hovered` distinguishes the two so the editor can ignore a leave
    /// that arrives after an adjacent handle has already taken the preview.
    RequestTableAxisPreview {
        kind: TableAxis,
        index: usize,
        hovered: bool,
    },
    /// Select one native table row or column for batch operations.
    RequestSelectTableAxis { kind: TableAxis, index: usize },
    /// Open the axis context menu for a native table row or column.
    RequestOpenTableAxisMenu {
        kind: TableAxis,
        index: usize,
        position: Point<Pixels>,
    },
    /// Open the table size matrix picker from the bottom-right corner control.
    RequestOpenTableSizePicker {
        position: Point<Pixels>,
    },
    /// Reorder a native table row or column by swapping positions.
    RequestReorderTableAxis {
        kind: TableAxis,
        from: usize,
        to: usize,
    },
    /// Insert a table row or column at a specific boundary index.
    RequestInsertTableAxisAt {
        kind: TableAxis,
        index: usize,
    },
    /// Cursor reached the top of this block; move focus to the previous
    /// visible block, preserving the preferred horizontal position.
    RequestFocusPrevious { preferred_x: Option<f32> },
    /// Cursor reached the bottom of this block; move focus to the next
    /// visible block, preserving the preferred horizontal position.
    RequestFocusNext { preferred_x: Option<f32> },
    /// Move focus to the start of the previous visible block.
    RequestBlockUp,
    /// Move focus to the start of the next visible block.
    RequestBlockDown,
    /// This block should be deleted (empty and backspace/delete pressed).
    RequestDelete,
    /// The user clicked this block; notify siblings so they re-render
    /// in display mode.
    RequestFocus,
}

/// Domain semantic classification for block-level events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockEventCategory {
    /// Lifecycle and undo capture preparation.
    Lifecycle,
    /// Content mutation and state synchronization.
    ContentChange,
    /// Text editing and clipboard insertion mutations.
    TextEdit,
    /// Document tree structural mutation (lists, quotes, callouts, deletion).
    Structure,
    /// Native table navigation and axis mutations.
    Table,
    /// Cross-block focus navigation, links, and footnote tooltips.
    Interaction,
}

impl BlockEvent {
    /// Returns the semantic domain category of this event.
    pub fn category(&self) -> BlockEventCategory {
        match self {
            Self::PrepareUndo { .. } => BlockEventCategory::Lifecycle,
            Self::Changed => BlockEventCategory::ContentChange,
            Self::RequestNewline { .. }
            | Self::RequestNewlineAbove
            | Self::RequestMergeIntoPrevious { .. }
            | Self::RequestPasteMultiline { .. }
            | Self::RequestPasteImage { .. }
            | Self::RequestReplaceCrossBlockSelection { .. }
            | Self::RequestRenderedSelectAll => BlockEventCategory::TextEdit,
            Self::RequestEnterCalloutBody
            | Self::RequestQuoteBreak
            | Self::RequestCalloutBreak
            | Self::RequestIndent
            | Self::RequestOutdent
            | Self::RequestDowngradeNestedListItemToChildParagraph
            | Self::RequestToggleTaskChecked
            | Self::RequestDelete => BlockEventCategory::Structure,
            Self::RequestTableCellMoveHorizontal { .. }
            | Self::RequestTableCellMoveVertical { .. }
            | Self::RequestAppendTableColumn
            | Self::RequestAppendTableRow
            | Self::RequestTableAxisPreview { .. }
            | Self::RequestSelectTableAxis { .. }
            | Self::RequestOpenTableAxisMenu { .. }
            | Self::RequestOpenTableSizePicker { .. }
            | Self::RequestReorderTableAxis { .. }
            | Self::RequestInsertTableAxisAt { .. } => BlockEventCategory::Table,
            Self::RequestFocusPrevious { .. }
            | Self::RequestFocusNext { .. }
            | Self::RequestBlockUp
            | Self::RequestBlockDown
            | Self::RequestFocus
            | Self::RequestOpenLink { .. }
            | Self::RequestJumpToFootnoteDefinition { .. }
            | Self::RequestJumpToFootnoteBackref { .. }
            | Self::RequestFootnoteTooltip { .. } => BlockEventCategory::Interaction,
        }
    }

    /// Whether this event should dismiss any active cross-block text selection
    /// (typing, structural edits, and paste all replace it).
    pub fn clears_cross_block_selection(&self) -> bool {
        match self.category() {
            BlockEventCategory::ContentChange
            | BlockEventCategory::TextEdit
            | BlockEventCategory::Structure => true,
            BlockEventCategory::Table => matches!(
                self,
                Self::RequestAppendTableColumn
                    | Self::RequestAppendTableRow
                    | Self::RequestReorderTableAxis { .. }
                    | Self::RequestInsertTableAxisAt { .. }
            ),
            BlockEventCategory::Lifecycle | BlockEventCategory::Interaction => false,
        }
    }

    /// Whether this event mutates the document tree topology.
    pub fn is_structural(&self) -> bool {
        matches!(self.category(), BlockEventCategory::Structure)
    }
}

/// Undo coalescing category captured before a mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndoCaptureKind {
    /// Text edits that may merge with adjacent typing within the coalescing window.
    CoalescibleText,
    /// Structural or discrete edits that always form their own undo entry.
    NonCoalescible,
}



