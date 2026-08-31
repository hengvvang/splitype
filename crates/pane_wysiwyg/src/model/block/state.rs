//! Block state types: interaction states, append controls, and paint records.

use std::ops::Range;

use gpui::*;

use crate::markdown::block::CalloutKind;
use crate::markdown::block::image::ImageResolvedSource;
use crate::markdown::parse::BlockId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InlineFormat {
    Bold,
    Italic,
    Underline,
    Code,
    Strikethrough,
}

/// Cached standalone image presentation state for a block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageHandle {
    pub alt: String,
    pub src: String,
    pub title: Option<String>,
    pub resolved_source: ImageResolvedSource,
}

/// How a collapsed caret at an inline projection boundary inherits style.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CollapsedCaretAffinity {
    #[default]
    Default,
    OuterStart,
    OuterEnd,
}

/// Interactive sub-regions of table append controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableHoverRegion {
    /// Border indicator line.
    Edge,
    /// Anti-flicker buffer zone.
    BufferZone,
    /// The append (+) button itself.
    AppendButton,
}

/// Hover and close scheduling state for one axis (row or column) of table append controls.
#[derive(Default)]
pub struct TableAxisAppendState {
    pub edge_hovered: bool,
    pub buffer_zone_hovered: bool,
    pub button_hovered: bool,
    /// Overall control visibility/active state.
    pub is_active: bool,
    /// Delayed dismiss debounce task handle (120ms).
    pub dismiss_task: Option<Task<()>>,
}

impl TableAxisAppendState {
    /// Whether the control should be drawn in the UI.
    #[inline]
    pub fn is_visible(&self) -> bool {
        self.is_active || self.buffer_zone_hovered || self.button_hovered
    }

    /// Whether pointer is inside any interactive sub-region (to stay visible).
    #[inline]
    pub fn is_cursor_inside(&self) -> bool {
        self.edge_hovered || self.buffer_zone_hovered || self.button_hovered
    }

    /// Updates hover status for a specific region.
    pub fn set_region_hovered(&mut self, region: TableHoverRegion, hovered: bool) -> bool {
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
    pub fn reset(&mut self) {
        self.edge_hovered = false;
        self.buffer_zone_hovered = false;
        self.button_hovered = false;
        self.is_active = false;
        self.dismiss_task = None;
    }
}

/// Table append controls and hovered row/column state.
#[derive(Default)]
pub struct TableInteractionState {
    /// Column append controls (right edge and + button).
    pub column_append: TableAxisAppendState,
    /// Row append controls (bottom edge and + button).
    pub row_append: TableAxisAppendState,
    /// Which row is being hovered (for handle visibility).
    pub hovered_row: Option<usize>,
    /// Which column is being hovered (for handle visibility).
    pub hovered_column: Option<usize>,
}

impl TableInteractionState {
    /// Clears all table interaction and hover states.
    pub fn clear(&mut self) {
        self.column_append.reset();
        self.row_append.reset();
        self.hovered_row = None;
        self.hovered_column = None;
    }
}

/// State for the transient code block language picker dropdown.
#[derive(Default)]
pub struct CodeLanguagePickerState {
    /// Whether the picker dropdown is open.
    pub is_open: bool,
    /// Current search filter query string.
    pub query: String,
    /// Selection range within the query input field.
    pub selected_range: Range<usize>,
    /// Whether selection is reversed (right-to-left).
    pub selection_reversed: bool,
    /// IME composition marked range.
    pub marked_range: Option<Range<usize>>,
    /// Whether mouse is actively selecting query text.
    pub is_selecting: bool,
    /// Per-pane paints of the code-language input, resolved by pointer containment.
    pub paints: Vec<CodeLanguageLastPaint>,
    /// Scroll handle for the language picker options list.
    pub scroll_handle: ScrollHandle,
}

impl CodeLanguagePickerState {
    /// Opens the picker and resets the search query and selection.
    pub fn open(&mut self) {
        self.is_open = true;
        self.query.clear();
        self.reset_selection();
        self.scroll_handle = ScrollHandle::new();
    }

    /// Closes the picker and resets search/selection state.
    pub fn close(&mut self) {
        self.is_open = false;
        self.query.clear();
        self.reset_selection();
        self.paints.clear();
    }

    /// Resets the selection and marked range.
    pub fn reset_selection(&mut self) {
        self.selected_range = 0..0;
        self.selection_reversed = false;
        self.marked_range = None;
        self.is_selecting = false;
    }

    /// Offset of the cursor (anchor/head depending on direction).
    #[inline]
    pub fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    /// Record one pane's paint of the code-language input.
    pub fn push_paint(&mut self, bounds: Bounds<Pixels>, line: ShapedLine) {
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
    pub fn paint_at(&self, position: Point<Pixels>) -> Option<&CodeLanguageLastPaint> {
        self.paints
            .iter()
            .rev()
            .find(|entry| entry.bounds.contains(&position))
            .or_else(|| self.paints.last())
    }
}

/// Code block toolbar and language picker state.
#[derive(Default)]
pub struct CodeToolbarState {
    /// Whether the code block toolbar is hovered/visible.
    pub is_hovered: bool,
    /// Embedded language picker popup state.
    pub picker: CodeLanguagePickerState,
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

pub struct BlockLastPaint {
    pub bounds: Bounds<Pixels>,
    pub layout: Vec<WrappedLine>,
    pub line_height: Pixels,
}

/// Geometry and shaped line of one pane's paint of the code-language input.
///
/// The same block entity renders in every Wysiwyg pane, so the input keeps
/// one entry per pane, resolved by pointer containment like
/// [`BlockLastPaint`].
pub struct CodeLanguageLastPaint {
    pub bounds: Bounds<Pixels>,
    pub line: ShapedLine,
}

/// Upper bound on retained paint entries; pane counts are tiny (typically
/// 1–2) and mirror panes scroll together, so eviction only ever drops stale
/// geometry.
pub const MAX_LAST_PAINTS: usize = 8;

pub type ProjectionCacheKey = (bool, Option<u8>, Range<usize>, Option<Range<usize>>);


