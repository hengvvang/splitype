//! Keyboard input handling for the source code editor.
//!
//! Clipboard, select-all, and undo/redo keys are intentionally NOT handled
//! here: they are bound to the editor's unified actions in the
//! `EditorContent` key context and must not be double-handled by the pane.

use gpui::{Context, KeyDownEvent, Window};

use crate::editor::SourceCodeEditor;

impl SourceCodeEditor {
    /// Handles a key-down event against the editor. Returns true when
    /// consumed. Text-changing keys commit through the pane host.
    pub fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let key = event.keystroke.key.as_str();
        let ctrl = event.keystroke.modifiers.control || event.keystroke.modifiers.platform;
        let shift = event.keystroke.modifiers.shift;
        let alt = event.keystroke.modifiers.alt;

        if ctrl && !alt {
            match key {
                // Clipboard, select-all, and undo/redo are handled by the
                // editor's unified actions (bound in the EditorContent key
                // context); the pane must not double-handle them.
                "a" | "A" | "c" | "C" | "x" | "X" | "v" | "V" | "z" | "Z" | "y" | "Y" => {
                    return false;
                }
                "home" => {
                    self.move_to(0, shift);
                    self.scroll_cursor_into_view(window, cx);
                    return true;
                }
                "end" => {
                    let len = self.text.len();
                    self.move_to(len, shift);
                    self.scroll_cursor_into_view(window, cx);
                    return true;
                }
                "backspace" => {
                    self.delete_word_backward(cx);
                    return true;
                }
                "delete" => {
                    self.delete_word_forward(cx);
                    return true;
                }
                "d" | "D" => {
                    self.duplicate_line(cx);
                    return true;
                }
                "k" | "K" => {
                    if shift {
                        self.delete_line(cx);
                        return true;
                    }
                }
                "[" => {
                    self.outdent(cx);
                    return true;
                }
                "]" => {
                    self.indent(cx);
                    return true;
                }
                _ => {}
            }
        }

        if alt && ctrl {
            match key {
                "up" | "arrowup" => {
                    self.add_cursor_above();
                    cx.notify();
                    return true;
                }
                "down" | "arrowdown" => {
                    self.add_cursor_below();
                    cx.notify();
                    return true;
                }
                "[" => {
                    self.toggle_fold_at_cursor();
                    cx.notify();
                    return true;
                }
                "]" => {
                    self.unfold_all();
                    cx.notify();
                    return true;
                }
                _ => {}
            }
        }

        match key {
            "backspace" => {
                self.delete_backward(cx);
                self.scroll_cursor_into_view(window, cx);
                true
            }
            "delete" => {
                self.delete_forward(cx);
                self.scroll_cursor_into_view(window, cx);
                true
            }
            "enter" => {
                self.insert_newline_with_auto_indent(cx);
                self.scroll_cursor_into_view(window, cx);
                true
            }
            "tab" => {
                if shift {
                    self.outdent(cx);
                } else {
                    self.indent(cx);
                }
                self.scroll_cursor_into_view(window, cx);
                true
            }
            "space" => {
                self.insert_text_commit(" ", cx);
                self.scroll_cursor_into_view(window, cx);
                true
            }
            "left" | "arrowleft" => {
                self.move_left(shift, ctrl);
                self.scroll_cursor_into_view(window, cx);
                cx.notify();
                true
            }
            "right" | "arrowright" => {
                self.move_right(shift, ctrl);
                self.scroll_cursor_into_view(window, cx);
                cx.notify();
                true
            }
            "up" | "arrowup" => {
                self.move_up(shift);
                self.scroll_cursor_into_view(window, cx);
                cx.notify();
                true
            }
            "down" | "arrowdown" => {
                self.move_down(shift);
                self.scroll_cursor_into_view(window, cx);
                cx.notify();
                true
            }
            "home" => {
                self.move_to_line_start(shift);
                self.scroll_cursor_into_view(window, cx);
                cx.notify();
                true
            }
            "end" => {
                self.move_to_line_end(shift);
                self.scroll_cursor_into_view(window, cx);
                cx.notify();
                true
            }
            "pageup" => {
                for _ in 0..10 {
                    self.move_up(shift);
                }
                self.scroll_cursor_into_view(window, cx);
                cx.notify();
                true
            }
            "pagedown" => {
                for _ in 0..10 {
                    self.move_down(shift);
                }
                self.scroll_cursor_into_view(window, cx);
                cx.notify();
                true
            }
            "escape" => {
                if self.selections.count() > 1 {
                    let head = self.selections.primary().head;
                    self.selections.set_single_point(head);
                    cx.notify();
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
                            self.insert_text_commit(key, cx);
                            self.scroll_cursor_into_view(window, cx);
                            return true;
                        }
                    }
                }
                false
            }
        }
    }
}
