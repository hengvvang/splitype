//! Projected inline edits: headings, links, text replacement, and undo.

use std::ops::Range;

use gpui::*;

use crate::editor::document::protocol::{BlockEvent, UndoCaptureKind};
use crate::editor::projection::{ExpandedInlineSegmentKind, ExpandedLinkSpan};
use crate::editor::document::block::Block;
use crate::editor::document::block::CollapsedCaretAffinity;
use markdown::inline::text::{BlockText, InlineFragment, InlineInsertionAttributes};
use markdown::parse::BlockKind;
use std::time::Instant;

impl Block {
    pub(crate) fn prepare_undo_capture(&self, kind: UndoCaptureKind, cx: &mut Context<Self>) {
        cx.emit(BlockEvent::PrepareUndo {
            kind,
            target_block_id: Some(self.data.id),
            initial_text: Some(self.data.text.clone()),
        });
    }

    /// Detect Markdown shortcut prefixes in the edited text and convert the
    /// block's kind accordingly (e.g. `"- " -> BulletedListItem`).
    ///
    /// Only triggers when the current kind is [`BlockKind::Paragraph`].
    /// Returns the potentially updated kind, the text with prefix stripped,
    /// the new cursor offset, and the number of prefix characters removed.
    pub(crate) fn normalize_after_text_edit(
        &self,
        mut next_text: BlockText,
        cursor: usize,
    ) -> (BlockKind, BlockText, usize, usize) {
        if self.is_table_cell() {
            return (self.kind(), next_text, cursor, 0);
        }

        if !self.edits_verbatim_text() && self.kind() == BlockKind::Paragraph {
            let plain_text = next_text.plain_text();
            if let Some((kind, prefix_len)) = BlockKind::detect_markdown_shortcut(&plain_text) {
                next_text.remove_plain_prefix(prefix_len);
                return (
                    kind,
                    next_text,
                    cursor.saturating_sub(prefix_len),
                    prefix_len,
                );
            }
        }

        if !self.edits_verbatim_text() && self.kind() == BlockKind::BulletListItem {
            let plain_text = next_text.plain_text();
            if let Some((checked, prefix_len)) = BlockKind::parse_task_list_item_prefix(&plain_text)
            {
                next_text.remove_plain_prefix(prefix_len);
                return (
                    BlockKind::TaskListItem { checked },
                    next_text,
                    cursor.saturating_sub(prefix_len),
                    prefix_len,
                );
            }
        }

        // A focused separator shows its marker text for editing; once the text
        // no longer forms a valid thematic break (e.g. `--` after deleting one
        // dash), the block falls back to a paragraph so Enter splits normally
        // instead of leaving a phantom separator behind.
        if !self.edits_verbatim_text() && self.kind().is_thematic_break() {
            let plain_text = next_text.plain_text();
            if !BlockKind::parse_thematic_break_line(&plain_text) {
                return (BlockKind::Paragraph, next_text, cursor, 0);
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

    pub(crate) fn replacement_is_pure_link_span(fragments: &[InlineFragment]) -> bool {
        let Some(first_link) = fragments
            .first()
            .and_then(|fragment| fragment.link())
        else {
            return false;
        };

        fragments
            .iter()
            .all(|fragment| fragment.link() == Some(first_link))
    }

    pub(crate) fn apply_link_projection_edit(
        &mut self,
        link_span: &ExpandedLinkSpan,
        display_range: Range<usize>,
        new_text: &str,
        selected_range_relative: Option<Range<usize>>,
        mark_inserted_text: bool,
        cx: &mut Context<Self>,
    ) {
        let local_visible_range = display_range.start - link_span.display_range.start
            ..display_range.end - link_span.display_range.start;
        let local_display_text = self.display_text()[link_span.display_range.clone()].to_string();
        let local_tree = BlockText::plain(local_display_text);
        let local_result = local_tree.replace_plain_range_with_link_references(
            local_visible_range.clone(),
            new_text,
            InlineInsertionAttributes::default(),
            &self.link_reference_definitions,
        );
        let replacement_fragments = local_result.tree.fragments.clone();

        let replacement_start = link_span.start_fragment_index;
        let replacement_clean_start =
            Self::clean_offset_before_fragment_index(&self.data.text.fragments, replacement_start);
        let mut next_text = self.data.text.clone();
        next_text.replace_fragment_range(
            link_span.start_fragment_index..link_span.end_fragment_index,
            replacement_fragments.clone(),
        );

        if Self::replacement_is_pure_link_span(&replacement_fragments) {
            let old_kind = self.data.kind.clone();
            let old_text = self.data.text.clone();
            self.data.set_text(next_text.clone());
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
            if let Some(projected_link_span) = self.projection.as_ref().and_then(|projection| {
                projection
                    .link_spans
                    .iter()
                    .find(|span| span.plain_range == selected_plain)
            }) {
                let start = projected_link_span.display_range.start
                    + local_selected
                        .start
                        .min(projected_link_span.display_range.len());
                let end = projected_link_span.display_range.start
                    + local_selected
                        .end
                        .min(projected_link_span.display_range.len());
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
                if self.data.kind != old_kind || self.data.text != old_text {
                    cx.emit(BlockEvent::Changed);
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
        if self.edits_verbatim_text() {
            return InlineInsertionAttributes::default();
        }

        if self.projection.is_none() {
            return self.data.text.attributes_for_insertion_at(display_offset);
        }

        for segment in self.projection_segments() {
            match segment.kind {
                ExpandedInlineSegmentKind::StyledText
                    if display_offset >= segment.display_range.start
                        && display_offset <= segment.display_range.end =>
                {
                    let fragment = &self.data.text.fragments[segment.fragment_index];
                    let mut attrs = fragment.attributes();
                    attrs.math = None;
                    return attrs;
                }
                ExpandedInlineSegmentKind::OpeningDelimiter(_)
                    if display_offset == segment.display_range.end =>
                {
                    let fragment = &self.data.text.fragments[segment.fragment_index];
                    let mut attrs = fragment.attributes();
                    attrs.math = None;
                    return attrs;
                }
                ExpandedInlineSegmentKind::ClosingDelimiter(_)
                    if display_offset == segment.display_range.start =>
                {
                    let fragment = &self.data.text.fragments[segment.fragment_index];
                    let mut attrs = fragment.attributes();
                    attrs.math = None;
                    return attrs;
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
                        && let Some(link_span) = self
                            .projection
                            .as_ref()
                            .and_then(|projection| projection.link_spans.get(link_group))
                        && display_offset >= link_span.target_display_range.start
                        && display_offset <= link_span.target_display_range.end
                    {
                        return InlineInsertionAttributes::default();
                    }
                }
                _ => {}
            }
        }

        self.data
            .text
            .attributes_for_insertion_at(self.display_to_plain_offset(display_offset))
    }

    pub(crate) fn collapsed_caret_inherits_inline_code_style(&self) -> bool {
        self.selected_range.is_empty()
            && !self.edits_verbatim_text()
            && self
                .insertion_attributes_for_display_offset(self.cursor_offset())
                .style
                .code
    }

    /// Apply new text to the block, running shortcut detection and
    /// updating the render cache, cursor, and selection state.  Emits
    /// [`BlockEvent::Changed`] if the kind or text actually changed.
    pub(crate) fn apply_text_edit(
        &mut self,
        next_text: BlockText,
        cursor_plain: usize,
        marked_range_clean: Option<Range<usize>>,
        selected_range_clean: Option<Range<usize>>,
        selected_range_reversed: Option<bool>,
        caret_may_have_closed_span: bool,
        cx: &mut Context<Self>,
    ) {
        let old_kind = self.data.kind.clone();
        let old_text = self.data.text.clone();
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

        self.data.kind = next_kind;
        self.data.set_text(normalized_text);
        self.numbered_list_restart_requested = should_restart_numbered_list;
        self.sync_edit_mode_from_kind();
        self.sync_render_cache();
        let has_styled_spans = self
            .data
            .text
            .fragments
            .iter()
            .any(|f| f.style != markdown::inline::style::InlineStyle::default() || f.extra.is_some());
        if self.edit_mode.supports_inline_projection()
            && (keep_projection || caret_may_have_closed_span || has_styled_spans)
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

        if self.data.kind != old_kind || self.data.text != old_text {
            cx.emit(BlockEvent::Changed);
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
        text: &BlockText,
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
        if (matches!(self.kind(), BlockKind::Heading { .. })
            || matches!(self.kind(), BlockKind::Callout(_)))
            && source_offset <= content_source_start
        {
            source_offset
        } else {
            let plain = self
                .data
                .text
                .source_offset_map()
                .source_to_plain_offset(source_offset.saturating_sub(content_source_start));
            self.plain_to_display_cursor_offset(plain)
        }
    }

    pub(crate) fn callout_projection_prefix_range(&self) -> Option<Range<usize>> {
        if matches!(self.kind(), BlockKind::Callout(_)) {
            self.projection
                .as_ref()
                .and_then(|projection| projection.block_prefix_range.clone())
        } else {
            None
        }
    }

    pub(crate) fn apply_callout_prefix_edit(
        &mut self,
        display_range: Range<usize>,
        new_text: &str,
        selected_range_relative: Option<Range<usize>>,
        mark_inserted_text: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let is_callout = matches!(self.kind(), BlockKind::Callout(_));
        let is_blockquote = self.kind() == BlockKind::Blockquote;
        if !is_callout && !is_blockquote {
            return false;
        }

        if is_callout {
            let Some(prefix_range) = self.callout_projection_prefix_range() else {
                return false;
            };
            if display_range.start > prefix_range.end {
                return false;
            }
        }

        let display_text = self.display_text().to_string();
        let mut source = display_text;
        if display_range.start <= source.len() && display_range.end <= source.len() {
            source.replace_range(display_range.clone(), new_text);
        } else {
            return false;
        }

        let parsed_callout = markdown::block::CalloutKind::parse_header_line(&source);
        if is_blockquote && parsed_callout.is_none() {
            return false;
        }

        let selected_source_range = selected_range_relative
            .as_ref()
            .map(|relative| display_range.start + relative.start..display_range.start + relative.end);
        let cursor_source = selected_source_range
            .as_ref()
            .map(|range| range.end)
            .unwrap_or(display_range.start + new_text.len());
        let marked_source_range = if mark_inserted_text && !new_text.is_empty() {
            Some(display_range.start..display_range.start + new_text.len())
        } else {
            None
        };

        let (next_kind, next_text, content_source_start) =
            if let Some((parsed_variant, content)) = parsed_callout {
                let prefix_str = if content.is_empty() {
                    format!("[!{}]", parsed_variant.marker_lower())
                } else {
                    format!("[!{}] ", parsed_variant.marker_lower())
                };
                (
                    BlockKind::Callout(parsed_variant),
                    BlockText::from_markdown_with_link_references(
                        &content,
                        &self.link_reference_definitions,
                    ),
                    prefix_str.len(),
                )
            } else {
                (
                    BlockKind::Blockquote,
                    BlockText::from_markdown_with_link_references(
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

        self.data.kind = next_kind;
        self.data.set_text(next_text);
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
        self.cursor_blink_epoch = std::time::Instant::now();
        cx.emit(BlockEvent::Changed);
        cx.notify();
        true
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
            self.data.text.serialize_markdown()
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
                    BlockText::from_markdown_with_link_references(
                        &content,
                        &self.link_reference_definitions,
                    ),
                    level as usize + 1,
                )
            } else {
                (
                    BlockKind::Paragraph,
                    BlockText::from_markdown_with_link_references(
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
        let old_kind = self.data.kind.clone();
        let old_text = self.data.text.clone();

        self.data.kind = next_kind;
        self.data.set_text(next_text);
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

        if self.data.kind != old_kind || self.data.text != old_text {
            cx.emit(BlockEvent::Changed);
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
        if self.apply_callout_prefix_edit(
            display_range.clone(),
            new_text,
            selected_range_relative.clone(),
            mark_inserted_text,
            cx,
        ) {
            return;
        }

        if self.apply_heading_prefix_edit(
            display_range.clone(),
            new_text,
            selected_range_relative.clone(),
            mark_inserted_text,
            cx,
        ) {
            return;
        }

        // Inline `[label](url)` links round-trip through their projected source,
        // so edit them via the link projection even when the block is otherwise
        // source-preserving (for example it also contains inline math). This keeps
        // a link's anchor text editable the same way in every block; reference and
        // autolink links stay on the markdown-space path below, which preserves
        // their original source spelling.
        if !self.edits_verbatim_text()
            && let Some(link_span) = self
                .projected_link_span_fully_covering_range(&display_range)
                .filter(|span| !span.link.is_source_preserving())
                .cloned()
        {
            self.apply_link_projection_edit(
                &link_span,
                display_range,
                new_text,
                selected_range_relative,
                mark_inserted_text,
                cx,
            );
            return;
        }

        if !self.edits_verbatim_text() {
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
        let base_text = self.data.text.clone();
        let result = base_text.replace_plain_range_verbatim(
            plain_range.clone(),
            new_text,
            InlineInsertionAttributes::default(),
        );

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
            false,
            cx,
        );
    }
}
