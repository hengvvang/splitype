//! The Block entity: core AST unit in the document tree.

pub mod code_language;
pub mod edit_mode;
pub mod events;
pub mod footnotes;
pub mod ime;
pub mod image;
pub mod metadata;
pub mod mutations;
pub mod navigation;
pub mod paint_cache;
pub mod queries;
pub mod state;
pub mod table_cell;

pub use edit_mode::BlockEditMode;
pub use footnotes::FootnoteMap;
pub use navigation::normalize_code_language_input;
pub use state::*;

use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use gpui::*;

use crate::document::protocol::BlockEvent;
use crate::table_grid::TableGrid;
use crate::highlight::CodeHighlightResult;
use crate::mermaid::MermaidSvgRender;
use crate::projection::ExpandedInlineProjection;
use crate::markdown::block::CalloutKind;
use crate::markdown::block::image::ImageReferenceDefinitions;
use crate::markdown::block::link::LinkReferenceDefinitions;
use crate::markdown::block::table::{TableAxisMarker, TableCellPosition, TableColumnAlignment};
use crate::markdown::inline::render_cache::InlineRenderCache;
use crate::markdown::parse::BlockId;
use crate::markdown::parse::{BlockData, BlockKind};

impl EventEmitter<BlockEvent> for Block {}

/// A single editable block in the document tree.
///
/// Each block holds a [`BlockData`] containing the persistent data (kind,
/// text, UUIDs) and a [`FocusHandle`] for keyboard routing.  Runtime state
/// such as selection, cursor blink, and layout cache live on the struct.
///
/// Blocks delegate structural operations (split, merge, indent, delete) to
/// the parent editor via `BlockEvent` emissions.
pub struct Block {
    pub data: BlockData,
    pub render_cache: InlineRenderCache,
    pub code_highlight: Option<CodeHighlightResult>,
    pub children: Vec<Entity<Block>>,
    pub focus_handle: FocusHandle,
    pub code_language_focus_handle: FocusHandle,
    pub code_toolbar: CodeToolbarState,
    pub selected_range: Range<usize>,
    pub selection_reversed: bool,
    pub editor_selection_range: Option<Range<usize>>,
    pub marked_range: Option<Range<usize>>,
    /// Geometry and layout of the last frame's paints, one entry per pane
    /// that rendered this block. The same block entity renders in every
    /// Wysiwyg pane, so a single `(bounds, layout)` pair would only match
    /// the pane painted last; pointer hit-testing resolves the entry whose
    /// bounds contain the pointer instead.
    pub last_paints: Vec<BlockLastPaint>,
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
    pub is_selecting: bool,
    pub cursor_blink_epoch: Instant,
    pub vertical_motion_x: Option<Pixels>,
    pub cursor_blink_task: Option<Task<()>>,
    /// Cached projection used to show editable inline delimiters for the
    /// currently touched inline span(s).
    pub projection: Option<ExpandedInlineProjection>,
    /// Inputs that produced the current `projection`. When the next
    /// `sync_inline_projection_for_focus` computes the same inputs, the
    /// rebuild is skipped — saves a full O(fragments + text) walk per
    /// render frame (cursor blink + every arrow keypress).
    pub projection_cache_key: Option<ProjectionCacheKey>,
    /// Display text held as a SharedString so renders can clone an Arc
    /// instead of re-allocating per frame. Refreshed in `sync_render_cache`,
    /// `rebuild_inline_projection`, and `clear_inline_projection`.
    pub cached_display_text: SharedString,
    pub collapsed_caret_affinity: CollapsedCaretAffinity,
    /// Editing semantics of the block's text: rendered rich text or
    /// verbatim source/code text (shortcuts and inline formatting
    /// suppressed, delimiters stored as-is).
    pub edit_mode: BlockEditMode,
    show_source_line_numbers: bool,
    pub show_code_line_numbers: bool,
    pub table_grid: Option<TableGrid>,
    pub table_cell_position: Option<TableCellPosition>,
    pub table_cell_alignment: Option<TableColumnAlignment>,
    pub table_axis_preview: Option<TableAxisMarker>,
    pub table_axis_selection: Option<TableAxisMarker>,
    pub table_interaction: TableInteractionState,
    pub image_handle: Option<ImageHandle>,
    pub html_details_open: bool,
    pub image_base_dir: Option<PathBuf>,
    pub image_reference_definitions: Arc<ImageReferenceDefinitions>,
    pub link_reference_definitions: Arc<LinkReferenceDefinitions>,
    pub footnote_registry: Arc<FootnoteMap>,
    /// Footnote id currently hovered inside this block's text (reference
    /// hovers), used to avoid re-emitting tooltip events on every move.
    pub hovered_footnote_id: Option<String>,
    pub list_group_separator_candidate: bool,
    pub numbered_list_restart_requested: bool,
    pub quote_reparse_requested: bool,
    /// Kind-trait snapshot written by the tree-metadata rebuild; detects kind
    /// changes (e.g. quote -> paragraph downgrades) that need a metadata
    /// refresh even though the structure itself is unchanged.
    pub tree_metadata_flags: u8,
    /// Last Mermaid display render for this block, keyed by content
    /// fingerprint and the display parameters that produced it. Rendering
    /// and the surrounding SVG file reads are skipped while the key holds.
    pub mermaid_render_cache: Option<(u64, u32, u32, MermaidSvgRender)>,
    pub search_matches: Vec<(Range<usize>, bool)>,
}

impl Block {
    pub fn with_data(cx: &mut App, data: BlockData) -> Self {
        let edit_mode = BlockEditMode::for_kind(&data.kind);
        let render_cache = if let BlockKind::Callout(variant) = &data.kind
            && data.text.plain_text().is_empty()
        {
            InlineRenderCache::plain(format!("[!{}]", variant.marker_lower()))
        } else {
            data.text.render_cache()
        };
        let mut block = Self {
            data,
            render_cache,
            code_highlight: None,
            children: Vec::new(),
            focus_handle: cx.focus_handle(),
            code_language_focus_handle: cx.focus_handle(),
            code_toolbar: CodeToolbarState::default(),
            selected_range: 0..0,
            selection_reversed: false,
            editor_selection_range: None,
            marked_range: None,
            last_paints: Vec::new(),
            render_depth: 0,
            quote_depth: 0,
            quote_group_id: None,
            visible_quote_depth: 0,
            visible_quote_group_id: None,
            callout_depth: 0,
            callout_group_id: None,
            callout_variant: None,
            footnote_group_id: None,
            parent_is_list_item: false,
            list_ordinal: None,
            is_selecting: false,
            cursor_blink_epoch: Instant::now(),
            vertical_motion_x: None,
            cursor_blink_task: None,
            projection: None,
            projection_cache_key: None,
            cached_display_text: SharedString::default(),
            collapsed_caret_affinity: CollapsedCaretAffinity::Default,
            edit_mode,
            show_source_line_numbers: false,
            show_code_line_numbers: false,
            table_grid: None,
            table_cell_position: None,
            table_cell_alignment: None,
            table_axis_preview: None,
            table_axis_selection: None,
            table_interaction: TableInteractionState::default(),
            image_handle: None,
            html_details_open: false,
            image_base_dir: None,
            image_reference_definitions: Arc::default(),
            link_reference_definitions: Arc::default(),
            footnote_registry: Arc::default(),
            hovered_footnote_id: None,
            list_group_separator_candidate: false,
            numbered_list_restart_requested: false,
            quote_reparse_requested: false,
            tree_metadata_flags: 0,
            mermaid_render_cache: None,
            search_matches: Vec::new(),
        };
        block.sync_code_highlight();
        block.refresh_cached_display_text();
        block
    }

    pub fn kind(&self) -> BlockKind {
        self.data.kind.clone()
    }

    pub fn is_verbatim_mode(&self) -> bool {
        self.edit_mode == BlockEditMode::Verbatim
    }

    pub fn show_source_line_numbers(&self) -> bool {
        self.show_source_line_numbers
    }

    pub fn set_reference_context(
        &mut self,
        base_dir: Option<PathBuf>,
        image_reference_definitions: Arc<ImageReferenceDefinitions>,
        link_reference_definitions: Arc<LinkReferenceDefinitions>,
        footnote_registry: Arc<FootnoteMap>,
    ) -> bool {
        let mut changed = false;
        if self.image_base_dir != base_dir {
            self.image_base_dir = base_dir;
            changed = true;
        }
        if self.image_reference_definitions != image_reference_definitions {
            self.image_reference_definitions = image_reference_definitions;
            changed = true;
        }
        changed |= self.sync_link_reference_definitions(link_reference_definitions);
        changed |= self.sync_footnote_registry(footnote_registry);
        changed |= self.sync_image_handle();
        changed
    }

    pub fn apply_structure_context(&mut self, ctx: BlockStructureContext) {
        self.render_depth = ctx.render_depth;
        self.quote_depth = ctx.quote_depth;
        self.quote_group_id = ctx.quote_group_id;
        self.visible_quote_depth = ctx.visible_quote_depth;
        self.visible_quote_group_id = ctx.visible_quote_group_id;
        self.callout_depth = ctx.callout_depth;
        self.callout_group_id = ctx.callout_group_id;
        self.callout_variant = ctx.callout_variant;
        self.footnote_group_id = ctx.footnote_group_id;
        self.parent_is_list_item = ctx.parent_is_list_item;
        self.list_ordinal = ctx.list_ordinal;
        self.list_group_separator_candidate = ctx.list_group_separator_candidate;
        self.tree_metadata_flags = ctx.tree_metadata_flags;
    }

    pub fn edits_verbatim_text(&self) -> bool {
        self.edit_mode.edits_verbatim_text()
    }

    pub fn set_verbatim_mode(&mut self) {
        self.clear_inline_projection();
        self.edit_mode = BlockEditMode::Verbatim;
        self.show_source_line_numbers = false;
    }

    pub fn set_source_document_mode(&mut self) {
        self.set_verbatim_mode();
        self.show_source_line_numbers = true;
    }

    pub fn sync_edit_mode_from_kind(&mut self) {
        if self.table_cell_position.is_some() {
            self.edit_mode = BlockEditMode::RenderedRich;
            self.show_source_line_numbers = false;
            return;
        }
        if self.edit_mode != BlockEditMode::Verbatim {
            if self.kind().is_code_block() {
                self.clear_inline_projection();
            }
            self.edit_mode = BlockEditMode::for_kind(&self.data.kind);
            self.show_source_line_numbers = false;
        }
    }
}
