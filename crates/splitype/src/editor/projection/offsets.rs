//! Plain ↔ Display ↔ Source 3D coordinate projection mapping.

use std::ops::Range;

use crate::editor::projection::ExpandedInlineSegmentKind;
use crate::editor::document::block::{Block, CollapsedCaretAffinity};

impl Block {
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

    pub(crate) fn display_offset_to_source_offset(&self, display_offset: usize) -> usize {
        if self.edits_verbatim_text() || self.kind().is_code_block() {
            return display_offset.min(self.display_len());
        }

        if let Some(link_span) =
            self.projected_link_span_fully_covering_range(&(display_offset..display_offset))
        {
            let map = self.data.text.source_offset_map();
            let label_source_start = map.plain_to_source_offset(link_span.plain_range.start);
            let span_source_start =
                label_source_start.saturating_sub(link_span.link.open_marker().len());
            let local_display = display_offset
                .saturating_sub(link_span.display_range.start)
                .min(link_span.display_range.len());
            return span_source_start + local_display;
        }

        if let Some(footnote_span) = self.projection.as_ref().and_then(|projection| {
            projection.footnote_span_fully_covering_range(&(display_offset..display_offset))
        }) {
            let raw = footnote_span.footnote.raw_markdown();
            let raw_len = raw.len();
            let local_offset = display_offset
                .saturating_sub(footnote_span.display_range.start)
                .min(footnote_span.display_range.len());
            let mapped = (raw_len * local_offset) / footnote_span.display_range.len().max(1);
            let map = self.data.text.source_offset_map();
            let span_source_start = map.plain_to_source_offset(footnote_span.plain_range.start);
            return span_source_start + mapped;
        }

        if let Some(projection) = &self.projection {
            let map = self.data.text.source_offset_map();
            let segments = &projection.segments;

            for (seg_idx, segment) in segments.iter().enumerate() {
                if display_offset >= segment.display_range.start
                    && display_offset <= segment.display_range.end
                {
                    let frag_plain_start = self.data.text.fragments[..segment.fragment_index]
                        .iter()
                        .map(|f| f.text.len())
                        .sum::<usize>();
                    let frag_text_len = self
                        .data
                        .text
                        .fragments
                        .get(segment.fragment_index)
                        .map(|f| f.text.len())
                        .unwrap_or(0);
                    let source_text_start = map.plain_to_source_offset(frag_plain_start);
                    let source_text_end = source_text_start + frag_text_len;

                    match segment.kind {
                        ExpandedInlineSegmentKind::OpeningDelimiter(_) => {
                            let mut first_open = segment.display_range.start;
                            let mut last_open = segment.display_range.end;
                            for prev_seg in segments[..seg_idx].iter().rev() {
                                if prev_seg.fragment_index == segment.fragment_index
                                    && matches!(
                                        prev_seg.kind,
                                        ExpandedInlineSegmentKind::OpeningDelimiter(_)
                                    )
                                {
                                    first_open = prev_seg.display_range.start;
                                } else {
                                    break;
                                }
                            }
                            for next_seg in segments[seg_idx + 1..].iter() {
                                if next_seg.fragment_index == segment.fragment_index
                                    && matches!(
                                        next_seg.kind,
                                        ExpandedInlineSegmentKind::OpeningDelimiter(_)
                                    )
                                {
                                    last_open = next_seg.display_range.end;
                                } else {
                                    break;
                                }
                            }
                            let total_open_len = last_open - first_open;
                            let source_open_start =
                                source_text_start.saturating_sub(total_open_len);
                            return source_open_start + (display_offset - first_open);
                        }
                        ExpandedInlineSegmentKind::StyledText => {
                            return source_text_start
                                + (display_offset - segment.display_range.start);
                        }
                        ExpandedInlineSegmentKind::ClosingDelimiter(_) => {
                            let mut first_close = segment.display_range.start;
                            for prev_seg in segments[..seg_idx].iter().rev() {
                                if prev_seg.fragment_index == segment.fragment_index
                                    && matches!(
                                        prev_seg.kind,
                                        ExpandedInlineSegmentKind::ClosingDelimiter(_)
                                    )
                                {
                                    first_close = prev_seg.display_range.start;
                                } else {
                                    break;
                                }
                            }
                            return source_text_end + (display_offset - first_close);
                        }
                        ExpandedInlineSegmentKind::PlainText => {
                            return source_text_start
                                + (display_offset - segment.display_range.start);
                        }
                        ExpandedInlineSegmentKind::BlockPrefix => {
                            return 0;
                        }
                        _ => {}
                    }
                }
            }
        }

        let plain_offset = self.display_to_plain_offset(display_offset);
        self.data
            .text
            .source_offset_map()
            .plain_to_source_offset(plain_offset)
    }

    pub(crate) fn display_range_to_source_range(&self, range: Range<usize>) -> Range<usize> {
        let start = self.display_offset_to_source_offset(range.start);
        let end = self.display_offset_to_source_offset(range.end);
        start.min(end)..start.max(end)
    }

    pub(crate) fn source_offset_to_display_offset(&self, source_offset: usize) -> usize {
        if self.edits_verbatim_text() || self.kind().is_code_block() {
            return source_offset.min(self.display_len());
        }

        if let Some(projection) = &self.projection {
            let map = self.data.text.source_offset_map();
            let segments = &projection.segments;

            for (seg_idx, segment) in segments.iter().enumerate() {
                let frag_plain_start = self.data.text.fragments[..segment.fragment_index]
                    .iter()
                    .map(|f| f.text.len())
                    .sum::<usize>();
                let frag_text_len = self
                    .data
                    .text
                    .fragments
                    .get(segment.fragment_index)
                    .map(|f| f.text.len())
                    .unwrap_or(0);
                let source_text_start = map.plain_to_source_offset(frag_plain_start);
                let source_text_end = source_text_start + frag_text_len;

                match segment.kind {
                    ExpandedInlineSegmentKind::OpeningDelimiter(_) => {
                        let mut first_open = segment.display_range.start;
                        let mut last_open = segment.display_range.end;
                        for prev_seg in segments[..seg_idx].iter().rev() {
                            if prev_seg.fragment_index == segment.fragment_index
                                && matches!(
                                    prev_seg.kind,
                                    ExpandedInlineSegmentKind::OpeningDelimiter(_)
                                )
                            {
                                first_open = prev_seg.display_range.start;
                            } else {
                                break;
                            }
                        }
                        for next_seg in segments[seg_idx + 1..].iter() {
                            if next_seg.fragment_index == segment.fragment_index
                                && matches!(
                                    next_seg.kind,
                                    ExpandedInlineSegmentKind::OpeningDelimiter(_)
                                )
                            {
                                last_open = next_seg.display_range.end;
                            } else {
                                break;
                            }
                        }
                        let total_open_len = last_open - first_open;
                        let source_open_start = source_text_start.saturating_sub(total_open_len);
                        if source_offset >= source_open_start && source_offset <= source_text_start
                        {
                            return first_open + (source_offset - source_open_start);
                        }
                    }
                    ExpandedInlineSegmentKind::StyledText => {
                        if source_offset >= source_text_start && source_offset <= source_text_end {
                            return segment.display_range.start
                                + (source_offset - source_text_start);
                        }
                    }
                    ExpandedInlineSegmentKind::ClosingDelimiter(_) => {
                        let mut first_close = segment.display_range.start;
                        let mut last_close = segment.display_range.end;
                        for prev_seg in segments[..seg_idx].iter().rev() {
                            if prev_seg.fragment_index == segment.fragment_index
                                && matches!(
                                    prev_seg.kind,
                                    ExpandedInlineSegmentKind::ClosingDelimiter(_)
                                )
                            {
                                first_close = prev_seg.display_range.start;
                            } else {
                                break;
                            }
                        }
                        for next_seg in segments[seg_idx + 1..].iter() {
                            if next_seg.fragment_index == segment.fragment_index
                                && matches!(
                                    next_seg.kind,
                                    ExpandedInlineSegmentKind::ClosingDelimiter(_)
                                )
                            {
                                last_close = next_seg.display_range.end;
                            } else {
                                break;
                            }
                        }
                        let total_close_len = last_close - first_close;
                        let source_close_end = source_text_end + total_close_len;
                        if source_offset >= source_text_end && source_offset <= source_close_end {
                            return first_close + (source_offset - source_text_end);
                        }
                    }
                    ExpandedInlineSegmentKind::PlainText => {
                        let source_end = source_text_start + segment.display_range.len();
                        if source_offset >= source_text_start && source_offset <= source_end {
                            return segment.display_range.start
                                + (source_offset - source_text_start);
                        }
                    }
                    _ => {}
                }
            }
        }

        let plain_offset = self
            .data
            .text
            .source_offset_map()
            .source_to_plain_offset(source_offset);
        self.plain_to_display_cursor_offset(plain_offset)
    }

    pub(crate) fn source_range_to_display_range(&self, range: Range<usize>) -> Range<usize> {
        let start = self.source_offset_to_display_offset(range.start);
        let end = self.source_offset_to_display_offset(range.end);
        start.min(end)..start.max(end)
    }
}
