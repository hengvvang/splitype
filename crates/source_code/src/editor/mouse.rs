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
        let gutter_width = self.gutter_width_px(cx);

        let bounds = self.last_bounds();
        let rel_y = f32::from(position.y - bounds.origin.y - px(padding));
        let display_row = (rel_y / line_height).floor().max(0.0) as u32;

        let frame = self
            .frame_rows
            .binary_search_by(|frame| frame.display_row.cmp(&display_row))
            .ok()
            .map(|idx| &self.frame_rows[idx])?;

        let segment = self.text.slice_owned(frame.range.clone());
        let rel_x = f32::from(position.x - bounds.origin.x - px(gutter_width + 12.0));

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
    /// toggle the fold instead.
    pub fn handle_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        // Fold-chevron zone: the leftmost gutter column toggles the fold
        // headed by the row under the pointer.
        if let Some(row) = self.fold_marker_row_at(event.position, cx) {
            self.toggle_fold_at_row(row);
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

    /// Mouse-move while dragging: extend the selection. Returns whether
    /// the move changed editor state (only while a drag is active), so the
    /// host can skip re-rendering for plain pointer motion.
    pub fn handle_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.is_dragging {
            return false;
        }
        if let Some(offset) = self.hit_test(event.position, window, cx) {
            self.update_drag(offset);
            cx.notify();
            return true;
        }
        false
    }

    /// Mouse-up ends the drag session.
    pub fn handle_mouse_up(&mut self, _event: &MouseUpEvent, cx: &mut Context<Self>) {
        self.end_drag();
        cx.notify();
    }

    /// The buffer row whose fold chevron sits under `position`, if any.
    /// Returns the row when it is folded or foldable, within the leftmost
    /// gutter column, on a rendered frame row.
    fn fold_marker_row_at(&self, position: Point<Pixels>, cx: &App) -> Option<u32> {
        let theme = cx.global::<ThemeManager>().current_arc();
        let line_height =
            (theme.typography.code_size.max(12.0) * theme.typography.text_line_height).round();
        let padding = theme.dimensions.editor_padding;
        let gutter_width = self.gutter_width_px(cx);
        let bounds = self.last_bounds();
        let zone_right = bounds.left() + px(gutter_width.min(18.0));
        if position.x < bounds.left() || position.x >= zone_right {
            return None;
        }
        let rel_y = f32::from(position.y - bounds.origin.y - px(padding));
        let display_row = (rel_y / line_height).floor().max(0.0) as u32;
        let frame = self
            .frame_rows
            .binary_search_by(|frame| frame.display_row.cmp(&display_row))
            .ok()?;
        let buffer_row = self.frame_rows[frame].buffer_row;
        let folded = self.folds.is_folded(buffer_row);
        let foldable = !folded && self.foldable_at(buffer_row).is_some();
        (folded || foldable).then_some(buffer_row)
    }
}
