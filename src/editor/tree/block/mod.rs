//! The Block entity: core AST unit in the document tree.

pub(crate) mod mutations;
pub(crate) mod navigation;
pub(crate) mod paint_cache;
pub(crate) mod queries;
pub(crate) mod state;

pub(crate) use navigation::normalize_code_language_input;
pub(crate) use state::*;

use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use gpui::*;

use crate::editor::block_protocol::BlockEvent;
use crate::editor::editing::projection::ExpandedInlineProjection;
use crate::editor::editing::table_grid::TableGrid;
use crate::editor::render::code_highlight::highlight::CodeHighlightResult;
use crate::editor::render::mermaid_render::MermaidSvgRender;
use crate::editor::tree::block_edit_mode::BlockEditMode;
use crate::editor::tree::footnotes::FootnoteMap;
use crate::model::block::CalloutKind;
use crate::model::block::image::ImageReferenceDefinitions;
use crate::model::block::link::LinkReferenceDefinitions;
use crate::model::block::table::TableAxisHighlight;
use crate::model::block::table::{TableAxisMarker, TableCellPosition, TableColumnAlignment};
use crate::model::inline::render_cache::InlineRenderCache;
use crate::model::parse::BlockId;
use crate::model::parse::{BlockData, BlockKind};

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
    pub(crate) render_cache: InlineRenderCache,
    pub(crate) code_highlight: Option<CodeHighlightResult>,
    pub children: Vec<Entity<Block>>,
    pub focus_handle: FocusHandle,
    pub(crate) code_language_focus_handle: FocusHandle,
    pub(crate) code_toolbar: CodeToolbarState,
    pub selected_range: Range<usize>,
    pub selection_reversed: bool,
    pub(crate) editor_selection_range: Option<Range<usize>>,
    pub marked_range: Option<Range<usize>>,
    /// Geometry and layout of the last frame's paints, one entry per pane
    /// that rendered this block. The same block entity renders in every
    /// Wysiwyg pane, so a single `(bounds, layout)` pair would only match
    /// the pane painted last; pointer hit-testing resolves the entry whose
    /// bounds contain the pointer instead.
    pub(crate) last_paints: Vec<BlockLastPaint>,
    pub render_depth: usize,
    pub quote_depth: usize,
    pub(crate) quote_group_id: Option<BlockId>,
    pub(crate) visible_quote_depth: usize,
    pub(crate) visible_quote_group_id: Option<BlockId>,
    pub(crate) callout_depth: usize,
    pub(crate) callout_group_id: Option<BlockId>,
    pub(crate) callout_variant: Option<CalloutKind>,
    pub(crate) footnote_group_id: Option<BlockId>,
    pub(crate) parent_is_list_item: bool,
    pub list_ordinal: Option<usize>,
    pub is_selecting: bool,
    pub cursor_blink_epoch: Instant,
    pub vertical_motion_x: Option<Pixels>,
    pub(crate) cursor_blink_task: Option<Task<()>>,
    /// Cached projection used to show editable inline delimiters for the
    /// currently touched inline span(s).
    pub(crate) projection: Option<ExpandedInlineProjection>,
    /// Inputs that produced the current `projection`. When the next
    /// `sync_inline_projection_for_focus` computes the same inputs, the
    /// rebuild is skipped — saves a full O(fragments + text) walk per
    /// render frame (cursor blink + every arrow keypress).
    pub(crate) projection_cache_key: Option<ProjectionCacheKey>,
    /// Display text held as a SharedString so renders can clone an Arc
    /// instead of re-allocating per frame. Refreshed in `sync_render_cache`,
    /// `rebuild_inline_projection`, and `clear_inline_projection`.
    pub(crate) cached_display_text: SharedString,
    pub(crate) collapsed_caret_affinity: CollapsedCaretAffinity,
    /// Editing semantics of the block's text: rendered rich text or
    /// verbatim source/code text (shortcuts and inline formatting
    /// suppressed, delimiters stored as-is).
    pub(crate) edit_mode: BlockEditMode,
    show_source_line_numbers: bool,
    pub(crate) show_code_line_numbers: bool,
    pub(crate) table_grid: Option<TableGrid>,
    pub(crate) table_cell_position: Option<TableCellPosition>,
    pub(crate) table_cell_alignment: Option<TableColumnAlignment>,
    pub(crate) table_axis_preview: Option<TableAxisMarker>,
    pub(crate) table_axis_selection: Option<TableAxisMarker>,
    pub(crate) table_axis_highlight: TableAxisHighlight,
    pub(crate) table_interaction: TableInteractionState,
    pub(crate) image_handle: Option<ImageHandle>,
    pub(crate) html_details_open: bool,
    pub(crate) image_base_dir: Option<PathBuf>,
    pub(crate) image_reference_definitions: Arc<ImageReferenceDefinitions>,
    pub(crate) link_reference_definitions: Arc<LinkReferenceDefinitions>,
    pub(crate) footnote_registry: Arc<FootnoteMap>,
    /// Footnote id currently hovered inside this block's text (reference
    /// hovers), used to avoid re-emitting tooltip events on every move.
    pub(crate) hovered_footnote_id: Option<String>,
    pub(crate) list_group_separator_candidate: bool,
    pub(crate) numbered_list_restart_requested: bool,
    pub(crate) quote_reparse_requested: bool,
    /// Kind-trait snapshot written by the tree-metadata rebuild; detects kind
    /// changes (e.g. quote -> paragraph downgrades) that need a metadata
    /// refresh even though the structure itself is unchanged.
    pub(crate) tree_metadata_flags: u8,
    /// Last Mermaid display render for this block, keyed by content
    /// fingerprint and the display parameters that produced it. Rendering
    /// and the surrounding SVG file reads are skipped while the key holds.
    pub(crate) mermaid_render_cache: Option<(u64, u32, u32, MermaidSvgRender)>,
}

impl Block {
    pub fn with_data(cx: &mut Context<Self>, data: BlockData) -> Self {
        let edit_mode = BlockEditMode::for_kind(&data.kind);
        let render_cache = if let BlockKind::Callout(variant) = &data.kind
            && data.text.plain_text().is_empty()
        {
            InlineRenderCache::plain(variant.marker_lower())
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
            table_axis_highlight: TableAxisHighlight::None,
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
        };
        block.sync_code_highlight();
        block.refresh_cached_display_text();
        block
    }

    pub fn kind(&self) -> BlockKind {
        self.data.kind.clone()
    }

    pub(crate) fn is_verbatim_mode(&self) -> bool {
        self.edit_mode == BlockEditMode::Verbatim
    }

    pub(crate) fn show_source_line_numbers(&self) -> bool {
        self.show_source_line_numbers
    }

    pub(crate) fn set_reference_context(
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

    pub(crate) fn apply_structure_context(&mut self, ctx: BlockStructureContext) {
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

    pub(crate) fn edits_verbatim_text(&self) -> bool {
        self.edit_mode.edits_verbatim_text()
    }

    pub(crate) fn set_verbatim_mode(&mut self) {
        self.clear_inline_projection();
        self.edit_mode = BlockEditMode::Verbatim;
        self.show_source_line_numbers = false;
    }

    pub(crate) fn set_source_document_mode(&mut self) {
        self.set_verbatim_mode();
        self.show_source_line_numbers = true;
    }

    pub(crate) fn sync_edit_mode_from_kind(&mut self) {
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
