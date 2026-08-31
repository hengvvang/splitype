//! Mouse interaction handling for SourceCode editor.

use gpui::*;
use theme::{ThemeManager, TypographyScope, TypographyStore};

use crate::state::SourceCodeState;

/// Maps a pointer position inside the pane bounds to (buffer_row, byte_col_within_line).
pub fn hit_test(
    state: &SourceCodeState,
    position: Point<Pixels>,
    window: &Window,
    cx: &App,
) -> (usize, usize) {
    let theme = cx.global::<ThemeManager>().current_arc();
    let font_size = theme.typography.code_size.max(12.0);
    let line_height = (font_size * theme.typography.text_line_height).round();
    let padding = theme.dimensions.editor_padding;
    let font = TypographyStore::default_font(TypographyScope::Code);

    let last_bounds = *state.last_bounds.lock().unwrap();
    let total_lines = state.line_count();

    let gutter_width = state.gutter_layout(font_size).width();

    let bounds_origin = last_bounds
        .map(|b| b.origin)
        .unwrap_or(point(px(0.0), px(0.0)));
    let rel_y = f32::from(position.y - bounds_origin.y) - padding;
    let visible_row = (rel_y / line_height).floor().max(0.0) as u32;

    let buffer_row = state
        .fold_map
        .visible_row_to_buffer_row(visible_row, total_lines as u32) as usize;
    let buffer_row = buffer_row.min(total_lines.saturating_sub(1));

    let line_str = state.line_str(buffer_row);
    let rel_x = f32::from(position.x - bounds_origin.x) - gutter_width - 12.0;

    let col = if rel_x <= 0.0 || line_str.is_empty() {
        0
    } else {
        let shaped = window.text_system().shape_line(
            SharedString::new(line_str),
            px(font_size),
            &[TextRun {
                len: line_str.len(),
                font,
                color: theme.colors.text_default,
                ..Default::default()
            }],
            None,
        );
        shaped.index_for_x(px(rel_x)).unwrap_or(line_str.len())
    };

    (buffer_row, col)
}

/// Mouse-down on the Source pane: position caret, start drag, multi-cursor on Alt, or select line/word.
pub fn handle_mouse_down(
    state: &mut SourceCodeState,
    event: &MouseDownEvent,
    window: &Window,
    cx: &App,
) {
    let shift = event.modifiers.shift;
    let alt = event.modifiers.alt;
    let click_count = event.click_count;

    let (line_idx, col) = hit_test(state, event.position, window, cx);
    let offset = state.offset_at_line_col(line_idx, col);

    if click_count >= 3 {
        state.select_line_at(line_idx);
    } else if click_count == 2 {
        state.select_word_at(offset);
    } else if alt {
        state.add_cursor_at(offset);
    } else if shift {
        state.move_to(offset, true);
    } else {
        state.start_drag(offset);
    }
}

/// Mouse-move while dragging on the Source pane: extend selection.
pub fn handle_mouse_move(
    state: &mut SourceCodeState,
    event: &MouseMoveEvent,
    window: &Window,
    cx: &App,
) {
    if !state.is_dragging {
        return;
    }
    let (line_idx, col) = hit_test(state, event.position, window, cx);
    let offset = state.offset_at_line_col(line_idx, col);
    state.update_drag(offset);
}

/// Mouse-up ends the drag session.
pub fn handle_mouse_up(state: &mut SourceCodeState) {
    state.end_drag();
}
