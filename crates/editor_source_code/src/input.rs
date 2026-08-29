//! Source Code pane input handling — pure state transitions.
//!
//! These functions operate only on [`SourceCodeState`]; coordination-layer
//! actions (document sync, undo/redo, notify) go through
//! [`editor_model::PaneHost`], which the coordinating crate implements. The app
//! routes events here and forwards the pane's state.

use gpui::*;

use editor_model::PaneHost;
use theme::{ThemeManager, TypographyScope, TypographyStore};

use crate::state::SourceCodeState;

/// Dispatches a key-down event against the Source pane state. Returns true
/// when the key was consumed.
pub fn handle_key_down(
    state: &mut SourceCodeState,
    pane_id: editor_model::PaneId,
    event: &KeyDownEvent,
    window: &mut Window,
    cx: &mut App,
    host: &dyn PaneHost,
) -> bool {
    let key = event.keystroke.key.as_str();
    let ctrl = event.keystroke.modifiers.control || event.keystroke.modifiers.platform;
    let shift = event.keystroke.modifiers.shift;
    let alt = event.keystroke.modifiers.alt;

    if ctrl && !alt {
        match key {
            "a" | "A" => {
                state.select_all();
                host.notify(cx);
                return true;
            }
            "c" | "C" => {
                if let Some(selected) = state.selected_text() {
                    cx.write_to_clipboard(ClipboardItem::new_string(selected.to_string()));
                    return true;
                }
            }
            "x" | "X" => {
                let mut text_to_copy = None;
                if let Some(selected) = state.selected_text() {
                    text_to_copy = Some(selected.to_string());
                    state.delete_backward();
                }
                if let Some(text) = text_to_copy {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                    host.sync_source_edit(pane_id, cx);
                    return true;
                }
            }
            "v" | "V" => {
                if let Some(clipboard) = cx.read_from_clipboard()
                    && let Some(text) = clipboard.text()
                {
                    state.insert_text(&text);
                    host.sync_source_edit(pane_id, cx);
                    return true;
                }
            }
            "z" | "Z" => {
                if shift {
                    host.redo(window, cx);
                } else {
                    host.undo(window, cx);
                }
                return true;
            }
            "y" | "Y" => {
                host.redo(window, cx);
                return true;
            }
            "home" => {
                state.move_to(0, shift);
                host.notify(cx);
                return true;
            }
            "end" => {
                let len = state.text.len();
                state.move_to(len, shift);
                host.notify(cx);
                return true;
            }
            _ => {}
        }
    }

    match key {
        "backspace" => {
            state.delete_backward();
            host.sync_source_edit(pane_id, cx);
            true
        }
        "delete" => {
            state.delete_forward();
            host.sync_source_edit(pane_id, cx);
            true
        }
        "enter" => {
            state.insert_text("\n");
            host.sync_source_edit(pane_id, cx);
            true
        }
        "tab" => {
            state.insert_text("    ");
            host.sync_source_edit(pane_id, cx);
            true
        }
        "space" => {
            state.insert_text(" ");
            host.sync_source_edit(pane_id, cx);
            true
        }
        "left" | "arrowleft" => {
            state.move_left(shift);
            host.notify(cx);
            true
        }
        "right" | "arrowright" => {
            state.move_right(shift);
            host.notify(cx);
            true
        }
        "up" | "arrowup" => {
            state.move_up(shift);
            host.notify(cx);
            true
        }
        "down" | "arrowdown" => {
            state.move_down(shift);
            host.notify(cx);
            true
        }
        "home" => {
            state.move_to_line_start(shift);
            host.notify(cx);
            true
        }
        "end" => {
            state.move_to_line_end(shift);
            host.notify(cx);
            true
        }
        "pageup" => {
            for _ in 0..10 {
                state.move_up(shift);
            }
            host.notify(cx);
            true
        }
        "pagedown" => {
            for _ in 0..10 {
                state.move_down(shift);
            }
            host.notify(cx);
            true
        }
        _ => {
            if !ctrl && !alt && !key.is_empty() {
                let mut chars = key.chars();
                if let Some(first) = chars.next() {
                    if chars.next().is_none() && !first.is_control() {
                        state.insert_text(key);
                        host.sync_source_edit(pane_id, cx);
                        return true;
                    }
                } else if !key.starts_with("arrow") && !key.starts_with("f") {
                    state.insert_text(key);
                    host.sync_source_edit(pane_id, cx);
                    return true;
                }
            }
            false
        }
    }
}

/// Maps a pointer position inside the pane bounds to (line, byte-column).
fn hit_test(
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

    let last_bounds = state.last_bounds;
    let total_lines = state.line_count();

    let line_digits = total_lines.to_string().len();
    let gutter_width = (line_digits as f32 * (font_size * 0.6) + 24.0).max(36.0);

    let bounds_origin = last_bounds.map(|b| b.origin).unwrap_or(point(px(0.0), px(0.0)));
    let rel_y = f32::from(position.y - bounds_origin.y) - padding;
    let line_idx = (rel_y / line_height).floor().max(0.0) as usize;
    let line_idx = line_idx.min(total_lines.saturating_sub(1));

    let line_str = state.line_str(line_idx);
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

    (line_idx, col)
}

/// Mouse-down on the Source pane: position the caret, start a drag
/// session, or select line/word on multi-clicks.
pub fn handle_mouse_down(
    state: &mut SourceCodeState,
    event: &MouseDownEvent,
    window: &Window,
    cx: &App,
) {
    let shift = event.modifiers.shift;
    let click_count = event.click_count;

    let (line_idx, col) = hit_test(state, event.position, window, cx);
    let offset = state.offset_at_line_col(line_idx, col);

    if click_count >= 3 {
        state.select_line_at(line_idx);
    } else if click_count == 2 {
        state.select_word_at(offset);
    } else if shift {
        state.move_to(offset, true);
    } else {
        state.start_drag(offset);
    }
}

/// Mouse-move while dragging on the Source pane: extend the selection.
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
