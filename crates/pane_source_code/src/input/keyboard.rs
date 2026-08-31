//! Rich keyboard input handling for SourceCode editor.

use gpui::*;
use core_contracts::PaneHost;

use crate::state::SourceCodeState;

/// Handles a key-down event against the SourceCodeState. Returns true if consumed.
pub fn handle_key_down(
    state: &mut SourceCodeState,
    _pane_id: core_contracts::PaneId,
    event: &KeyDownEvent,
    _window: &mut Window,
    cx: &mut App,
    _host: &dyn PaneHost,
) -> bool {
    let key = event.keystroke.key.as_str();
    let ctrl = event.keystroke.modifiers.control || event.keystroke.modifiers.platform;
    let shift = event.keystroke.modifiers.shift;
    let alt = event.keystroke.modifiers.alt;

    if ctrl && !alt {
        match key {
            "a" | "A" => {
                state.select_all();
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
                    return true;
                }
            }
            "v" | "V" => {
                if let Some(clipboard) = cx.read_from_clipboard()
                    && let Some(text) = clipboard.text()
                {
                    state.insert_text(&text);
                    return true;
                }
            }
            "z" | "Z" | "y" | "Y" => {
                // Let GPUI action handlers handle Undo/Redo without re-entrant lease
                return false;
            }
            "home" => {
                state.move_to(0, shift);
                return true;
            }
            "end" => {
                let len = state.text.len();
                state.move_to(len, shift);
                return true;
            }
            "backspace" => {
                state.delete_word_backward();
                return true;
            }
            "delete" => {
                state.delete_word_forward();
                return true;
            }
            "d" | "D" => {
                state.duplicate_line();
                return true;
            }
            "k" | "K" => {
                if shift {
                    state.delete_line();
                    return true;
                }
            }
            "[" => {
                state.outdent();
                return true;
            }
            "]" => {
                state.indent();
                return true;
            }
            _ => {}
        }
    }

    if alt && ctrl {
        match key {
            "up" | "arrowup" => {
                state.add_cursor_above();
                return true;
            }
            "down" | "arrowdown" => {
                state.add_cursor_below();
                return true;
            }
            _ => {}
        }
    }

    match key {
        "backspace" => {
            state.delete_backward();
            true
        }
        "delete" => {
            state.delete_forward();
            true
        }
        "enter" => {
            state.insert_newline_with_auto_indent();
            true
        }
        "tab" => {
            if shift {
                state.outdent();
            } else {
                state.indent();
            }
            true
        }
        "space" => {
            state.insert_text(" ");
            true
        }
        "left" | "arrowleft" => {
            state.move_left(shift, ctrl);
            true
        }
        "right" | "arrowright" => {
            state.move_right(shift, ctrl);
            true
        }
        "up" | "arrowup" => {
            state.move_up(shift);
            true
        }
        "down" | "arrowdown" => {
            state.move_down(shift);
            true
        }
        "home" => {
            state.move_to_line_start(shift);
            true
        }
        "end" => {
            state.move_to_line_end(shift);
            true
        }
        "pageup" => {
            for _ in 0..10 {
                state.move_up(shift);
            }
            true
        }
        "pagedown" => {
            for _ in 0..10 {
                state.move_down(shift);
            }
            true
        }
        "escape" => {
            if state.selections.count() > 1 {
                let head = state.selections.primary().head;
                state.selections.set_single_point(head);
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
                        return true;
                    }
                } else if !key.starts_with("arrow") && !key.starts_with("f") {
                    state.insert_text(key);
                    return true;
                }
            }
            false
        }
    }
}
