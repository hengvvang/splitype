//! Rich keyboard input handling for SourceCode editor.

use gpui::*;
use editor_model::PaneHost;

use crate::state::SourceCodeState;

/// Handles a key-down event against the SourceCodeState. Returns true if consumed.
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
            "backspace" => {
                state.delete_word_backward();
                host.sync_source_edit(pane_id, cx);
                return true;
            }
            "delete" => {
                state.delete_word_forward();
                host.sync_source_edit(pane_id, cx);
                return true;
            }
            "d" | "D" => {
                state.duplicate_line();
                host.sync_source_edit(pane_id, cx);
                return true;
            }
            "k" | "K" => {
                if shift {
                    state.delete_line();
                    host.sync_source_edit(pane_id, cx);
                    return true;
                }
            }
            "[" => {
                state.outdent();
                host.sync_source_edit(pane_id, cx);
                return true;
            }
            "]" => {
                state.indent();
                host.sync_source_edit(pane_id, cx);
                return true;
            }
            _ => {}
        }
    }

    if alt && ctrl {
        match key {
            "up" | "arrowup" => {
                state.add_cursor_above();
                host.notify(cx);
                return true;
            }
            "down" | "arrowdown" => {
                state.add_cursor_below();
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
            state.insert_newline_with_auto_indent();
            host.sync_source_edit(pane_id, cx);
            true
        }
        "tab" => {
            if shift {
                state.outdent();
            } else {
                state.indent();
            }
            host.sync_source_edit(pane_id, cx);
            true
        }
        "space" => {
            state.insert_text(" ");
            host.sync_source_edit(pane_id, cx);
            true
        }
        "left" | "arrowleft" => {
            state.move_left(shift, ctrl);
            host.notify(cx);
            true
        }
        "right" | "arrowright" => {
            state.move_right(shift, ctrl);
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
        "escape" => {
            if state.selections.count() > 1 {
                let head = state.selections.primary().head;
                state.selections.set_single_point(head);
                host.notify(cx);
                true
            } else {
                false
            }
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
