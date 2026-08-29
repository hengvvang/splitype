//! Inline projection lifecycle, build/clear, and caret affinity.

use std::ops::Range;

use crate::projection::{
    ExpandedInlineProjection, ExpandedInlineSegment, ExpandedLinkSpan,
    ProjectedLinkSelectionSnapshot,
};
use crate::document::block::{Block, CollapsedCaretAffinity};
use markdown::inline::render_cache::InlineRenderCache;
use markdown::parse::BlockKind;
use gpui::Pixels;

impl Block {
    pub fn display_cache(&self) -> &InlineRenderCache {
        self.projection
            .as_ref()
            .map(|projection| &projection.cache)
            .unwrap_or(&self.render_cache)
    }

    pub fn sync_inline_projection_for_focus(&mut self, focused: bool) {
        let supports_projection = self.edit_mode.supports_inline_projection();
        if !focused || !supports_projection {
            self.clear_inline_projection();
            return;
        }

        let projected_prefix_selection = if self.projection.is_some() {
            self.projection.as_ref().and_then(|projection| {
                projection.block_prefix_range.as_ref().and_then(|prefix_range| {
                    if self.selected_range.start <= prefix_range.end
                        && self.selected_range.end <= prefix_range.end
                    {
                        Some((
                            self.selected_range.clone(),
                            self.selection_reversed,
                        ))
                    } else {
                        None
                    }
                })
            })
        } else if matches!(self.kind(), BlockKind::Callout(_))
            && self.data.text.plain_text().is_empty()
        {
            Some((self.selected_range.clone(), self.selection_reversed))
        } else {
            None
        };

        let projected_link_selection = self.projection.as_ref().and_then(|projection| {
            projection
                .link_span_fully_covering_range(&self.selected_range)
                .map(|span| ProjectedLinkSelectionSnapshot {
                    plain_range: span.plain_range.clone(),
                    display_relative_range: self
                        .selected_range
                        .start
                        .saturating_sub(span.display_range.start)
                        ..self
                            .selected_range
                            .end
                            .saturating_sub(span.display_range.start),
                    selection_reversed: self.selection_reversed,
                })
        });
        let plain_selected = self.display_to_plain_range(self.selected_range.clone());
        let plain_marked = self
            .marked_range
            .clone()
            .map(|range| self.display_to_plain_range(range));
        let kind_key = match self.kind() {
            BlockKind::Heading { level } => Some(level),
            BlockKind::Callout(variant) => Some(10 + variant as u8),
            _ => None,
        };
        if let Some((cached_supports, cached_kind, cached_selected, cached_marked)) =
            &self.projection_cache_key
            && *cached_supports == supports_projection
            && *cached_kind == kind_key
            && *cached_selected == plain_selected
            && *cached_marked == plain_marked
        {
            return;
        }
        let (plain_anchor, plain_focus) = self.plain_selection_anchor_focus();
        let (anchor_affinity, focus_affinity) = self.selection_endpoint_affinities();
        let collapsed_affinity = self.display_collapsed_caret_affinity();
        self.rebuild_inline_projection(plain_selected.clone(), plain_marked.clone());
        if let Some((prefix_range, reversed)) = projected_prefix_selection
            && let Some(new_prefix_range) = self
                .projection
                .as_ref()
                .and_then(|projection| projection.block_prefix_range.clone())
        {
            let start = prefix_range.start.min(new_prefix_range.end);
            let end = prefix_range.end.min(new_prefix_range.end);
            self.selected_range = start..end;
            self.selection_reversed = reversed;
            self.collapsed_caret_affinity = CollapsedCaretAffinity::Default;
        } else if let Some(snapshot) = projected_link_selection
            && let Some(span) = self
                .projection
                .as_ref()
                .and_then(|projection| projection.link_span_for_plain_range(&snapshot.plain_range))
        {
            let start = span.display_range.start
                + snapshot
                    .display_relative_range
                    .start
                    .min(span.display_range.len());
            let end = span.display_range.start
                + snapshot
                    .display_relative_range
                    .end
                    .min(span.display_range.len());
            self.selected_range = start..end;
            self.selection_reversed = snapshot.selection_reversed;
            self.collapsed_caret_affinity = CollapsedCaretAffinity::Default;
        } else if plain_selected.is_empty() {
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
    }

    pub fn clear_inline_projection(&mut self) {
        if self.projection.is_none() {
            self.projection_cache_key = None;
            return;
        }

        let plain_marked = self
            .marked_range
            .clone()
            .map(|range| self.display_to_plain_range(range));
        let (plain_anchor, plain_focus) = self.plain_selection_anchor_focus();
        self.projection = None;
        self.projection_cache_key = None;
        self.set_selection_from_anchor_focus(plain_anchor, plain_focus);
        self.marked_range = plain_marked;
        self.collapsed_caret_affinity = CollapsedCaretAffinity::Default;
        self.refresh_cached_display_text();
    }

    pub fn rebuild_inline_projection(
        &mut self,
        plain_selected: Range<usize>,
        plain_marked: Option<Range<usize>>,
    ) {
        let kind_key = match self.kind() {
            BlockKind::Heading { level } => Some(level),
            BlockKind::Callout(variant) => Some(10 + variant as u8),
            _ => None,
        };
        self.projection_cache_key = Some((
            self.edit_mode.supports_inline_projection(),
            kind_key,
            plain_selected.clone(),
            plain_marked.clone(),
        ));
        let (block_prefix, footnote_head_len) = match self.kind() {
            BlockKind::Heading { level } => {
                (Some(format!("{} ", "#".repeat(level as usize))), None)
            }
            BlockKind::FootnoteDefinition => {
                let head_len = self
                    .data
                    .text
                    .plain_text()
                    .find(':')
                    .unwrap_or_else(|| self.data.text.plain_len());
                (Some("[^".to_string()), Some(head_len))
            }
            BlockKind::Callout(variant) => {
                let prefix = if self.data.text.plain_text().is_empty() {
                    format!("[!{}]", variant.marker_lower())
                } else {
                    format!("[!{}] ", variant.marker_lower())
                };
                (Some(prefix), None)
            }
            _ => (None, None),
        };
        self.projection = ExpandedInlineProjection::build_with_prefix(
            &self.data.text.fragments,
            plain_selected,
            plain_marked,
            block_prefix.as_deref(),
            footnote_head_len,
        );
        self.refresh_cached_display_text();
    }

    pub fn projection_segments(&self) -> &[ExpandedInlineSegment] {
        self.projection
            .as_ref()
            .map(|projection| projection.segments.as_slice())
            .unwrap_or(&[])
    }

    /// Display ranges of the projected delimiter markers, used to color the
    /// revealed source syntax while editing. Empty when no projection is active.
    pub fn projected_delimiter_ranges(&self) -> Vec<std::ops::Range<usize>> {
        self.projection
            .as_ref()
            .map(|projection| projection.delimiter_ranges())
            .unwrap_or_default()
    }

    pub fn projected_link_span_fully_covering_range(
        &self,
        range: &Range<usize>,
    ) -> Option<&ExpandedLinkSpan> {
        self.projection
            .as_ref()
            .and_then(|projection| projection.link_span_fully_covering_range(range))
    }

    pub fn collapsed_caret_affinity_for_display_offset(
        &self,
        offset: usize,
    ) -> CollapsedCaretAffinity {
        self.projection
            .as_ref()
            .map(|projection| projection.collapsed_affinity_for_display_offset(offset))
            .unwrap_or(CollapsedCaretAffinity::Default)
    }

    /// Affinity of the current selection's anchor and focus, used to restore
    /// each endpoint accurately when the projection is rebuilt.
    pub fn selection_endpoint_affinities(
        &self,
    ) -> (CollapsedCaretAffinity, CollapsedCaretAffinity) {
        let (anchor, focus) = self.selection_anchor_focus();
        (
            self.collapsed_caret_affinity_for_display_offset(anchor),
            self.collapsed_caret_affinity_for_display_offset(focus),
        )
    }

    pub fn display_collapsed_caret_affinity(&self) -> CollapsedCaretAffinity {
        if !self.selected_range.is_empty() {
            return CollapsedCaretAffinity::Default;
        }

        self.projection
            .as_ref()
            .map(|projection| {
                projection.collapsed_affinity_for_display_offset(self.cursor_offset())
            })
            .unwrap_or(self.collapsed_caret_affinity)
    }

    pub fn sync_collapsed_caret_affinity(&mut self) {
        self.collapsed_caret_affinity = if self.selected_range.is_empty() {
            self.projection
                .as_ref()
                .map(|projection| {
                    projection.collapsed_affinity_for_display_offset(self.cursor_offset())
                })
                .unwrap_or(CollapsedCaretAffinity::Default)
        } else {
            CollapsedCaretAffinity::Default
        };
    }

    pub fn assign_collapsed_selection_offset(
        &mut self,
        offset: usize,
        affinity: CollapsedCaretAffinity,
        preferred_x: Option<Pixels>,
    ) {
        let clamped_offset = offset.min(self.display_len());
        self.selected_range = clamped_offset..clamped_offset;
        self.selection_reversed = false;
        self.vertical_motion_x = preferred_x;
        self.collapsed_caret_affinity = affinity;
        self.sync_collapsed_caret_affinity();
    }
}
