//! Block cursor navigation, vertical motion, word boundary detection, and selections.

use std::ops::Range;
use std::time::{Duration, Instant};

use gpui::*;

use super::Block;
use super::state::CollapsedCaretAffinity;
use crate::editor::geometry::text_layout as element;
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};

impl Block {
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
        let changed = self.is_selecting || self.code_toolbar.picker.is_selecting;
        self.is_selecting = false;
        self.code_toolbar.picker.is_selecting = false;
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
        markdown::inline::offsets::ImeConverter::utf8_range_to_utf16_in(
            self.display_text(),
            range,
        )
    }

    pub(crate) fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        markdown::inline::offsets::ImeConverter::utf16_range_to_utf8_in(
            self.display_text(),
            range_utf16,
        )
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
}

pub(crate) fn normalize_code_language_input(text: &str) -> String {
    text.replace("\r\n", " ")
        .replace(['\r', '\n'], " ")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {}
