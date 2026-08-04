//! Pure text layout geometry: hard-line splitting, wrapped-line math,
//! offset-to-position mapping, and link/footnote hit-testing.
//!
//! These helpers operate on shaped lines and block state only — no GPUI
//! elements, themes, or rendering.

use std::ops::Range;

use gpui::*;

use crate::editor::tree::block::Block;
use crate::model::inline::footnote::InlineFootnoteHit;
use crate::model::inline::link::InlineLinkHit;

const SOURCE_LINE_NUMBER_MIN_DIGITS: usize = 2;
const SOURCE_LINE_NUMBER_GAP: f32 = 12.0;
const SOURCE_LINE_NUMBER_DIGIT_WIDTH_RATIO: f32 = 0.62;

pub(crate) fn source_line_count(text: &str) -> usize {
    text.split('\n').count().max(1)
}

pub(crate) fn source_line_number_gutter_width(line_count: usize, font_size: Pixels) -> Pixels {
    let digits = line_count
        .max(1)
        .to_string()
        .len()
        .max(SOURCE_LINE_NUMBER_MIN_DIGITS);
    px(digits as f32 * f32::from(font_size) * SOURCE_LINE_NUMBER_DIGIT_WIDTH_RATIO)
        + px(SOURCE_LINE_NUMBER_GAP)
}

pub(crate) fn source_text_bounds(bounds: Bounds<Pixels>, gutter_width: Pixels) -> Bounds<Pixels> {
    if gutter_width <= px(0.0) {
        return bounds;
    }

    let max_gutter = (f32::from(bounds.size.width) - 1.0).max(0.0);
    let gutter_width = px(f32::from(gutter_width).min(max_gutter));
    Bounds::new(
        point(bounds.origin.x + gutter_width, bounds.origin.y),
        size(
            (bounds.size.width - gutter_width).max(px(1.0)),
            bounds.size.height,
        ),
    )
}

pub(crate) fn source_line_number_tops(lines: &[WrappedLine], line_height: Pixels) -> Vec<Pixels> {
    let mut tops = Vec::with_capacity(lines.len());
    let mut y = Pixels::default();
    for line in lines {
        tops.push(y);
        y += wrapped_line_height(line, line_height);
    }
    tops
}


pub(crate) fn hard_line_ranges(text: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = 0;
    for (idx, _) in text.match_indices('\n') {
        ranges.push(start..idx);
        start = idx + 1;
    }
    ranges.push(start..text.len());
    ranges
}

/// Map a flat visible-text offset to `(line_index, offset_within_line)`.
pub(crate) fn line_index_for_offset(ranges: &[Range<usize>], offset: usize) -> (usize, usize) {
    let clamped = offset.min(ranges.last().map(|r| r.end).unwrap_or(0));
    for (i, range) in ranges.iter().enumerate() {
        if clamped <= range.end {
            return (i, clamped.saturating_sub(range.start));
        }
    }
    let last = ranges.len() - 1;
    (last, ranges[last].len())
}

pub(crate) fn aligned_line_left(
    line: &WrappedLine,
    bounds: Bounds<Pixels>,
    align: TextAlign,
) -> Pixels {
    let slack = (bounds.size.width - line.width()).max(px(0.0));
    match align {
        TextAlign::Left => bounds.left(),
        TextAlign::Center => bounds.left() + slack / 2.0,
        TextAlign::Right => bounds.left() + slack,
    }
}

pub(crate) fn wrapped_line_height(line: &WrappedLine, line_height: Pixels) -> Pixels {
    line.size(line_height).height
}

pub(crate) fn wrapped_line_top(
    lines: &[WrappedLine],
    line_height: Pixels,
    line_idx: usize,
) -> Pixels {
    lines.iter().take(line_idx).fold(px(0.0), |height, line| {
        height + wrapped_line_height(line, line_height)
    })
}

pub(crate) fn wrapped_line_for_y(
    lines: &[WrappedLine],
    line_height: Pixels,
    relative_y: Pixels,
) -> Option<(usize, Pixels)> {
    if lines.is_empty() {
        return None;
    }

    let mut top = px(0.0);
    for (line_idx, line) in lines.iter().enumerate() {
        let height = wrapped_line_height(line, line_height);
        if relative_y < top + height || line_idx + 1 == lines.len() {
            return Some((line_idx, (relative_y - top).max(px(0.0))));
        }
        top += height;
    }

    Some((lines.len() - 1, px(0.0)))
}

pub(crate) fn wrap_boundary_offset(line: &WrappedLine, wrap_idx: usize) -> Option<usize> {
    let boundary = line.wrap_boundaries().get(wrap_idx)?;
    let run = line.unwrapped_layout.runs.get(boundary.run_ix)?;
    let glyph = run.glyphs.get(boundary.glyph_ix)?;
    Some(glyph.index)
}

pub(crate) fn wrapped_row_offsets(line: &WrappedLine) -> Vec<usize> {
    let mut offsets = Vec::with_capacity(line.wrap_boundaries().len() + 2);
    offsets.push(0);
    for wrap_idx in 0..line.wrap_boundaries().len() {
        if let Some(offset) = wrap_boundary_offset(line, wrap_idx) {
            offsets.push(offset.min(line.len()));
        }
    }
    offsets.push(line.len());
    offsets.dedup();
    offsets
}

pub(crate) fn wrapped_row_origin_x(
    line: &WrappedLine,
    bounds: Bounds<Pixels>,
    align: TextAlign,
    row_start: usize,
    row_end: usize,
) -> Pixels {
    let row_width =
        line.unwrapped_layout.x_for_index(row_end) - line.unwrapped_layout.x_for_index(row_start);
    let align_width = line.width();
    let slack = (align_width - row_width).max(px(0.0));
    let line_left = aligned_line_left(line, bounds, align);
    match align {
        TextAlign::Left => line_left,
        TextAlign::Center => line_left + slack / 2.0,
        TextAlign::Right => line_left + slack,
    }
}

pub(crate) fn position_for_offset(
    line: &WrappedLine,
    offset: usize,
    line_height: Pixels,
    prefer_next_wrap_start: bool,
) -> Option<Point<Pixels>> {
    let offsets = wrapped_row_offsets(line);
    for row_idx in 0..offsets.len().saturating_sub(1) {
        let row_start = offsets[row_idx];
        let row_end = offsets[row_idx + 1];
        let is_start_of_wrapped_row = prefer_next_wrap_start && row_idx > 0 && offset == row_start;
        if is_start_of_wrapped_row || (offset >= row_start && offset < row_end) {
            let row_start_x = line.unwrapped_layout.x_for_index(row_start);
            let x = line.unwrapped_layout.x_for_index(offset) - row_start_x;
            return Some(point(x, line_height * row_idx as f32));
        }
    }

    line.position_for_index(offset, line_height)
}

pub(crate) fn cursor_bounds_for_offset(
    lines: &[WrappedLine],
    bounds: Bounds<Pixels>,
    line_height: Pixels,
    text: &str,
    offset: usize,
    align: TextAlign,
    cursor_width: Pixels,
) -> Option<Bounds<Pixels>> {
    let ranges = hard_line_ranges(text);
    let (line_idx, offset_in_line) = line_index_for_offset(&ranges, offset);
    let layout = lines.get(line_idx)?;
    let origin_x = aligned_line_left(layout, bounds, align);
    let cursor_pos = position_for_offset(layout, offset_in_line, line_height, true)?;
    let y_offset = bounds.top() + wrapped_line_top(lines, line_height, line_idx);
    Some(Bounds::new(
        point(origin_x + cursor_pos.x, y_offset + cursor_pos.y),
        size(cursor_width, line_height),
    ))
}

pub(crate) fn range_bounds(
    lines: &[WrappedLine],
    bounds: Bounds<Pixels>,
    line_height: Pixels,
    text: &str,
    range: Range<usize>,
    align: TextAlign,
) -> Option<Bounds<Pixels>> {
    let segments = range_segment_bounds(lines, bounds, line_height, text, range.clone(), align);
    if segments.is_empty() {
        return cursor_bounds_for_offset(
            lines,
            bounds,
            line_height,
            text,
            range.start,
            align,
            px(1.0),
        );
    }

    let mut union = segments[0];
    for segment in segments.iter().skip(1) {
        union = Bounds::from_corners(
            point(
                union.left().min(segment.left()),
                union.top().min(segment.top()),
            ),
            point(
                union.right().max(segment.right()),
                union.bottom().max(segment.bottom()),
            ),
        );
    }
    Some(union)
}

pub(crate) fn range_segment_bounds_for_hard_line(
    lines: &[WrappedLine],
    bounds: Bounds<Pixels>,
    line_height: Pixels,
    line_idx: usize,
    start_offset: usize,
    end_offset: usize,
    align: TextAlign,
) -> Vec<Bounds<Pixels>> {
    let Some(line) = lines.get(line_idx) else {
        return Vec::new();
    };
    let line_top = bounds.top() + wrapped_line_top(lines, line_height, line_idx);
    let offsets = wrapped_row_offsets(line);
    let mut segments = Vec::new();

    for row_idx in 0..offsets.len().saturating_sub(1) {
        let row_start = offsets[row_idx];
        let row_end = offsets[row_idx + 1];
        let seg_start = start_offset.max(row_start).min(row_end);
        let seg_end = end_offset.min(row_end).max(row_start);
        if seg_start >= seg_end {
            continue;
        }

        let row_start_x = line.unwrapped_layout.x_for_index(row_start);
        let start_x = line.unwrapped_layout.x_for_index(seg_start) - row_start_x;
        let end_x = line.unwrapped_layout.x_for_index(seg_end) - row_start_x;
        let origin_x = wrapped_row_origin_x(line, bounds, align, row_start, row_end);
        let y = line_top + line_height * row_idx as f32;
        segments.push(Bounds::from_corners(
            point(origin_x + start_x, y),
            point(origin_x + end_x, y + line_height),
        ));
    }

    segments
}

pub(crate) fn range_segment_bounds(
    lines: &[WrappedLine],
    bounds: Bounds<Pixels>,
    line_height: Pixels,
    text: &str,
    range: Range<usize>,
    align: TextAlign,
) -> Vec<Bounds<Pixels>> {
    if range.start >= range.end || lines.is_empty() {
        return Vec::new();
    }

    let ranges = hard_line_ranges(text);
    let (start_line, start_offset) = line_index_for_offset(&ranges, range.start);
    let (end_line, end_offset) = line_index_for_offset(&ranges, range.end);
    let mut segments = Vec::new();

    for line_idx in start_line..=end_line {
        let hard_range = &ranges[line_idx];
        let line_start = if line_idx == start_line {
            start_offset
        } else {
            0
        };
        let line_end = if line_idx == end_line {
            end_offset
        } else {
            hard_range.len()
        };
        segments.extend(range_segment_bounds_for_hard_line(
            lines,
            bounds,
            line_height,
            line_idx,
            line_start,
            line_end,
            align,
        ));
    }

    segments
}

pub(crate) fn point_inside_bounds(bounds: Bounds<Pixels>, position: Point<Pixels>) -> bool {
    position.x >= bounds.left()
        && position.x < bounds.right()
        && position.y >= bounds.top()
        && position.y < bounds.bottom()
}

pub(crate) fn link_at_position<'a>(
    input: &'a Block,
    lines: &[WrappedLine],
    bounds: Bounds<Pixels>,
    line_height: Pixels,
    position: Point<Pixels>,
) -> Option<&'a InlineLinkHit> {
    if input.is_source_raw_mode()
        || input.display_text().is_empty()
        || lines.is_empty()
        || position.y < bounds.top()
        || position.y >= bounds.bottom()
    {
        return None;
    }

    let text = input.display_text();
    let align = input.text_align();

    for span in input.inline_spans() {
        let Some(link) = span.link.as_ref() else {
            continue;
        };
        if span.range.is_empty() {
            continue;
        }

        for link_bounds in
            range_segment_bounds(lines, bounds, line_height, text, span.range.clone(), align)
        {
            if point_inside_bounds(link_bounds, position) {
                return Some(link);
            }
        }
    }

    None
}

pub(crate) fn footnote_at_position<'a>(
    input: &'a Block,
    lines: &[WrappedLine],
    bounds: Bounds<Pixels>,
    line_height: Pixels,
    position: Point<Pixels>,
) -> Option<&'a InlineFootnoteHit> {
    if input.is_source_raw_mode()
        || input.display_text().is_empty()
        || lines.is_empty()
        || position.y < bounds.top()
        || position.y >= bounds.bottom()
    {
        return None;
    }

    let text = input.display_text();
    let align = input.text_align();

    for span in input.inline_spans() {
        let Some(footnote) = span.footnote.as_ref() else {
            continue;
        };
        if span.range.is_empty() {
            continue;
        }

        for footnote_bounds in
            range_segment_bounds(lines, bounds, line_height, text, span.range.clone(), align)
        {
            if point_inside_bounds(footnote_bounds, position) {
                return Some(footnote);
            }
        }
    }

    None
}

