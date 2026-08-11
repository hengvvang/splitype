//! Inline projection engine — editable Markdown delimiters on a block.
//!
//! This module manages the block-side projection state: the build /
//! clear / rebuild entry points that keep a block's projection in sync
//! with its source. The pure projection algorithm engine itself lives in
//! `crate::editor::editing::projection`.

use std::ops::Range;
use std::time::Instant;

use gpui::*;

use super::block::CollapsedCaretAffinity;
use crate::editor::block_protocol::{BlockAction, UndoCaptureKind};
use crate::editor::editing::projection::{
    ExpandedInlineProjection, ExpandedInlineSegment, ExpandedInlineSegmentKind, ExpandedLinkRun,
    ProjectedLinkSelectionSnapshot,
};
use crate::editor::tree::block::Block;
use crate::model::block::BlockKind;
use crate::model::inline::render_cache::InlineRenderCache;
use crate::model::inline::style::StyleFlag;
use crate::model::inline::text::{InlineFragment, InlineInsertionAttributes, RichText};

impl Block {
    pub(crate) fn display_cache(&self) -> &InlineRenderCache {
        self.projection
            .as_ref()
            .map(|projection| &projection.cache)
            .unwrap_or(&self.render_cache)
    }

    pub(crate) fn sync_inline_projection_for_focus(&mut self, focused: bool) {
        let supports_projection = self.edit_mode.supports_inline_projection();
        if !focused || !supports_projection {
            self.clear_inline_projection();
            return;
        }

        let projected_link_selection = self.projection.as_ref().and_then(|projection| {
            projection
                .link_run_fully_covering_range(&self.selected_range)
                .map(|run| ProjectedLinkSelectionSnapshot {
                    plain_range: run.plain_range.clone(),
                    display_relative_range: self
                        .selected_range
                        .start
                        .saturating_sub(run.display_range.start)
                        ..self
                            .selected_range
                            .end
                            .saturating_sub(run.display_range.start),
                    selection_reversed: self.selection_reversed,
                })
        });
        let plain_selected = self.display_to_plain_range(self.selected_range.clone());
        let plain_marked = self
            .marked_range
            .clone()
            .map(|range| self.display_to_plain_range(range));
        let heading_level = match self.kind() {
            BlockKind::Heading { level } => Some(level),
            _ => None,
        };
        if self.projection_cache_key.as_ref()
            == Some(&(
                supports_projection,
                heading_level,
                plain_selected.clone(),
                plain_marked.clone(),
            ))
        {
            return;
        }
        let (plain_anchor, plain_focus) = self.plain_selection_anchor_focus();
        let (anchor_affinity, focus_affinity) = self.selection_endpoint_affinities();
        let collapsed_affinity = self.display_collapsed_caret_affinity();
        self.rebuild_inline_projection(plain_selected.clone(), plain_marked.clone());
        if let Some(snapshot) = projected_link_selection
            && let Some(run) = self
                .projection
                .as_ref()
                .and_then(|projection| projection.link_run_for_plain_range(&snapshot.plain_range))
        {
            let start = run.display_range.start
                + snapshot
                    .display_relative_range
                    .start
                    .min(run.display_range.len());
            let end = run.display_range.start
                + snapshot
                    .display_relative_range
                    .end
                    .min(run.display_range.len());
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

    pub(crate) fn clear_inline_projection(&mut self) {
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

    pub(crate) fn rebuild_inline_projection(
        &mut self,
        plain_selected: Range<usize>,
        plain_marked: Option<Range<usize>>,
    ) {
        let heading_level = match self.kind() {
            BlockKind::Heading { level } => Some(level),
            _ => None,
        };
        self.projection_cache_key = Some((
            self.edit_mode.supports_inline_projection(),
            heading_level,
            plain_selected.clone(),
            plain_marked.clone(),
        ));
        let block_prefix = heading_level.map(|level| format!("{} ", "#".repeat(level as usize)));
        self.projection = ExpandedInlineProjection::build_with_prefix(
            &self.record.text.fragments,
            plain_selected,
            plain_marked,
            block_prefix.as_deref(),
        );
        self.refresh_cached_display_text();
    }

    pub(crate) fn projection_segments(&self) -> &[ExpandedInlineSegment] {
        self.projection
            .as_ref()
            .map(|projection| projection.segments.as_slice())
            .unwrap_or(&[])
    }

    pub(crate) fn projected_link_run_fully_covering_range(
        &self,
        range: &Range<usize>,
    ) -> Option<&ExpandedLinkRun> {
        self.projection
            .as_ref()
            .and_then(|projection| projection.link_run_fully_covering_range(range))
    }

    pub(crate) fn collapsed_caret_affinity_for_display_offset(
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
    pub(crate) fn selection_endpoint_affinities(
        &self,
    ) -> (CollapsedCaretAffinity, CollapsedCaretAffinity) {
        let (anchor, focus) = self.selection_anchor_focus();
        (
            self.collapsed_caret_affinity_for_display_offset(anchor),
            self.collapsed_caret_affinity_for_display_offset(focus),
        )
    }

    pub(crate) fn display_collapsed_caret_affinity(&self) -> CollapsedCaretAffinity {
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

    pub(crate) fn sync_collapsed_caret_affinity(&mut self) {
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

    pub(crate) fn assign_collapsed_selection_offset(
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

    pub(crate) fn plain_to_display_cursor_offset(&self, plain: usize) -> usize {
        let Some(projection) = &self.projection else {
            return plain;
        };
        projection
            .plain_to_display_cursor
            .get(plain.min(projection.plain_to_display_cursor.len().saturating_sub(1)))
            .copied()
            .unwrap_or(plain)
    }

    pub(crate) fn plain_to_display_cursor_offset_with_affinity(
        &self,
        plain: usize,
        affinity: CollapsedCaretAffinity,
    ) -> usize {
        let Some(projection) = &self.projection else {
            return plain;
        };
        projection
            .display_offset_for_plain_cursor(plain, affinity)
            .unwrap_or_else(|| self.plain_to_display_cursor_offset(plain))
    }

    pub(crate) fn plain_to_display_range_start(&self, plain: usize) -> usize {
        self.plain_to_display_cursor_offset(plain)
    }

    pub(crate) fn plain_to_display_range_end(&self, plain: usize) -> usize {
        self.plain_to_display_cursor_offset(plain)
    }

    pub(crate) fn plain_to_display_range(&self, range: Range<usize>) -> Range<usize> {
        if range.is_empty() {
            let offset = self.plain_to_display_cursor_offset(range.start);
            offset..offset
        } else {
            self.plain_to_display_range_start(range.start)
                ..self.plain_to_display_range_end(range.end)
        }
    }

    pub(crate) fn display_to_plain_range(&self, range: Range<usize>) -> Range<usize> {
        self.display_to_plain_offset(range.start)..self.display_to_plain_offset(range.end)
    }

    pub(crate) fn display_to_plain_offset(&self, offset: usize) -> usize {
        self.unexpand_offset(offset)
    }

    pub(crate) fn projected_move_left_target(
        &self,
        offset: usize,
    ) -> Option<(usize, CollapsedCaretAffinity)> {
        self.projection
            .as_ref()
            .and_then(|projection| projection.move_left_target(offset))
    }

    pub(crate) fn projected_move_right_target(
        &self,
        offset: usize,
    ) -> Option<(usize, CollapsedCaretAffinity)> {
        self.projection
            .as_ref()
            .and_then(|projection| projection.move_right_target(offset))
    }

    pub(crate) fn selection_plain_range(&self) -> Range<usize> {
        self.display_to_plain_range(self.selected_range.clone())
    }

    pub(crate) fn display_range_to_source_range(&self, range: Range<usize>) -> Range<usize> {
        if self.uses_raw_text_editing() || self.kind().is_code_block() {
            return range.start.min(self.display_len())..range.end.min(self.display_len());
        }

        if let Some(link_run) = self.projected_link_run_fully_covering_range(&range) {
            let map = self.record.text.source_offset_map();
            let label_source_start = map.plain_to_source_offset(link_run.plain_range.start);
            let run_source_start =
                label_source_start.saturating_sub(link_run.link.open_marker().len());
            let start = run_source_start
                + range
                    .start
                    .saturating_sub(link_run.display_range.start)
                    .min(link_run.display_range.len());
            let end = run_source_start
                + range
                    .end
                    .saturating_sub(link_run.display_range.start)
                    .min(link_run.display_range.len());
            return start..end;
        }

        if let Some(footnote_run) = self
            .projection
            .as_ref()
            .and_then(|projection| projection.footnote_run_fully_covering_range(&range))
        {
            let raw = footnote_run.footnote.raw_markdown();
            let raw_len = raw.len();
            let local_start = range
                .start
                .saturating_sub(footnote_run.display_range.start)
                .min(footnote_run.display_range.len());
            let local_end = range
                .end
                .saturating_sub(footnote_run.display_range.start)
                .min(footnote_run.display_range.len());
            let mapped_start = (raw_len * local_start) / footnote_run.display_range.len().max(1);
            let mapped_end = (raw_len * local_end) / footnote_run.display_range.len().max(1);
            let map = self.record.text.source_offset_map();
            let run_source_start = map.plain_to_source_offset(footnote_run.plain_range.start);
            return run_source_start + mapped_start..run_source_start + mapped_end;
        }

        let plain_range = self.display_to_plain_range(range);
        self.record
            .text
            .source_offset_map()
            .plain_to_source_range(plain_range)
    }

    pub(crate) fn source_range_to_display_range(&self, range: Range<usize>) -> Range<usize> {
        if self.uses_raw_text_editing() || self.kind().is_code_block() {
            let len = self.display_len();
            return range.start.min(len)..range.end.min(len);
        }

        let plain_range = self
            .record
            .text
            .source_offset_map()
            .source_to_plain_range(range);
        self.plain_to_display_range(plain_range)
    }

    pub(crate) fn source_offset_to_display_offset(&self, offset: usize) -> usize {
        self.source_range_to_display_range(offset..offset).start
    }

    pub(crate) fn prepare_undo_capture(&self, kind: UndoCaptureKind, cx: &mut Context<Self>) {
        cx.emit(BlockAction::PrepareUndo { kind });
    }

    pub(crate) fn utf16_to_utf8_in(text: &str, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;

        for ch in text.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }

        utf8_offset
    }

    pub(crate) fn utf8_to_utf16_in(text: &str, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;

        for ch in text.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }

        utf16_offset
    }

    pub(crate) fn utf16_range_to_utf8_in(text: &str, range_utf16: &Range<usize>) -> Range<usize> {
        Self::utf16_to_utf8_in(text, range_utf16.start)
            ..Self::utf16_to_utf8_in(text, range_utf16.end)
    }

    pub(crate) fn utf8_range_to_utf16_in(text: &str, range: &Range<usize>) -> Range<usize> {
        Self::utf8_to_utf16_in(text, range.start)..Self::utf8_to_utf16_in(text, range.end)
    }

    /// Detect Markdown shortcut prefixes in the edited text and convert the
    /// block's kind accordingly (e.g. `"- " -> BulletedListItem`).
    ///
    /// Only triggers when the current kind is [`BlockKind::Paragraph`].
    /// Returns the potentially updated kind, the text with prefix stripped,
    /// the new cursor offset, and the number of prefix characters removed.
    pub(crate) fn normalize_after_text_edit(
        &self,
        mut next_text: RichText,
        cursor: usize,
    ) -> (BlockKind, RichText, usize, usize) {
        if self.is_table_cell() {
            return (self.kind(), next_text, cursor, 0);
        }

        if !self.uses_raw_text_editing() && self.kind() == BlockKind::Paragraph {
            let plain_text = next_text.plain_text();
            if let Some((kind, prefix_len)) = BlockKind::detect_markdown_shortcut(&plain_text) {
                next_text.remove_visible_prefix(prefix_len);
                return (
                    kind,
                    next_text,
                    cursor.saturating_sub(prefix_len),
                    prefix_len,
                );
            }
        }

        if !self.uses_raw_text_editing() && self.kind() == BlockKind::BulletListItem {
            let plain_text = next_text.plain_text();
            if let Some((checked, prefix_len)) =
                BlockKind::parse_task_list_item_prefix(&plain_text)
            {
                next_text.remove_visible_prefix(prefix_len);
                return (
                    BlockKind::TaskListItem { checked },
                    next_text,
                    cursor.saturating_sub(prefix_len),
                    prefix_len,
                );
            }
        }

        (self.kind(), next_text, cursor, 0)
    }

    pub(crate) fn quote_line_starts_block_syntax(line: &str) -> bool {
        let trimmed_end = line.trim_end();
        if trimmed_end.is_empty() {
            return false;
        }

        let leading_spaces = trimmed_end.bytes().take_while(|b| *b == b' ').count();
        if leading_spaces >= 4 {
            return true;
        }

        BlockKind::detect_markdown_shortcut(&format!("{trimmed_end} "))
            .is_some_and(|(kind, _)| kind != BlockKind::Paragraph)
            || BlockKind::parse_code_fence_opening(trimmed_end).is_some()
            || BlockKind::parse_thematic_break_line(trimmed_end)
            || BlockKind::parse_atx_heading_line(trimmed_end).is_some()
    }

    pub(crate) fn multiline_quote_edit_requires_reparse(text: &str) -> bool {
        text.split('\n')
            .skip(1)
            .any(Self::quote_line_starts_block_syntax)
    }

    pub(crate) fn adjust_range_for_shortcut(
        range: &Range<usize>,
        removed_prefix_len: usize,
    ) -> Range<usize> {
        range.start.saturating_sub(removed_prefix_len)..range.end.saturating_sub(removed_prefix_len)
    }

    pub(crate) fn projected_styles_touching_display_range(
        &self,
        display_range: &Range<usize>,
    ) -> Vec<(usize, StyleFlag)> {
        let mut targets = Vec::new();
        for segment in self.projection_segments() {
            let touches = display_range.start < segment.display_range.end
                && segment.display_range.start < display_range.end;
            if touches
                && matches!(
                    segment.kind,
                    ExpandedInlineSegmentKind::OpeningDelimiter(_)
                        | ExpandedInlineSegmentKind::ClosingDelimiter(_)
                )
            {
                let kind = match segment.kind {
                    ExpandedInlineSegmentKind::OpeningDelimiter(kind)
                    | ExpandedInlineSegmentKind::ClosingDelimiter(kind) => kind,
                    _ => continue,
                };
                if let Some(flag) = kind.style_flag() {
                    let target = (segment.fragment_index, flag);
                    if !targets.contains(&target) {
                        targets.push(target);
                    }
                }
            }
        }
        targets
    }

    pub(crate) fn clean_offset_before_fragment_index(
        fragments: &[InlineFragment],
        index: usize,
    ) -> usize {
        fragments
            .iter()
            .take(index)
            .map(|fragment| fragment.text.len())
            .sum()
    }

    pub(crate) fn replacement_is_pure_link_run(fragments: &[InlineFragment]) -> bool {
        let Some(first_link) = fragments
            .first()
            .and_then(|fragment| fragment.link.as_ref())
        else {
            return false;
        };

        fragments
            .iter()
            .all(|fragment| fragment.link.as_ref() == Some(first_link))
    }

    pub(crate) fn apply_link_projection_edit(
        &mut self,
        link_run: &ExpandedLinkRun,
        display_range: Range<usize>,
        new_text: &str,
        selected_range_relative: Option<Range<usize>>,
        mark_inserted_text: bool,
        cx: &mut Context<Self>,
    ) {
        let local_visible_range = display_range.start - link_run.display_range.start
            ..display_range.end - link_run.display_range.start;
        let local_display_text = self.display_text()[link_run.display_range.clone()].to_string();
        let local_tree = RichText::plain(local_display_text);
        let local_result = local_tree.replace_visible_range_with_link_references(
            local_visible_range.clone(),
            new_text,
            InlineInsertionAttributes::default(),
            &self.link_reference_definitions,
        );
        let replacement_fragments = local_result.tree.fragments.clone();

        let replacement_start = link_run.start_fragment_index;
        let replacement_clean_start = Self::clean_offset_before_fragment_index(
            &self.record.text.fragments,
            replacement_start,
        );
        let mut next_text = self.record.text.clone();
        next_text.replace_fragment_range(
            link_run.start_fragment_index..link_run.end_fragment_index,
            replacement_fragments.clone(),
        );

        if Self::replacement_is_pure_link_run(&replacement_fragments) {
            let old_kind = self.record.kind.clone();
            let old_text = self.record.text.clone();
            self.record.set_text(next_text.clone());
            self.sync_edit_mode_from_kind();
            self.sync_render_cache();

            let replacement_plain_len = replacement_fragments
                .iter()
                .map(|fragment| fragment.text.len())
                .sum::<usize>();
            let selected_plain =
                replacement_clean_start..replacement_clean_start + replacement_plain_len;
            self.rebuild_inline_projection(selected_plain.clone(), None);

            let local_selected = selected_range_relative.clone().unwrap_or_else(|| {
                let cursor = local_visible_range.start + new_text.len();
                cursor..cursor
            });
            if let Some(projected_link_run) = self.projection.as_ref().and_then(|projection| {
                projection
                    .link_runs
                    .iter()
                    .find(|run| run.plain_range == selected_plain)
            }) {
                let start = projected_link_run.display_range.start
                    + local_selected
                        .start
                        .min(projected_link_run.display_range.len());
                let end = projected_link_run.display_range.start
                    + local_selected
                        .end
                        .min(projected_link_run.display_range.len());
                self.selected_range = start..end;
                self.selection_reversed = false;
                self.marked_range = if mark_inserted_text && !new_text.is_empty() {
                    Some(start..end)
                } else {
                    None
                };
                self.collapsed_caret_affinity = CollapsedCaretAffinity::Default;
                self.cursor_blink_epoch = Instant::now();
                self.clear_vertical_motion();
                if self.record.kind != old_kind || self.record.text != old_text {
                    cx.emit(BlockAction::Changed);
                }
                cx.notify();
                return;
            }
        }

        let local_selected = selected_range_relative.as_ref().map(|relative| {
            let absolute = local_visible_range.start + relative.start
                ..local_visible_range.start + relative.end;
            local_result.map_range(&absolute)
        });
        let cursor = local_selected
            .as_ref()
            .map(|range| range.end)
            .unwrap_or_else(|| local_result.map_offset(local_visible_range.start + new_text.len()));
        let prefix = replacement_clean_start;
        let selected_plain = local_selected.map(|range| prefix + range.start..prefix + range.end);
        let marked_plain = if mark_inserted_text && !new_text.is_empty() {
            let inserted_range =
                local_visible_range.start..local_visible_range.start + new_text.len();
            let mapped = local_result.map_range(&inserted_range);
            Some(prefix + mapped.start..prefix + mapped.end)
        } else {
            None
        };
        self.apply_text_edit(
            next_text,
            prefix + cursor,
            marked_plain,
            selected_plain.clone(),
            selected_plain
                .as_ref()
                .and_then(|range| (!range.is_empty()).then_some(false)),
            false,
            cx,
        );
    }

    pub(crate) fn insertion_attributes_for_display_offset(
        &self,
        display_offset: usize,
    ) -> InlineInsertionAttributes {
        if self.uses_raw_text_editing() {
            return InlineInsertionAttributes::default();
        }

        if self.projection.is_none() {
            return self.record.text.attributes_for_insertion_at(display_offset);
        }

        for segment in self.projection_segments() {
            match segment.kind {
                ExpandedInlineSegmentKind::StyledText
                    if display_offset >= segment.display_range.start
                        && display_offset <= segment.display_range.end =>
                {
                    let fragment = &self.record.text.fragments[segment.fragment_index];
                    return InlineInsertionAttributes {
                        style: fragment.style,
                        html_style: fragment.html_style,
                        link: fragment.link.clone(),
                        footnote: fragment.footnote.clone(),
                        math: None,
                    };
                }
                ExpandedInlineSegmentKind::OpeningDelimiter(_)
                    if display_offset == segment.display_range.end =>
                {
                    let fragment = &self.record.text.fragments[segment.fragment_index];
                    return InlineInsertionAttributes {
                        style: fragment.style,
                        html_style: fragment.html_style,
                        link: fragment.link.clone(),
                        footnote: fragment.footnote.clone(),
                        math: None,
                    };
                }
                ExpandedInlineSegmentKind::ClosingDelimiter(_)
                    if display_offset == segment.display_range.start =>
                {
                    let fragment = &self.record.text.fragments[segment.fragment_index];
                    return InlineInsertionAttributes {
                        style: fragment.style,
                        html_style: fragment.html_style,
                        link: fragment.link.clone(),
                        footnote: fragment.footnote.clone(),
                        math: None,
                    };
                }
                // Caret just outside a span: after a closing delimiter or before
                // an opening one. Insert plain text so it isn't absorbed back into
                // the span, matching how code and strikethrough already behave.
                ExpandedInlineSegmentKind::ClosingDelimiter(_)
                    if display_offset == segment.display_range.end =>
                {
                    return InlineInsertionAttributes::default();
                }
                ExpandedInlineSegmentKind::OpeningDelimiter(_)
                    if display_offset == segment.display_range.start =>
                {
                    return InlineInsertionAttributes::default();
                }
                ExpandedInlineSegmentKind::LinkTargetText => {
                    if let Some(link_group) = segment.link_group
                        && let Some(link_run) = self
                            .projection
                            .as_ref()
                            .and_then(|projection| projection.link_runs.get(link_group))
                        && display_offset >= link_run.target_display_range.start
                        && display_offset <= link_run.target_display_range.end
                    {
                        return InlineInsertionAttributes::default();
                    }
                }
                _ => {}
            }
        }

        self.record
            .text
            .attributes_for_insertion_at(self.display_to_plain_offset(display_offset))
    }

    pub(crate) fn attributes_for_fragment(fragment: &InlineFragment) -> InlineInsertionAttributes {
        InlineInsertionAttributes {
            style: fragment.style,
            html_style: fragment.html_style,
            link: fragment.link.clone(),
            footnote: fragment.footnote.clone(),
            math: None,
        }
    }

    pub(crate) fn replacement_attributes_for_display_range(
        &self,
        display_range: &Range<usize>,
    ) -> InlineInsertionAttributes {
        if self.uses_raw_text_editing() {
            return InlineInsertionAttributes::default();
        }

        if display_range.is_empty() {
            return self.insertion_attributes_for_display_offset(display_range.start);
        }

        if self.projection.is_some() {
            return self
                .projected_replacement_attributes_for_visible_range(display_range)
                .unwrap_or_default();
        }

        self.fragment_attributes_for_plain_range(self.display_to_plain_range(display_range.clone()))
            .unwrap_or_default()
    }

    pub(crate) fn projected_replacement_attributes_for_visible_range(
        &self,
        display_range: &Range<usize>,
    ) -> Option<InlineInsertionAttributes> {
        self.projection_segments().iter().find_map(|segment| {
            (segment.kind == ExpandedInlineSegmentKind::StyledText
                && segment.display_range.start <= display_range.start
                && display_range.end <= segment.display_range.end)
                .then(|| {
                    self.record
                        .text
                        .fragments
                        .get(segment.fragment_index)
                        .map(Self::attributes_for_fragment)
                })
                .flatten()
        })
    }

    pub(crate) fn fragment_attributes_for_plain_range(
        &self,
        plain_range: Range<usize>,
    ) -> Option<InlineInsertionAttributes> {
        if plain_range.is_empty() {
            return None;
        }

        let mut cursor = 0usize;
        for fragment in &self.record.text.fragments {
            let fragment_start = cursor;
            let fragment_end = fragment_start + fragment.text.len();
            if fragment_start <= plain_range.start && plain_range.end <= fragment_end {
                return Some(Self::attributes_for_fragment(fragment));
            }
            cursor = fragment_end;
        }

        None
    }

    pub(crate) fn collapsed_caret_inherits_inline_code_style(&self) -> bool {
        self.selected_range.is_empty()
            && !self.uses_raw_text_editing()
            && self
                .insertion_attributes_for_display_offset(self.cursor_offset())
                .style
                .code
    }

    /// Apply new text to the block, running shortcut detection and
    /// updating the render cache, cursor, and selection state.  Emits
    /// [`BlockAction::Changed`] if the kind or text actually changed.
    pub(crate) fn apply_text_edit(
        &mut self,
        next_text: RichText,
        cursor_plain: usize,
        marked_range_clean: Option<Range<usize>>,
        selected_range_clean: Option<Range<usize>>,
        selected_range_reversed: Option<bool>,
        caret_may_have_closed_span: bool,
        cx: &mut Context<Self>,
    ) {
        let old_kind = self.record.kind.clone();
        let old_text = self.record.text.clone();
        let old_text_was_empty = old_text.plain_text().is_empty();
        let mut collapsed_affinity = self.display_collapsed_caret_affinity();
        let keep_projection =
            self.projection.is_some() && self.edit_mode.supports_inline_projection();

        let (next_kind, normalized_text, adjusted_cursor, shortcut_removed_len) =
            self.normalize_after_text_edit(next_text, cursor_plain);
        let should_restart_numbered_list = old_kind == BlockKind::Paragraph
            && old_text_was_empty
            && self.list_group_separator_candidate
            && next_kind == BlockKind::NumberedListItem;

        let next_marked_plain = marked_range_clean
            .as_ref()
            .map(|range| Self::adjust_range_for_shortcut(range, shortcut_removed_len));
        let next_selected_plain = selected_range_clean
            .as_ref()
            .map(|range| Self::adjust_range_for_shortcut(range, shortcut_removed_len))
            .unwrap_or_else(|| adjusted_cursor..adjusted_cursor);

        self.record.kind = next_kind;
        self.record.set_text(normalized_text);
        self.numbered_list_restart_requested = should_restart_numbered_list;
        self.sync_edit_mode_from_kind();
        self.sync_render_cache();
        // Rebuild when a projection already existed, or when this edit may have
        // closed a delimiter, creating a span whose markers now need projecting.
        if self.edit_mode.supports_inline_projection()
            && (keep_projection || caret_may_have_closed_span)
        {
            self.rebuild_inline_projection(next_selected_plain.clone(), next_marked_plain.clone());
        }

        // If the edit closed a span (its delimiters were absorbed), place the
        // caret after the new closing marker so typing continues as plain text.
        if caret_may_have_closed_span
            && next_selected_plain.is_empty()
            && self.projection.as_ref().is_some_and(|projection| {
                projection.caret_closes_span_at_plain(next_selected_plain.start)
            })
        {
            collapsed_affinity = CollapsedCaretAffinity::OuterEnd;
        }

        self.marked_range = next_marked_plain
            .clone()
            .map(|range| self.plain_to_display_range(range));
        if next_selected_plain.is_empty() {
            let offset = self.plain_to_display_cursor_offset_with_affinity(
                next_selected_plain.start,
                collapsed_affinity,
            );
            self.assign_collapsed_selection_offset(offset, collapsed_affinity, None);
        } else {
            self.selected_range = self.plain_to_display_range(next_selected_plain);
            self.selection_reversed = selected_range_reversed.unwrap_or(self.selection_reversed);
            self.collapsed_caret_affinity = CollapsedCaretAffinity::Default;
        }
        self.cursor_blink_epoch = Instant::now();
        self.clear_vertical_motion();

        if self.record.kind != old_kind || self.record.text != old_text {
            cx.emit(BlockAction::Changed);
        }
        cx.notify();
    }

    pub(crate) fn heading_projection_prefix_range(&self) -> Option<Range<usize>> {
        self.projection
            .as_ref()
            .and_then(|projection| projection.block_prefix_range.clone())
    }

    pub(crate) fn heading_source_offset_for_display_offset(
        &self,
        display_offset: usize,
        prefix_len: usize,
    ) -> usize {
        if display_offset <= prefix_len {
            display_offset
        } else {
            prefix_len
                + self
                    .display_range_to_source_range(display_offset..display_offset)
                    .start
        }
    }

    pub(crate) fn source_range_to_plain_range(
        text: &RichText,
        content_source_start: usize,
        range: Range<usize>,
    ) -> Range<usize> {
        let map = text.source_offset_map();
        map.source_to_plain_offset(range.start.saturating_sub(content_source_start))
            ..map.source_to_plain_offset(range.end.saturating_sub(content_source_start))
    }

    pub(crate) fn content_source_offset_to_display_offset(
        &self,
        content_source_start: usize,
        source_offset: usize,
    ) -> usize {
        if matches!(self.kind(), BlockKind::Heading { .. }) && source_offset <= content_source_start
        {
            source_offset
        } else {
            let plain = self
                .record
                .text
                .source_offset_map()
                .source_to_plain_offset(source_offset.saturating_sub(content_source_start));
            self.plain_to_display_cursor_offset(plain)
        }
    }

    pub(crate) fn apply_heading_prefix_edit(
        &mut self,
        display_range: Range<usize>,
        new_text: &str,
        selected_range_relative: Option<Range<usize>>,
        mark_inserted_text: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(prefix_range) = self.heading_projection_prefix_range() else {
            return false;
        };
        if display_range.start >= prefix_range.end {
            return false;
        }

        let prefix_len = prefix_range.len();
        let source_range = self
            .heading_source_offset_for_display_offset(display_range.start, prefix_len)
            ..self.heading_source_offset_for_display_offset(display_range.end, prefix_len);
        let mut source = format!(
            "{}{}",
            &self.display_text()[prefix_range],
            self.record.text.serialize_markdown()
        );
        source.replace_range(source_range.clone(), new_text);

        let selected_source_range = selected_range_relative
            .as_ref()
            .map(|relative| source_range.start + relative.start..source_range.start + relative.end);
        let cursor_source = selected_source_range
            .as_ref()
            .map(|range| range.end)
            .unwrap_or(source_range.start + new_text.len());
        let marked_source_range = if mark_inserted_text && !new_text.is_empty() {
            Some(source_range.start..source_range.start + new_text.len())
        } else {
            None
        };

        let (next_kind, next_text, content_source_start) =
            if let Some((level, content)) = BlockKind::parse_atx_heading_line(&source) {
                (
                    BlockKind::Heading { level },
                    RichText::from_markdown_with_link_references(
                        &content,
                        &self.link_reference_definitions,
                    ),
                    level as usize + 1,
                )
            } else {
                (
                    BlockKind::Paragraph,
                    RichText::from_markdown_with_link_references(
                        &source,
                        &self.link_reference_definitions,
                    ),
                    0,
                )
            };

        let next_selected_source = selected_source_range
            .clone()
            .unwrap_or(cursor_source..cursor_source);
        let next_selected_plain = Self::source_range_to_plain_range(
            &next_text,
            content_source_start,
            next_selected_source.clone(),
        );
        let next_marked_plain = marked_source_range.as_ref().map(|range| {
            Self::source_range_to_plain_range(&next_text, content_source_start, range.clone())
        });
        let old_kind = self.record.kind.clone();
        let old_text = self.record.text.clone();

        self.record.kind = next_kind;
        self.record.set_text(next_text);
        self.sync_edit_mode_from_kind();
        self.sync_render_cache();
        if self.edit_mode.supports_inline_projection() {
            self.rebuild_inline_projection(next_selected_plain, next_marked_plain);
        }

        self.selected_range = self.content_source_offset_to_display_offset(
            content_source_start,
            next_selected_source.start,
        )
            ..self.content_source_offset_to_display_offset(
                content_source_start,
                next_selected_source.end,
            );
        self.selection_reversed = false;
        self.marked_range = marked_source_range.map(|range| {
            self.content_source_offset_to_display_offset(content_source_start, range.start)
                ..self.content_source_offset_to_display_offset(content_source_start, range.end)
        });
        self.collapsed_caret_affinity = CollapsedCaretAffinity::Default;
        self.cursor_blink_epoch = Instant::now();
        self.clear_vertical_motion();

        if self.record.kind != old_kind || self.record.text != old_text {
            cx.emit(BlockAction::Changed);
        }
        cx.notify();
        true
    }

    /// Replace text in display coordinates: splice `new_text` into the text
    /// at `display_range`, re-parse inline markers, and update cursor state.
    /// When `mark_inserted_text` is true the inserted text becomes the IME
    /// marked range.
    ///
    /// When the block is in editing-expansion mode (code spans show `` ` ``
    /// delimiters), the `display_range` is first mapped back to the original
    /// tree's offset space.
    pub(crate) fn replace_text_in_display_range(
        &mut self,
        display_range: Range<usize>,
        new_text: &str,
        selected_range_relative: Option<Range<usize>>,
        mark_inserted_text: bool,
        cx: &mut Context<Self>,
    ) {
        if self.apply_heading_prefix_edit(
            display_range.clone(),
            new_text,
            selected_range_relative.clone(),
            mark_inserted_text,
            cx,
        ) {
            return;
        }

        let inserted_attributes = self.replacement_attributes_for_display_range(&display_range);

        // Inline `[label](url)` links round-trip through their projected source,
        // so edit them via the link projection even when the block is otherwise
        // source-preserving (for example it also contains inline math). This keeps
        // a link's anchor text editable the same way in every block; reference and
        // autolink links stay on the markdown-space path below, which preserves
        // their original source spelling.
        if !self.uses_raw_text_editing()
            && let Some(link_run) = self
                .projected_link_run_fully_covering_range(&display_range)
                .filter(|run| !run.link.is_source_preserving())
                .cloned()
        {
            self.apply_link_projection_edit(
                &link_run,
                display_range,
                new_text,
                selected_range_relative,
                mark_inserted_text,
                cx,
            );
            return;
        }

        if self.should_use_source_space_link_edit() {
            self.apply_source_space_text_edit(
                display_range,
                new_text,
                selected_range_relative,
                mark_inserted_text,
                cx,
            );
            return;
        }

        // Editing outside an inline link's run would otherwise re-derive the
        // inline tree from collapsed plain text, which no longer contains the
        // `[label](url)` markers and silently drops the link. Edit in markdown
        // space (as source-preserving links already do) so the link round-trips.
        if !self.uses_raw_text_editing() && self.record.text.has_inline_links() {
            self.apply_source_space_text_edit(
                display_range,
                new_text,
                selected_range_relative,
                mark_inserted_text,
                cx,
            );
            return;
        }

        let plain_range = self.display_to_plain_range(display_range.clone());
        let mut base_text = self.record.text.clone();
        let overlaps_delimiters = self.projection.is_some() && !self.uses_raw_text_editing();
        if overlaps_delimiters {
            let touched_styles = self.projected_styles_touching_display_range(&display_range);
            if !touched_styles.is_empty() {
                base_text.unwrap_styles_on_fragments(&touched_styles);
            }
        }

        let base_plain_len = base_text.plain_text().len();
        let replaced_text = self.display_text()[display_range.clone()].to_string();
        let result = if self.uses_raw_text_editing() {
            base_text.replace_visible_range_raw(
                plain_range.clone(),
                new_text,
                InlineInsertionAttributes::default(),
            )
        } else {
            base_text.replace_visible_range_with_link_references(
                plain_range.clone(),
                new_text,
                inserted_attributes,
                &self.link_reference_definitions,
            )
        };

        // A span was closed when re-parsing absorbed delimiters into a style,
        // leaving the plain text shorter than expected. Skip IME and deletions.
        let expected_plain_len =
            base_plain_len.saturating_sub(plain_range.len()) + new_text.len();
        let caret_may_have_closed_span = !self.uses_raw_text_editing()
            && !new_text.is_empty()
            && !mark_inserted_text
            && result.tree.plain_text().len() < expected_plain_len;
        let quote_structure_edit = !self.uses_raw_text_editing()
            && self.quote_depth > 0
            && (new_text.contains('\n')
                || replaced_text.contains('\n')
                || (self.kind() == BlockKind::Blockquote
                    && Self::multiline_quote_edit_requires_reparse(&result.tree.plain_text())));
        if quote_structure_edit {
            self.quote_reparse_requested = true;
        }
        let inserted_range = plain_range.start..plain_range.start + new_text.len();
        let marked_range = if mark_inserted_text && !new_text.is_empty() {
            Some(result.map_range(&inserted_range))
        } else {
            None
        };
        let selected_range = selected_range_relative.as_ref().map(|relative| {
            let absolute = plain_range.start + relative.start..plain_range.start + relative.end;
            result.map_range(&absolute)
        });
        let cursor = selected_range
            .as_ref()
            .map(|range| range.end)
            .unwrap_or_else(|| result.map_offset(plain_range.start + new_text.len()));

        self.apply_text_edit(
            result.tree,
            cursor,
            marked_range,
            selected_range.clone(),
            selected_range
                .as_ref()
                .and_then(|range| (!range.is_empty()).then_some(false)),
            caret_may_have_closed_span,
            cx,
        );
    }
}
