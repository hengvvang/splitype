//! BlockView — GPUI View for a single block node in the document tree.

use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::*;
use unicode_segmentation::*;

use crate::editor::block_protocol::{BlockAction, UndoCaptureKind};
use crate::editor::editing::projection::ExpandedInlineProjection;
use crate::editor::editing::table_grid::TableGrid;
use crate::editor::geometry::text_layout as element;
use crate::editor::render::code_highlight::highlight::CodeHighlightResult;
use crate::editor::render::mermaid_render::MermaidSvgRender;
use crate::editor::tree::block_edit_mode::BlockEditMode;
use crate::editor::tree::footnotes::FootnoteMap;
use crate::model::block::CalloutKind;
use crate::model::block::image::ImageReferenceDefinitions;
use crate::model::block::image::ImageResolvedSource;
use crate::model::block::link::LinkReferenceDefinitions;
use crate::model::block::table::TableCellPosition;
use crate::model::block::table::{TableAxisHighlight, TableAxisMarker, TableColumnAlignment};
use crate::model::inline::render_cache::{InlineRenderCache, InlineSpan};
#[cfg(test)]
use crate::model::inline::style::InlineStyle;
use crate::model::inline::text::BlockText;
use crate::model::parse::{BlockData, BlockId, BlockKind};

// ---------------------------------------------------------------------------
// View-local types
// ---------------------------------------------------------------------------

/// Inline formatting command issued by editor actions.
#[derive(Clone, Copy)]
pub(crate) enum InlineFormat {
    Bold,
    Italic,
    Underline,
    Code,
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

impl EventEmitter<BlockAction> for Block {}

/// A single editable block in the document tree.
///
/// Each block holds a [`BlockData`] containing the persistent data (kind,
/// text, UUIDs) and a [`FocusHandle`] for keyboard routing.  Runtime state
/// such as selection, cursor blink, and layout cache live on the struct.
///
/// Blocks delegate structural operations (split, merge, indent, delete) to
/// the parent editor via `BlockAction` emissions.
pub struct Block {
    pub data: BlockData,
    pub(crate) render_cache: InlineRenderCache,
    pub(crate) code_highlight: Option<CodeHighlightResult>,
    pub children: Vec<Entity<Block>>,
    pub focus_handle: FocusHandle,
    pub(crate) code_language_focus_handle: FocusHandle,
    pub(crate) code_language_selected_range: Range<usize>,
    pub(crate) code_language_selection_reversed: bool,
    pub(crate) code_language_marked_range: Option<Range<usize>>,
    /// Per-pane paints of the code-language input, resolved by pointer
    /// containment like [`Block::last_paints`].
    pub(crate) code_language_paints: Vec<CodeLanguageLastPaint>,
    pub(crate) code_language_is_selecting: bool,
    pub(crate) code_toolbar_hovered: bool,
    pub(crate) code_language_picker_open: bool,
    pub(crate) latex_template_picker_open: bool,
    pub(crate) code_language_query: String,
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
    pub(crate) projection_cache_key: Option<(bool, Option<u8>, Range<usize>, Option<Range<usize>>)>,
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
    pub(crate) table_append_column_edge_hovered: bool,
    pub(crate) table_append_column_hovered: bool,
    pub(crate) table_append_column_zone_hovered: bool,
    pub(crate) table_append_column_button_hovered: bool,
    pub(crate) table_append_column_close_task: Option<Task<()>>,
    pub(crate) table_append_row_edge_hovered: bool,
    pub(crate) table_append_row_hovered: bool,
    pub(crate) table_append_row_zone_hovered: bool,
    pub(crate) table_append_row_button_hovered: bool,
    pub(crate) table_append_row_close_task: Option<Task<()>>,
    /// Which row is being hovered (for Anytype-style handle visibility).
    pub(crate) table_hovered_row: Option<usize>,
    /// Which column is being hovered (for Anytype-style handle visibility).
    pub(crate) table_hovered_column: Option<usize>,
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

/// Geometry and text layout of one pane's paint of a block during the last
/// frame.
///
/// The same block entity renders in every Wysiwyg pane, so `Block` keeps one
/// entry per pane instead of a single `(bounds, layout)` pair — the single
/// pair would only ever match the pane painted last, breaking pointer
/// hit-testing in every other pane.
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
const MAX_LAST_PAINTS: usize = 8;

impl Block {
    /// Record one pane's paint of this block. Replaces an entry with the
    /// same bounds (the pane re-painted unchanged geometry) and otherwise
    /// appends, bounding the retained history to the most recent paints.
    pub(crate) fn push_last_paint(
        &mut self,
        bounds: Bounds<Pixels>,
        layout: Vec<WrappedLine>,
        line_height: Pixels,
    ) {
        if let Some(entry) = self
            .last_paints
            .iter_mut()
            .find(|entry| entry.bounds == bounds)
        {
            *entry = BlockLastPaint {
                bounds,
                layout,
                line_height,
            };
            return;
        }
        self.last_paints.push(BlockLastPaint {
            bounds,
            layout,
            line_height,
        });
        if self.last_paints.len() > MAX_LAST_PAINTS {
            self.last_paints.remove(0);
        }
    }

    /// The paint entry whose bounds contain `position` — the pane the
    /// pointer is inside. Falls back to the newest entry.
    pub(crate) fn last_paint_at(&self, position: Point<Pixels>) -> Option<&BlockLastPaint> {
        self.last_paints
            .iter()
            .rev()
            .find(|entry| entry.bounds.contains(&position))
            .or_else(|| self.last_paints.last())
    }

    /// The newest paint entry (any pane). Used where no pointer position is
    /// available — keyboard navigation, IME popup placement, row strides.
    pub(crate) fn last_paint(&self) -> Option<&BlockLastPaint> {
        self.last_paints.last()
    }

    /// Record one pane's paint of the code-language input, mirroring
    /// [`Self::push_last_paint`].
    pub(crate) fn push_code_language_paint(&mut self, bounds: Bounds<Pixels>, line: ShapedLine) {
        if let Some(entry) = self
            .code_language_paints
            .iter_mut()
            .find(|entry| entry.bounds == bounds)
        {
            *entry = CodeLanguageLastPaint { bounds, line };
            return;
        }
        self.code_language_paints
            .push(CodeLanguageLastPaint { bounds, line });
        if self.code_language_paints.len() > MAX_LAST_PAINTS {
            self.code_language_paints.remove(0);
        }
    }

    /// The code-language input paint whose bounds contain `position`.
    pub(crate) fn code_language_paint_at(
        &self,
        position: Point<Pixels>,
    ) -> Option<&CodeLanguageLastPaint> {
        self.code_language_paints
            .iter()
            .rev()
            .find(|entry| entry.bounds.contains(&position))
            .or_else(|| self.code_language_paints.last())
    }

    /// The newest code-language input paint (any pane).
    pub(crate) fn code_language_paint(&self) -> Option<&CodeLanguageLastPaint> {
        self.code_language_paints.last()
    }

    pub fn with_data(cx: &mut Context<Self>, data: BlockData) -> Self {
        let edit_mode = BlockEditMode::for_kind(&data.kind);
        let render_cache = data.text.render_cache();
        let mut block = Self {
            data,
            render_cache,
            code_highlight: None,
            children: Vec::new(),
            focus_handle: cx.focus_handle(),
            code_language_focus_handle: cx.focus_handle(),
            code_language_selected_range: 0..0,
            code_language_selection_reversed: false,
            code_language_marked_range: None,
            code_language_paints: Vec::new(),
            code_language_is_selecting: false,
            code_toolbar_hovered: false,
            code_language_picker_open: false,
            latex_template_picker_open: false,
            code_language_query: String::new(),
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
            table_append_column_edge_hovered: false,
            table_append_column_hovered: false,
            table_append_column_zone_hovered: false,
            table_append_column_button_hovered: false,
            table_append_column_close_task: None,
            table_append_row_edge_hovered: false,
            table_append_row_hovered: false,
            table_append_row_zone_hovered: false,
            table_append_row_button_hovered: false,
            table_append_row_close_task: None,
            table_hovered_row: None,
            table_hovered_column: None,
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

    /// Flags describing the block-kind traits that tree metadata (quote
    /// depths, anchors, list ordinals, footnote anchors) derives from.
    /// Written by the tree-metadata rebuild; compared on text edits to detect
    /// kind changes that require a metadata refresh without a structural edit.
    pub fn display_text(&self) -> &str {
        self.display_cache().text()
    }

    /// Cheap clone of the current display text as a `SharedString` (Arc bump)
    /// — avoids a fresh String allocation per render. The cached value is
    /// refreshed by [`Self::refresh_cached_display_text`] whenever the
    /// underlying text might have changed.
    pub(crate) fn shared_display_text(&self) -> SharedString {
        self.cached_display_text.clone()
    }

    pub(crate) fn refresh_cached_display_text(&mut self) {
        let current = self.display_cache().text();
        if self.cached_display_text.as_ref() != current {
            self.cached_display_text = SharedString::from(current.to_string());
        }
    }

    pub(crate) fn inline_tree_from_markdown_with_context(&self, markdown: &str) -> BlockText {
        BlockText::from_markdown_with_link_references(markdown, &self.link_reference_definitions)
    }

    pub fn inline_spans(&self) -> &[InlineSpan] {
        self.display_cache().spans()
    }

    #[cfg(test)]
    pub fn inline_style_at(&self, offset: usize) -> InlineStyle {
        self.display_cache().style_at(offset)
    }

    #[cfg(test)]
    pub(crate) fn inline_html_style_at(
        &self,
        offset: usize,
    ) -> Option<crate::model::inline::html::HtmlInlineStyle> {
        self.display_cache().html_style_at(offset)
    }

    #[cfg(test)]
    pub(crate) fn inline_link_at(&self, offset: usize) -> Option<&str> {
        self.display_cache().link_at(offset)
    }

    pub(crate) fn has_mixed_inline_visuals(&self) -> bool {
        self.data.text.has_mixed_inline_visuals()
    }

    pub(crate) fn footnote_definition_id(&self) -> Option<String> {
        self.kind().is_footnote_definition().then(|| {
            crate::model::block::footnote::split_footnote_definition_text(
                &self.data.text.plain_text(),
            )
            .0
            .to_string()
        })
    }

    pub(crate) fn footnote_definition_has_backref(&self) -> bool {
        self.footnote_definition_id().as_deref().is_some_and(|id| {
            self.footnote_registry
                .binding(id)
                .and_then(|binding| binding.first_reference.as_ref())
                .is_some()
        })
    }

    pub(crate) fn display_range_for_footnote_occurrence(
        &self,
        occurrence_index: usize,
    ) -> Option<Range<usize>> {
        let mut plain_offset = 0usize;
        for fragment in &self.data.text.fragments {
            let len = fragment.text.len();
            if fragment
                .footnote
                .as_ref()
                .is_some_and(|footnote| footnote.occurrence_index == occurrence_index)
            {
                return Some(self.plain_to_display_range(plain_offset..plain_offset + len));
            }
            plain_offset += len;
        }
        None
    }

    pub fn is_empty(&self) -> bool {
        self.display_text().is_empty()
    }

    pub fn is_direct_list_child(&self) -> bool {
        self.parent_is_list_item && !self.kind().is_list_item()
    }

    pub fn is_nested_list_item(&self) -> bool {
        self.parent_is_list_item && self.kind().is_list_item()
    }

    pub fn can_adjust_list_nesting(&self) -> bool {
        (self.kind().is_list_item() || self.parent_is_list_item) && !self.kind().is_code_block()
    }

    pub fn can_outdent_list_nesting(&self) -> bool {
        self.kind().is_list_item() || self.parent_is_list_item
    }

    pub(crate) fn display_len(&self) -> usize {
        self.display_cache().len()
    }

    pub(crate) fn split_text(&self, offset: usize) -> (BlockText, BlockText) {
        self.data
            .text
            .split_at(self.display_to_plain_offset(offset))
    }

    pub(crate) fn clear_vertical_motion(&mut self) {
        self.vertical_motion_x = None;
    }

    pub(crate) fn sync_render_cache(&mut self) {
        let plain_selected = self.display_to_plain_range(self.selected_range.clone());
        let plain_marked = self
            .marked_range
            .clone()
            .map(|range| self.display_to_plain_range(range));
        let (plain_anchor, plain_focus) = self.plain_selection_anchor_focus();
        let (anchor_affinity, focus_affinity) = self.selection_endpoint_affinities();
        let collapsed_affinity = self.display_collapsed_caret_affinity();
        let keep_projection =
            self.projection.is_some() && self.edit_mode.supports_inline_projection();
        self.render_cache = self.data.text.render_cache();
        self.sync_code_highlight();
        self.sync_image_handle();
        self.projection = None;
        self.projection_cache_key = None;
        if keep_projection {
            self.rebuild_inline_projection(plain_selected.clone(), plain_marked.clone());
            if plain_selected.is_empty() {
                let offset = self.plain_to_display_cursor_offset_with_affinity(
                    plain_selected.start,
                    collapsed_affinity,
                );
                self.assign_collapsed_selection_offset(offset, collapsed_affinity, None);
            } else {
                self.set_selection_from_plain_anchor_focus(
                    plain_anchor,
                    plain_focus,
                    anchor_affinity,
                    focus_affinity,
                );
                self.collapsed_caret_affinity = CollapsedCaretAffinity::Default;
            }
            self.marked_range = plain_marked.map(|range| self.plain_to_display_range(range));
        } else {
            self.set_selection_from_anchor_focus(plain_anchor, plain_focus);
            self.marked_range = plain_marked;
            self.collapsed_caret_affinity = CollapsedCaretAffinity::Default;
        }
        self.refresh_cached_display_text();
    }

    fn sync_link_reference_definitions(
        &mut self,
        link_reference_definitions: Arc<LinkReferenceDefinitions>,
    ) -> bool {
        if self.link_reference_definitions == link_reference_definitions {
            return false;
        }

        let selected_source = (!self.edits_verbatim_text())
            .then(|| self.display_range_to_source_range(self.selected_range.clone()));
        let marked_source = (!self.edits_verbatim_text())
            .then(|| {
                self.marked_range
                    .clone()
                    .map(|range| self.display_range_to_source_range(range))
            })
            .flatten();
        let selection_reversed = self.selection_reversed;
        let collapsed_affinity = self.display_collapsed_caret_affinity();
        let had_projection = self.projection.is_some();

        self.link_reference_definitions = link_reference_definitions;
        if self.edits_verbatim_text() {
            return true;
        }

        let markdown = self.data.text.serialize_markdown();
        let next_text = BlockText::from_markdown_with_link_references(
            &markdown,
            &self.link_reference_definitions,
        );
        if self.data.text == next_text {
            return true;
        }

        self.data.set_text(next_text);
        self.sync_edit_mode_from_kind();
        self.sync_render_cache();

        if let Some(selected_source) = selected_source {
            let restored = self.source_range_to_display_range(selected_source);
            if restored.is_empty() {
                self.assign_collapsed_selection_offset(
                    restored.start,
                    collapsed_affinity,
                    self.vertical_motion_x,
                );
            } else {
                self.selected_range = restored;
                self.selection_reversed = selection_reversed;
                self.collapsed_caret_affinity = CollapsedCaretAffinity::Default;
            }
        }

        self.marked_range = marked_source.map(|range| self.source_range_to_display_range(range));

        if had_projection {
            self.sync_inline_projection_for_focus(true);
        }
        true
    }

    fn sync_footnote_registry(&mut self, footnote_registry: Arc<FootnoteMap>) -> bool {
        if self.footnote_registry == footnote_registry {
            return false;
        }

        let selected_source = (!self.edits_verbatim_text())
            .then(|| self.display_range_to_source_range(self.selected_range.clone()));
        let marked_source = (!self.edits_verbatim_text())
            .then(|| {
                self.marked_range
                    .clone()
                    .map(|range| self.display_range_to_source_range(range))
            })
            .flatten();
        let selection_reversed = self.selection_reversed;
        let collapsed_affinity = self.display_collapsed_caret_affinity();
        let had_projection = self.projection.is_some();

        self.footnote_registry = footnote_registry;
        if self.edits_verbatim_text() || !self.data.text.has_footnote_references() {
            return true;
        }

        let mut next_text = self.data.text.clone();
        let mut occurrence_iter = self
            .footnote_registry
            .occurrences_for_block(self.data.id)
            .unwrap_or(&[])
            .iter();
        next_text.apply_footnote_reference_state(|id| {
            if self.footnote_registry.binding(id).is_none() {
                return None;
            }
            let occurrence = occurrence_iter.next()?;
            if occurrence.id != id {
                return None;
            }
            Some(occurrence.occurrence_index)
        });
        if self.data.text == next_text {
            return true;
        }

        self.data.set_text(next_text);
        self.sync_edit_mode_from_kind();
        self.sync_render_cache();

        if let Some(selected_source) = selected_source {
            let restored = self.source_range_to_display_range(selected_source);
            if restored.is_empty() {
                self.assign_collapsed_selection_offset(
                    restored.start,
                    collapsed_affinity,
                    self.vertical_motion_x,
                );
            } else {
                self.selected_range = restored;
                self.selection_reversed = selection_reversed;
                self.collapsed_caret_affinity = CollapsedCaretAffinity::Default;
            }
        }

        self.marked_range = marked_source.map(|range| self.source_range_to_display_range(range));

        if had_projection {
            self.sync_inline_projection_for_focus(true);
        }
        true
    }

    pub(crate) fn should_use_source_space_link_edit(&self) -> bool {
        !self.edits_verbatim_text() && self.data.text.has_source_preserving_links()
    }

    pub(crate) fn apply_source_space_text_edit(
        &mut self,
        display_range: Range<usize>,
        new_text: &str,
        selected_range_relative: Option<Range<usize>>,
        mark_inserted_text: bool,
        cx: &mut Context<Self>,
    ) {
        let old_plain_len = self.data.text.plain_text().len();
        let source_range = self.display_range_to_source_range(display_range.clone());
        let mut markdown = self.data.text.serialize_markdown();
        let replaced_text = markdown[source_range.clone()].to_string();
        markdown.replace_range(source_range.clone(), new_text);

        let next_text = BlockText::from_markdown_with_link_references(
            &markdown,
            &self.link_reference_definitions,
        );
        let map = next_text.source_offset_map();
        let selected_source = selected_range_relative
            .as_ref()
            .map(|relative| source_range.start + relative.start..source_range.start + relative.end);
        let cursor_source = selected_source
            .as_ref()
            .map(|range| range.end)
            .unwrap_or(source_range.start + new_text.len());
        let marked_source = if mark_inserted_text && !new_text.is_empty() {
            Some(source_range.start..source_range.start + new_text.len())
        } else {
            None
        };
        let selected_plain = selected_source
            .as_ref()
            .map(|range| map.source_to_plain_range(range.clone()));
        let marked_plain = marked_source
            .as_ref()
            .map(|range| map.source_to_plain_range(range.clone()));
        let cursor_plain = map.source_to_plain_offset(cursor_source);

        let quote_structure_edit = self.quote_depth > 0
            && (new_text.contains('\n')
                || replaced_text.contains('\n')
                || (self.kind() == BlockKind::Blockquote
                    && Self::multiline_quote_edit_requires_reparse(&next_text.plain_text())));
        if quote_structure_edit {
            self.quote_reparse_requested = true;
        }

        // Typing a closing marker (for example the `)` that completes a link)
        // absorbs that markup into a span, so the plain text grows by less than
        // the inserted text. Flag it so the caret is placed just past the new
        // closing delimiter instead of landing inside the span.
        let caret_may_have_closed_span = !new_text.is_empty()
            && !mark_inserted_text
            && next_text.plain_text().len() < old_plain_len + new_text.len();

        self.apply_text_edit(
            next_text,
            cursor_plain,
            marked_plain,
            selected_plain.clone(),
            selected_plain
                .as_ref()
                .and_then(|range| (!range.is_empty()).then_some(false)),
            caret_may_have_closed_span,
            cx,
        );
    }

    pub(crate) fn mark_changed(&mut self, cx: &mut Context<Self>) {
        self.sync_edit_mode_from_kind();
        self.sync_render_cache();
        self.cursor_blink_epoch = Instant::now();
        self.clear_vertical_motion();
        cx.emit(BlockAction::Changed);
        cx.notify();
    }

    pub(crate) fn convert_to_paragraph(&mut self, cx: &mut Context<Self>) {
        self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
        self.data.kind = BlockKind::Paragraph;
        self.data.raw_source = None;
        self.quote_reparse_requested = false;
        self.mark_changed(cx);
    }

    pub(crate) fn convert_to_separator(&mut self, cx: &mut Context<Self>) {
        self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
        self.make_separator();
        cx.emit(BlockAction::Changed);
        cx.notify();
    }

    /// Turns this block into a separator in place without emitting events or
    /// capturing undo, so editor-level flows that already manage those can
    /// reuse the conversion.
    pub(crate) fn make_separator(&mut self) {
        let current_text = self.display_text().to_string();
        let source_text = if current_text.trim().is_empty() {
            "---".to_string()
        } else {
            current_text
        };
        let source_len = source_text.len();
        self.clear_inline_projection();
        self.data.kind = BlockKind::ThematicBreak;
        self.data.raw_source = Some(source_text.clone());
        self.data.set_text(BlockText::plain(source_text));
        self.quote_reparse_requested = false;
        self.sync_edit_mode_from_kind();
        self.sync_render_cache();
        self.assign_collapsed_selection_offset(source_len, CollapsedCaretAffinity::Default, None);
        self.marked_range = None;
        self.cursor_blink_epoch = Instant::now();
        self.clear_vertical_motion();
    }

    pub(crate) fn enter_code_block(
        &mut self,
        language: Option<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
        self.clear_inline_projection();
        self.data.kind = BlockKind::CodeBlock { language };
        self.data.raw_source = None;
        self.data.set_text(BlockText::plain(String::new()));
        self.quote_reparse_requested = false;
        self.sync_edit_mode_from_kind();
        self.sync_render_cache();
        self.assign_collapsed_selection_offset(0, CollapsedCaretAffinity::Default, None);
        self.marked_range = None;
        self.cursor_blink_epoch = Instant::now();
        self.clear_vertical_motion();
        cx.emit(BlockAction::Changed);
        cx.notify();
    }

    /// Convert the current paragraph into a display-math block. `body` is
    /// stored as the formula source (the `$$` delimiters are rebuilt on
    /// serialization), and the caret lands at the start of the body.
    pub(crate) fn enter_math_block(&mut self, body: &str, cx: &mut Context<Self>) {
        self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
        self.clear_inline_projection();
        self.data.kind = BlockKind::MathBlock;
        self.data.set_text(BlockText::plain(body.to_string()));
        self.quote_reparse_requested = false;
        self.sync_edit_mode_from_kind();
        self.sync_render_cache();
        self.assign_collapsed_selection_offset(0, CollapsedCaretAffinity::Default, None);
        self.marked_range = None;
        self.cursor_blink_epoch = Instant::now();
        self.clear_vertical_motion();
        cx.emit(BlockAction::Changed);
        cx.notify();
    }

    /// Toggle a style flag directly on the fragment tree without ever
    /// manipulating raw marker characters.  The selection range determines
    /// which fragments have their [`InlineStyle`] flag flipped.
    ///
    /// Serializers later translate these flags back to markers on export.
    pub(crate) fn toggle_inline_format(&mut self, format: InlineFormat, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() || self.edits_verbatim_text() {
            return;
        }

        let mut next_text = self.data.text.clone();
        let selection = self.selection_plain_range();
        let changed = match format {
            InlineFormat::Bold => next_text.toggle_bold(selection.clone()),
            InlineFormat::Italic => next_text.toggle_italic(selection.clone()),
            InlineFormat::Underline => next_text.toggle_underline(selection.clone()),
            InlineFormat::Code => next_text.toggle_code(selection.clone()),
        };
        if !changed {
            return;
        }

        self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
        self.apply_text_edit(
            next_text,
            selection.end,
            None,
            Some(selection),
            Some(self.selection_reversed),
            false,
            cx,
        );
    }

    fn current_line_layout_and_offset(&self) -> Option<(&WrappedLine, usize)> {
        let paint = self.last_paint()?;
        let text = self.display_text();
        let ranges = element::hard_line_ranges(text);
        let (line_idx, offset_in_line) =
            element::line_index_for_offset(&ranges, self.cursor_offset());
        Some((paint.layout.get(line_idx)?, offset_in_line))
    }

    pub(crate) fn vertical_anchor_x(&self) -> Pixels {
        self.vertical_motion_x
            .or_else(|| {
                self.current_line_layout_and_offset()
                    .and_then(|(layout, offset_in_line)| {
                        element::position_for_offset(
                            layout,
                            offset_in_line,
                            self.last_paint().map_or(px(0.0), |p| p.line_height),
                            true,
                        )
                        .map(|position| position.x)
                    })
            })
            .unwrap_or(px(0.0))
    }

    /// Attempt to move the cursor up (direction < 0) or down one visual line
    /// within the current block.  Returns false if the cursor is already at
    /// the first or last line, so the editor can transfer focus instead.
    pub(crate) fn move_cursor_vertically(
        &mut self,
        direction: i32,
        preferred_x: Pixels,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(paint) = self.last_paint() else {
            return false;
        };
        let lines = &paint.layout;
        let line_height = paint.line_height;

        let text = self.display_text();
        let ranges = element::hard_line_ranges(text);
        let (current_line_idx, offset_in_line) =
            element::line_index_for_offset(&ranges, self.cursor_offset());
        let Some(current_layout) = lines.get(current_line_idx) else {
            return false;
        };
        let Some(current_position) =
            element::position_for_offset(current_layout, offset_in_line, line_height, true)
        else {
            return false;
        };

        let current_y =
            element::wrapped_line_top(lines, line_height, current_line_idx) + current_position.y;
        let target_y = if direction < 0 {
            current_y - line_height + line_height / 2.0
        } else {
            current_y + line_height + line_height / 2.0
        };
        if target_y < px(0.0) {
            return false;
        }

        let total_height = lines.iter().fold(px(0.0), |height, line| {
            height + element::wrapped_line_height(line, line_height)
        });
        if target_y >= total_height {
            return false;
        }

        let Some((target_line_idx, target_y_in_line)) =
            element::wrapped_line_for_y(lines, line_height, target_y)
        else {
            return false;
        };
        let target_layout = &lines[target_line_idx];
        let target_point = point(preferred_x, target_y_in_line);
        let target_offset_in_line =
            match target_layout.closest_index_for_position(target_point, line_height) {
                Ok(idx) | Err(idx) => idx,
            };

        let flat_offset = ranges[target_line_idx].start + target_offset_in_line;
        self.move_to_with_preferred_x(flat_offset, Some(preferred_x), cx);
        true
    }

    /// Compute the character offset where the cursor should land when focus
    /// enters this block from above or below.  Uses the stored vertical
    /// motion anchor so cursor horizontal position is preserved across
    /// different-height blocks.
    pub fn entry_offset_for_vertical_focus(
        &self,
        prefer_last_line: bool,
        preferred_x: Option<Pixels>,
    ) -> usize {
        let Some(paint) = self.last_paint() else {
            return if prefer_last_line {
                self.display_len()
            } else {
                0
            };
        };
        let lines = &paint.layout;
        let line_height = paint.line_height;

        let text = self.display_text();
        let ranges = element::hard_line_ranges(text);
        let target_line_idx = if prefer_last_line { lines.len() - 1 } else { 0 };
        let target_layout = &lines[target_line_idx];
        let target_x = preferred_x.unwrap_or(px(0.0));
        let target_y = if prefer_last_line {
            element::wrapped_line_height(target_layout, line_height) - line_height / 2.0
        } else {
            line_height / 2.0
        };

        let offset_in_line = match target_layout
            .closest_index_for_position(point(target_x, target_y), line_height)
        {
            Ok(idx) | Err(idx) => idx,
        };
        ranges[target_line_idx].start + offset_in_line
    }

    pub fn move_to_with_preferred_x(
        &mut self,
        offset: usize,
        preferred_x: Option<Pixels>,
        cx: &mut Context<Self>,
    ) {
        self.assign_collapsed_selection_offset(
            offset,
            CollapsedCaretAffinity::Default,
            preferred_x,
        );
        self.cursor_blink_epoch = Instant::now();
        cx.notify();
    }

    /// Starts the cursor blink loop: a repeating background timer every 33ms
    /// that calls `cx.notify()` to repaint the cursor — but only while the
    /// cursor opacity is actually animating. During the first 0.5 s after
    /// each `cursor_blink_epoch` reset (which arrow keys / typing trigger),
    /// opacity is pinned to 1.0, so a repaint would just re-do the full
    /// projection rebuild for no visible change.
    ///
    /// The blink task is automatically cancelled when the block loses focus
    /// (the task handle is dropped in [`Block::render`]).
    pub(crate) fn start_cursor_blink(&mut self, cx: &mut Context<Self>) {
        self.cursor_blink_epoch = Instant::now();
        self.cursor_blink_task = Some(cx.spawn(
            async |this: WeakEntity<Block>, cx: &mut AsyncApp| loop {
                cx.background_executor()
                    .timer(Duration::from_millis(33))
                    .await;
                if this
                    .update(cx, |this: &mut Block, cx: &mut Context<Block>| {
                        if this.cursor_blink_epoch.elapsed().as_secs_f32() >= 0.5 {
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            },
        ));
    }

    /// Cosine-based smooth blink: fully opaque for 0.5s, then oscillates
    /// with a period of ~1s (33ms x 30 ticks ~= 1s).
    pub fn cursor_opacity(&self) -> f32 {
        let elapsed = self.cursor_blink_epoch.elapsed().as_secs_f32();
        if elapsed < 0.5 {
            return 1.0;
        }
        let t = elapsed - 0.5;
        (f32::cos(t * std::f32::consts::TAU) + 1.0) / 2.0
    }

    pub fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    pub(crate) fn end_pointer_selection_session(&mut self) -> bool {
        let changed = self.is_selecting || self.code_language_is_selecting;
        self.is_selecting = false;
        self.code_language_is_selecting = false;
        changed
    }

    pub(crate) fn selection_anchor_focus(&self) -> (usize, usize) {
        if self.selection_reversed {
            (self.selected_range.end, self.selected_range.start)
        } else {
            (self.selected_range.start, self.selected_range.end)
        }
    }

    pub(crate) fn plain_selection_anchor_focus(&self) -> (usize, usize) {
        let (anchor, focus) = self.selection_anchor_focus();
        (
            self.display_to_plain_offset(anchor),
            self.display_to_plain_offset(focus),
        )
    }

    pub(crate) fn set_selection_from_anchor_focus(&mut self, anchor: usize, focus: usize) {
        let clamped_anchor = anchor.min(self.display_len());
        let clamped_focus = focus.min(self.display_len());
        self.selected_range = clamped_anchor.min(clamped_focus)..clamped_anchor.max(clamped_focus);
        self.selection_reversed = !self.selected_range.is_empty() && clamped_focus < clamped_anchor;
    }

    pub(crate) fn set_selection_from_plain_anchor_focus(
        &mut self,
        anchor: usize,
        focus: usize,
        anchor_affinity: CollapsedCaretAffinity,
        focus_affinity: CollapsedCaretAffinity,
    ) {
        // Map each endpoint back through its own affinity. Several display
        // positions can share one plain offset (a trailing link's `](url)`
        // delimiters all collapse onto the anchor-text end), so the plain
        // plain->display cursor map would snap an endpoint that sat after the
        // closing delimiter back to just inside it. Honoring the captured
        // affinity keeps such endpoints in place across a projection rebuild.
        self.set_selection_from_anchor_focus(
            self.plain_to_display_cursor_offset_with_affinity(anchor, anchor_affinity),
            self.plain_to_display_cursor_offset_with_affinity(focus, focus_affinity),
        );
    }

    pub fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.move_to_with_preferred_x(offset, None, cx);
    }

    pub fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let clamped_offset = offset.min(self.display_len());
        if self.selection_reversed {
            self.selected_range.start = clamped_offset;
        } else {
            self.selected_range.end = clamped_offset;
        }
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        self.cursor_blink_epoch = Instant::now();
        self.clear_vertical_motion();
        self.sync_collapsed_caret_affinity();
        cx.notify();
    }

    pub(crate) fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        Self::utf8_range_to_utf16_in(self.display_text(), range)
    }

    pub(crate) fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        Self::utf16_range_to_utf8_in(self.display_text(), range_utf16)
    }

    pub fn previous_boundary(&self, offset: usize) -> usize {
        let text = self.display_text();
        let mut cursor = GraphemeCursor::new(offset.min(text.len()), text.len(), true);
        cursor.prev_boundary(text, 0).ok().flatten().unwrap_or(0)
    }

    pub fn next_boundary(&self, offset: usize) -> usize {
        let text = self.display_text();
        let mut cursor = GraphemeCursor::new(offset.min(text.len()), text.len(), true);
        cursor
            .next_boundary(text, 0)
            .ok()
            .flatten()
            .unwrap_or(text.len())
    }

    /// Offset of the start of the word before `offset`, or 0 if there is none.
    pub fn previous_word_start(&self, offset: usize) -> usize {
        let text = self.display_text();
        let offset = offset.min(text.len());
        text.unicode_word_indices()
            .map(|(start, _)| start)
            .take_while(|start| *start < offset)
            .last()
            .unwrap_or(0)
    }

    /// Offset of the start of the word after `offset`, or the text length if
    /// there is none.
    pub fn next_word_start(&self, offset: usize) -> usize {
        let text = self.display_text();
        let offset = offset.min(text.len());
        text.unicode_word_indices()
            .map(|(start, _)| start)
            .find(|start| *start > offset)
            .unwrap_or(text.len())
    }

    /// Reverse of `display_offset`: maps an expanded display offset
    /// back to the plain tree offset.
    pub(crate) fn unexpand_offset(&self, expanded: usize) -> usize {
        let Some(projection) = &self.projection else {
            return expanded;
        };
        projection
            .display_to_plain
            .get(expanded.min(projection.display_to_plain.len().saturating_sub(1)))
            .copied()
            .unwrap_or(expanded)
    }

    pub fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.display_text().is_empty() {
            return 0;
        }

        // The pointer selects the pane it is inside: with multiple Wysiwyg
        // panes the same block paints once per pane with different bounds.
        let Some(paint) = self.last_paint_at(position) else {
            return 0;
        };
        let bounds = paint.bounds;
        let lines = &paint.layout;
        let line_height = paint.line_height;

        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.display_len();
        }

        let text = self.display_text();
        let ranges = element::hard_line_ranges(text);
        let relative_y = position.y - bounds.top();
        let Some((line_idx, y_in_line)) =
            element::wrapped_line_for_y(lines, line_height, relative_y)
        else {
            return 0;
        };
        let layout = &lines[line_idx];
        let origin_x = element::aligned_line_left(layout, bounds, self.text_align());

        let offset_in_line = match layout
            .closest_index_for_position(point(position.x - origin_x, y_in_line), line_height)
        {
            Ok(idx) | Err(idx) => idx,
        };
        // The layout was built from the text at the last paint; if the text
        // has since gained or lost hard line breaks (e.g. reference text was
        // replaced), clamp to the last known hard line instead of panicking.
        let hard_line_idx = line_idx.min(ranges.len().saturating_sub(1));
        ranges[hard_line_idx].start + offset_in_line
    }

    pub(crate) fn active_range_or_cursor_bounds(&self) -> Option<Bounds<Pixels>> {
        let paint = self.last_paint()?;
        let bounds = paint.bounds;
        let lines = &paint.layout;
        let line_height = paint.line_height;
        let text = self.display_text();
        let active_range = self
            .marked_range
            .clone()
            .unwrap_or_else(|| self.selected_range.clone());

        if active_range.is_empty() {
            return element::cursor_bounds_for_offset(
                lines,
                bounds,
                line_height,
                text,
                self.cursor_offset(),
                self.text_align(),
                px(1.0),
            );
        }

        element::range_bounds(
            lines,
            bounds,
            line_height,
            text,
            active_range,
            self.text_align(),
        )
    }
}

// ---------------------------------------------------------------------------
// Code language editing runtime
// ---------------------------------------------------------------------------

pub(crate) fn normalize_code_language_input(text: &str) -> String {
    text.replace("\r\n", " ")
        .replace(['\r', '\n'], " ")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {}
