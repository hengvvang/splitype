//! Mouse interaction: hit-testing through the last frame's row layout,
//! click-to-place, drag selection, and multi-cursor via Alt+Click.

use gpui::{
    App, Context, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, SharedString,
    TextRun, Window, px,
};
use theme::{ThemeManager, TypographyScope, TypographyStore};

use crate::editor::SourceCodeEditor;

impl SourceCodeEditor {
    /// Maps a pointer position inside the pane bounds to a byte offset in
    /// the document, or `None` when the row under the pointer was not part
    /// of the last rendered frame (or the editor was never laid out).
    pub fn hit_test(&self, position: Point<Pixels>, window: &Window, cx: &App) -> Option<usize> {
        let theme = cx.global::<ThemeManager>().current_arc();
        let font_size = theme.typography.code_size.max(12.0);
        let line_height = (font_size * theme.typography.text_line_height).round();
        let padding = theme.dimensions.editor_padding;
        let text_offset = self.text_offset_px(cx);

        let bounds = self.last_bounds();
        let rel_y = f32::from(position.y - bounds.origin.y - px(padding));
        let display_row = (rel_y / line_height).floor().max(0.0) as u32;

        let frame = self
            .frame_rows
            .binary_search_by(|frame| frame.display_row.cmp(&display_row))
            .ok()
            .map(|idx| &self.frame_rows[idx])?;

        let segment = self.text.slice_owned(frame.range.clone());
        let rel_x = f32::from(position.x - bounds.origin.x - px(text_offset));

        let col_in_segment = if rel_x <= 0.0 || segment.is_empty() {
            0
        } else {
            let segment_len = segment.len();
            let font = TypographyStore::default_font(TypographyScope::Code);
            let shaped = window.text_system().shape_line(
                SharedString::new(segment),
                px(font_size),
                &[TextRun {
                    len: segment_len,
                    font,
                    color: theme.colors.text_default,
                    ..Default::default()
                }],
                None,
            );
            shaped.index_for_x(px(rel_x)).unwrap_or(segment_len)
        };

        Some(frame.range.start + col_in_segment)
    }

    /// Mouse-down: place caret, start a drag, add a cursor (Alt), or
    /// select word/line by click count. Clicks on a gutter fold chevron
    /// toggle the fold; clicks on a line number select the entire line.
    pub fn handle_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        // Fold-chevron zone: right of line numbers, toggles the fold
        // headed by the row under the pointer.
        if let Some(row) = self.fold_marker_row_at(event.position, cx) {
            self.toggle_fold_at_row(row);
            cx.notify();
            return;
        }

        // Line-number click: selects the entire line (or line range if shift-clicked)
        // and starts line-drag selection.
        if let Some(buffer_row) = self.line_number_row_at(event.position, cx) {
            let row = buffer_row as usize;
            if event.modifiers.shift {
                let (anchor_row, _) = self.point_of(self.selections.primary().anchor);
                self.select_lines_range(anchor_row, row);
            } else {
                self.is_dragging = true;
                self.line_drag_anchor = Some(row);
                self.select_line_at(row);
            }
            cx.notify();
            return;
        }

        let shift = event.modifiers.shift;
        let alt = event.modifiers.alt;
        let click_count = event.click_count;

        let Some(offset) = self.hit_test(event.position, window, cx) else {
            return;
        };

        if click_count >= 3 {
            let row = self.point_of(offset).0;
            self.select_line_at(row);
        } else if click_count == 2 {
            self.select_word_at(offset);
        } else if alt {
            self.add_cursor_at(offset);
        } else if shift {
            self.move_to(offset, true);
        } else {
            self.start_drag(offset);
        }
        cx.notify();
    }

    /// Mouse-move: updates drag selection and tracks gutter/fold hover states.
    /// Returns whether the move changed editor state so the host can skip
    /// re-rendering when the pointer motion is uninteresting.
    pub fn handle_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let mut state_changed = false;

        if self.is_dragging {
            if let Some(anchor_row) = self.line_drag_anchor {
                let current_row = self.buffer_row_at_y(event.position.y, cx);
                self.select_lines_range(anchor_row, current_row);
                state_changed = true;
            } else if let Some(offset) = self.hit_test(event.position, window, cx) {
                self.update_drag(offset);
                state_changed = true;
            }
        }

        let bounds = self.last_bounds();
        let in_bounds = bounds.contains(&event.position);
        let gutter_layout = self.gutter_layout(cx);
        let gutter_width = gutter_layout.gutter_width();
        let in_gutter = in_bounds
            && event.position.x >= bounds.left()
            && event.position.x < bounds.left() + px(gutter_width);

        if in_gutter != self.gutter_hovered {
            self.gutter_hovered = in_gutter;
            state_changed = true;
        }

        let hovered_fold = if in_gutter {
            self.fold_marker_row_at(event.position, cx)
        } else {
            None
        };

        if hovered_fold != self.hovered_fold_row {
            self.hovered_fold_row = hovered_fold;
            state_changed = true;
        }

        let hovered_line_num = if in_gutter {
            self.line_number_row_at(event.position, cx)
        } else {
            None
        };

        if hovered_line_num != self.hovered_line_number_row {
            self.hovered_line_number_row = hovered_line_num;
            state_changed = true;
        }

        if state_changed {
            cx.notify();
        }

        state_changed
    }

    /// Mouse-up ends the drag session.
    pub fn handle_mouse_up(&mut self, _event: &MouseUpEvent, cx: &mut Context<Self>) {
        self.end_drag();
        cx.notify();
    }

    /// The buffer row whose line number sits under `position`, if any.
    fn line_number_row_at(&self, position: Point<Pixels>, cx: &App) -> Option<u32> {
        if !self.settings.line_numbers {
            return None;
        }
        let bounds = self.last_bounds();
        let gutter_layout = self.gutter_layout(cx);
        let in_line_numbers = position.x >= bounds.left()
            && position.x < bounds.left() + px(gutter_layout.fold_area_left());
        if !in_line_numbers {
            return None;
        }
        let theme = cx.global::<ThemeManager>().current_arc();
        let line_height =
            (theme.typography.code_size.max(12.0) * theme.typography.text_line_height).round();
        let padding = theme.dimensions.editor_padding;
        let rel_y = f32::from(position.y - bounds.origin.y - px(padding));
        let display_row = (rel_y / line_height).floor().max(0.0) as u32;
        let frame_idx = self
            .frame_rows
            .binary_search_by(|frame| frame.display_row.cmp(&display_row))
            .ok()?;
        let frame = &self.frame_rows[frame_idx];
        Some(frame.buffer_row)
    }

    /// Resolves the buffer row at vertical coordinate `y`, clamping to visible and buffer limits.
    fn buffer_row_at_y(&self, y: Pixels, cx: &App) -> usize {
        if self.frame_rows.is_empty() {
            return 0;
        }
        let theme = cx.global::<ThemeManager>().current_arc();
        let line_height =
            (theme.typography.code_size.max(12.0) * theme.typography.text_line_height).round();
        let padding = theme.dimensions.editor_padding;
        let bounds = self.last_bounds();
        let rel_y = f32::from(y - bounds.origin.y - px(padding));
        let display_row = (rel_y / line_height).floor().max(0.0) as u32;

        let row = match self
            .frame_rows
            .binary_search_by(|f| f.display_row.cmp(&display_row))
        {
            Ok(idx) => self.frame_rows[idx].buffer_row as usize,
            Err(idx) => {
                if idx == 0 {
                    self.frame_rows[0].buffer_row as usize
                } else {
                    let clamped = idx.min(self.frame_rows.len()) - 1;
                    self.frame_rows[clamped].buffer_row as usize
                }
            }
        };
        row.min(self.line_count().saturating_sub(1))
    }

    /// The buffer row whose fold chevron sits under `position`, if any.
    /// Returns the row when it is folded or foldable, within the fold area
    /// column (to the right of line numbers), on the first visual row of that line.
    fn fold_marker_row_at(&self, position: Point<Pixels>, cx: &App) -> Option<u32> {
        let theme = cx.global::<ThemeManager>().current_arc();
        let line_height =
            (theme.typography.code_size.max(12.0) * theme.typography.text_line_height).round();
        let padding = theme.dimensions.editor_padding;
        let gutter_layout = self.gutter_layout(cx);
        let bounds = self.last_bounds();

        let fold_left = bounds.left() + px(gutter_layout.fold_area_left());
        let fold_right = bounds.left() + px(gutter_layout.gutter_width());
        if position.x < fold_left || position.x >= fold_right {
            return None;
        }

        let rel_y = f32::from(position.y - bounds.origin.y - px(padding));
        let display_row = (rel_y / line_height).floor().max(0.0) as u32;
        let frame_idx = self
            .frame_rows
            .binary_search_by(|frame| frame.display_row.cmp(&display_row))
            .ok()?;
        let frame = &self.frame_rows[frame_idx];
        if !frame.is_first {
            return None;
        }

        let buffer_row = frame.buffer_row;
        let folded = self.folds.is_folded(buffer_row);
        let foldable = !folded && self.foldable_at(buffer_row).is_some();
        (folded || foldable).then_some(buffer_row)
    }
}
