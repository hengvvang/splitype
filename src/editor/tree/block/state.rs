//! Block state types: interaction states, append controls, and paint records.

use std::ops::Range;

use gpui::*;

use crate::model::block::CalloutKind;
use crate::model::block::image::ImageResolvedSource;
use crate::model::parse::BlockId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum InlineFormat {
    Bold,
    Italic,
    Underline,
    Code,
    Strikethrough,
}

/// Cached standalone image presentation state for a block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ImageHandle {
    pub(crate) alt: String,
    pub(crate) src: String,
    pub(crate) title: Option<String>,
    pub(crate) resolved_source: ImageResolvedSource,
}

/// How a collapsed caret at an inline projection boundary inherits style.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CollapsedCaretAffinity {
    #[default]
    Default,
    OuterStart,
    OuterEnd,
}

/// Interactive sub-regions of table append controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TableHoverRegion {
    /// Border indicator line.
    Edge,
    /// Anti-flicker buffer zone.
    BufferZone,
    /// The append (+) button itself.
    AppendButton,
}

/// Hover and close scheduling state for one axis (row or column) of table append controls.
#[derive(Default)]
pub(crate) struct TableAxisAppendState {
    pub(crate) edge_hovered: bool,
    pub(crate) buffer_zone_hovered: bool,
    pub(crate) button_hovered: bool,
    /// Overall control visibility/active state.
    pub(crate) is_active: bool,
    /// Delayed dismiss debounce task handle (120ms).
    pub(crate) dismiss_task: Option<Task<()>>,
}

impl TableAxisAppendState {
    /// Whether the control should be drawn in the UI.
    #[inline]
    pub(crate) fn is_visible(&self) -> bool {
        self.is_active || self.buffer_zone_hovered || self.button_hovered
    }

    /// Whether pointer is inside any interactive sub-region (to stay visible).
    #[inline]
    pub(crate) fn is_cursor_inside(&self) -> bool {
        self.edge_hovered || self.buffer_zone_hovered || self.button_hovered
    }

    /// Updates hover status for a specific region.
    pub(crate) fn set_region_hovered(&mut self, region: TableHoverRegion, hovered: bool) -> bool {
        let changed = match region {
            TableHoverRegion::Edge if self.edge_hovered != hovered => {
                self.edge_hovered = hovered;
                true
            }
            TableHoverRegion::BufferZone if self.buffer_zone_hovered != hovered => {
                self.buffer_zone_hovered = hovered;
                true
            }
            TableHoverRegion::AppendButton if self.button_hovered != hovered => {
                self.button_hovered = hovered;
                true
            }
            _ => false,
        };

        if self.is_cursor_inside() {
            self.dismiss_task = None;
            if !self.is_active {
                self.is_active = true;
                return true;
            }
        }
        changed
    }

    /// Resets all hover flags and cancels pending dismiss task.
    pub(crate) fn reset(&mut self) {
        self.edge_hovered = false;
        self.buffer_zone_hovered = false;
        self.button_hovered = false;
        self.is_active = false;
        self.dismiss_task = None;
    }
}

/// Table append controls and hovered row/column state.
#[derive(Default)]
pub(crate) struct TableInteractionState {
    /// Column append controls (right edge and + button).
    pub(crate) column_append: TableAxisAppendState,
    /// Row append controls (bottom edge and + button).
    pub(crate) row_append: TableAxisAppendState,
    /// Which row is being hovered (for handle visibility).
    pub(crate) hovered_row: Option<usize>,
    /// Which column is being hovered (for handle visibility).
    pub(crate) hovered_column: Option<usize>,
    /// Boundary index for inserting a column (0..=col_count).
    pub(crate) hovered_insert_column: Option<usize>,
    /// Boundary index for inserting a row (0..=row_count).
    pub(crate) hovered_insert_row: Option<usize>,
}

impl TableInteractionState {
    /// Clears all table interaction and hover states.
    pub(crate) fn clear(&mut self) {
        self.column_append.reset();
        self.row_append.reset();
        self.hovered_row = None;
        self.hovered_column = None;
        self.hovered_insert_column = None;
        self.hovered_insert_row = None;
    }
}

/// State for the transient code block language picker dropdown.
#[derive(Default)]
pub(crate) struct CodeLanguagePickerState {
    /// Whether the picker dropdown is open.
    pub(crate) is_open: bool,
    /// Current search filter query string.
    pub(crate) query: String,
    /// Selection range within the query input field.
    pub(crate) selected_range: Range<usize>,
    /// Whether selection is reversed (right-to-left).
    pub(crate) selection_reversed: bool,
    /// IME composition marked range.
    pub(crate) marked_range: Option<Range<usize>>,
    /// Whether mouse is actively selecting query text.
    pub(crate) is_selecting: bool,
    /// Per-pane paints of the code-language input, resolved by pointer containment.
    pub(crate) paints: Vec<CodeLanguageLastPaint>,
    /// Scroll handle for the language picker options list.
    pub(crate) scroll_handle: ScrollHandle,
}

impl CodeLanguagePickerState {
    /// Opens the picker and resets the search query and selection.
    pub(crate) fn open(&mut self) {
        self.is_open = true;
        self.query.clear();
        self.reset_selection();
        self.scroll_handle = ScrollHandle::new();
    }

    /// Closes the picker and resets search/selection state.
    pub(crate) fn close(&mut self) {
        self.is_open = false;
        self.query.clear();
        self.reset_selection();
        self.paints.clear();
    }

    /// Resets the selection and marked range.
    pub(crate) fn reset_selection(&mut self) {
        self.selected_range = 0..0;
        self.selection_reversed = false;
        self.marked_range = None;
        self.is_selecting = false;
    }

    /// Offset of the cursor (anchor/head depending on direction).
    #[inline]
    pub(crate) fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    /// Record one pane's paint of the code-language input.
    pub(crate) fn push_paint(&mut self, bounds: Bounds<Pixels>, line: ShapedLine) {
        if let Some(entry) = self.paints.iter_mut().find(|entry| entry.bounds == bounds) {
            *entry = CodeLanguageLastPaint { bounds, line };
            return;
        }
        self.paints.push(CodeLanguageLastPaint { bounds, line });
        if self.paints.len() > MAX_LAST_PAINTS {
            self.paints.remove(0);
        }
    }

    /// The paint entry whose bounds contain `position`.
    pub(crate) fn paint_at(&self, position: Point<Pixels>) -> Option<&CodeLanguageLastPaint> {
        self.paints
            .iter()
            .rev()
            .find(|entry| entry.bounds.contains(&position))
            .or_else(|| self.paints.last())
    }
}

/// Code block toolbar and language picker state.
#[derive(Default)]
pub(crate) struct CodeToolbarState {
    /// Whether the code block toolbar is hovered/visible.
    pub(crate) is_hovered: bool,
    /// Embedded language picker popup state.
    pub(crate) picker: CodeLanguagePickerState,
}

/// Structural hierarchy context assigned to a block during tree synchronization.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockStructureContext {
    pub render_depth: usize,
    pub quote_depth: usize,
    pub quote_group_id: Option<BlockId>,
    pub visible_quote_depth: usize,
    pub visible_quote_group_id: Option<BlockId>,
    pub callout_depth: usize,
    pub callout_group_id: Option<BlockId>,
    pub callout_variant: Option<CalloutKind>,
    pub footnote_group_id: Option<BlockId>,
    pub parent_is_list_item: bool,
    pub list_ordinal: Option<usize>,
    pub list_group_separator_candidate: bool,
    pub numbered_list_restart_requested: bool,
    pub quote_reparse_requested: bool,
    pub tree_metadata_flags: u8,
}

pub(crate) struct BlockLastPaint {
    pub bounds: Bounds<Pixels>,
    pub layout: Vec<WrappedLine>,
    pub line_height: Pixels,
}

/// Geometry and shaped line of one pane's paint of the code-language input.
///
/// The same block entity renders in every Wysiwyg pane, so the input keeps
/// one entry per pane, resolved by pointer containment like
/// [`BlockLastPaint`].
pub(crate) struct CodeLanguageLastPaint {
    pub bounds: Bounds<Pixels>,
    pub line: ShapedLine,
}

/// Upper bound on retained paint entries; pane counts are tiny (typically
/// 1–2) and mirror panes scroll together, so eviction only ever drops stale
/// geometry.
pub(crate) const MAX_LAST_PAINTS: usize = 8;

pub(crate) type ProjectionCacheKey = (bool, Option<u8>, Range<usize>, Option<Range<usize>>);
