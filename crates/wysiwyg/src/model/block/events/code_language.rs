//! Code-language picker handlers on a focused block: the transient
//! language field above a code block, its editing, and dismissal.

use gpui::*;

use crate::model::block::Block;
use crate::model::protocol::BlockEvent;
use crate::pane::actions::{
    Delete, DeleteBackward, End, FocusNext, FocusPrevious, Home, IndentBlock, MoveLeft, MoveRight,
    Newline, OutdentBlock, SelectAll, SelectLeft, SelectRight,
};
use platform_contracts::actions::{Copy, Cut, DismissTransientUi, Paste};
use syntax_highlighter::language::code_language_options_matching;
impl Block {
    pub fn on_code_block_hover(
        &mut self,
        hovered: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let should_show = *hovered || self.code_toolbar.picker.is_open;
        if self.code_toolbar.is_hovered != should_show {
            self.code_toolbar.is_hovered = should_show;
            cx.notify();
        }
    }

    pub fn on_code_language_picker_toggle(
        &mut self,
        _: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        if self.code_toolbar.picker.is_open {
            self.code_toolbar.picker.close();
            self.code_toolbar.is_hovered = false;
            self.focus_handle.focus(window, cx);
        } else {
            self.code_toolbar.picker.open();
            self.code_toolbar.is_hovered = true;
            self.code_language_focus_handle.focus(window, cx);
        }
        cx.notify();
    }

    pub fn on_code_copy_button_mouse_down(
        &mut self,
        _: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        cx.write_to_clipboard(ClipboardItem::new_string(self.data.text.plain_text()));
    }

    pub fn on_code_language_newline(
        &mut self,
        _: &Newline,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        if self.code_toolbar.picker.is_open {
            let value = code_language_options_matching(&self.code_toolbar.picker.query)
                .first()
                .map(|option| option.value);
            if let Some(value) = value {
                self.choose_code_language(value, cx);
            } else {
                self.code_toolbar.picker.close();
            }
        }
        self.focus_handle.focus(window, cx);
        cx.notify();
    }

    pub fn on_code_language_dismiss(
        &mut self,
        _: &DismissTransientUi,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        self.code_toolbar.picker.close();
        self.focus_handle.focus(window, cx);
        cx.notify();
    }

    pub fn on_code_language_delete_backward(
        &mut self,
        _: &DeleteBackward,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        if self.code_toolbar.picker.selected_range.is_empty() {
            let previous =
                self.previous_code_language_boundary(self.code_toolbar.picker.cursor_offset());
            self.select_code_language_to(previous, cx);
        }
        self.replace_code_language_text_in_range(
            self.code_toolbar.picker.selected_range.clone(),
            "",
            None,
            false,
            cx,
        );
    }

    pub fn on_code_language_delete(
        &mut self,
        _: &Delete,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        if self.code_toolbar.picker.selected_range.is_empty() {
            let next = self.next_code_language_boundary(self.code_toolbar.picker.cursor_offset());
            self.select_code_language_to(next, cx);
        }
        self.replace_code_language_text_in_range(
            self.code_toolbar.picker.selected_range.clone(),
            "",
            None,
            false,
            cx,
        );
    }

    pub fn on_code_language_move_left(
        &mut self,
        _: &MoveLeft,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        if self.code_toolbar.picker.selected_range.is_empty() {
            self.move_code_language_to(
                self.previous_code_language_boundary(self.code_toolbar.picker.cursor_offset()),
                cx,
            );
        } else {
            self.move_code_language_to(self.code_toolbar.picker.selected_range.start, cx);
        }
    }

    pub fn on_code_language_move_right(
        &mut self,
        _: &MoveRight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        if self.code_toolbar.picker.selected_range.is_empty() {
            self.move_code_language_to(
                self.next_code_language_boundary(self.code_toolbar.picker.cursor_offset()),
                cx,
            );
        } else {
            self.move_code_language_to(self.code_toolbar.picker.selected_range.end, cx);
        }
    }

    pub fn on_code_language_home(&mut self, _: &Home, window: &mut Window, cx: &mut Context<Self>) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        self.move_code_language_to(0, cx);
    }

    pub fn on_code_language_end(&mut self, _: &End, window: &mut Window, cx: &mut Context<Self>) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        self.move_code_language_to(self.code_language_input_text().len(), cx);
    }

    pub fn on_code_language_select_left(
        &mut self,
        _: &SelectLeft,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        self.select_code_language_to(
            self.previous_code_language_boundary(self.code_toolbar.picker.cursor_offset()),
            cx,
        );
    }

    pub fn on_code_language_select_right(
        &mut self,
        _: &SelectRight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        self.select_code_language_to(
            self.next_code_language_boundary(self.code_toolbar.picker.cursor_offset()),
            cx,
        );
    }

    pub fn on_code_language_select_all(
        &mut self,
        _: &SelectAll,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        self.move_code_language_to(0, cx);
        self.select_code_language_to(self.code_language_input_text().len(), cx);
    }

    pub fn on_code_language_copy(&mut self, _: &Copy, window: &mut Window, cx: &mut Context<Self>) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        if !self.code_toolbar.picker.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.code_language_input_text()[self.code_toolbar.picker.selected_range.clone()]
                    .to_string(),
            ));
        }
    }

    pub fn on_code_language_cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        if !self.code_toolbar.picker.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.code_language_input_text()[self.code_toolbar.picker.selected_range.clone()]
                    .to_string(),
            ));
            self.replace_code_language_text_in_range(
                self.code_toolbar.picker.selected_range.clone(),
                "",
                None,
                false,
                cx,
            );
        }
    }

    pub fn on_code_language_paste(
        &mut self,
        _: &Paste,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_code_language_text_in_range(
                self.code_toolbar.picker.selected_range.clone(),
                &text,
                None,
                false,
                cx,
            );
        }
    }

    pub fn on_code_language_focus_content(
        &mut self,
        _: &FocusPrevious,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        self.code_toolbar.picker.close();
        self.focus_handle.focus(window, cx);
        cx.notify();
    }

    pub fn on_code_language_focus_next(
        &mut self,
        _: &FocusNext,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        if self.code_toolbar.picker.is_open {
            return;
        }
        // Down from the language field leaves the code block: the editor focuses
        // the block below, creating a trailing paragraph first when the code
        // block is the last block. Enter does not exit (see on_code_language_newline).
        cx.emit(BlockEvent::RequestFocusNext { preferred_x: None });
    }

    pub fn on_code_language_indent(
        &mut self,
        _: &IndentBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.code_language_focus_handle.is_focused(window) {
            cx.stop_propagation();
        }
    }

    pub fn on_code_language_outdent(
        &mut self,
        _: &OutdentBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.code_language_focus_handle.is_focused(window) {
            cx.stop_propagation();
        }
    }

    pub fn on_code_language_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        self.code_toolbar.picker.is_selecting = true;
        self.code_language_focus_handle.focus(window, cx);
        let offset = self.code_language_index_for_mouse_position(event.position);
        if event.modifiers.shift {
            self.select_code_language_to(offset, cx);
        } else {
            self.move_code_language_to(offset, cx);
        }
    }

    pub fn on_code_language_mouse_up(
        &mut self,
        _: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        self.code_toolbar.picker.is_selecting = false;
    }

    pub fn on_code_language_mouse_up_out(
        &mut self,
        _: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // GPUI dispatches mouse_up_out during capture; do not stop propagation
        // here, or controls under the pointer cannot synthesize on_click.
        if self.code_toolbar.picker.is_selecting {
            self.code_toolbar.picker.is_selecting = false;
            cx.notify();
        }
    }

    pub fn on_code_language_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.code_toolbar.picker.is_selecting {
            // A stale selecting flag can survive a missed mouse-up. Only extend
            // the selection while the platform still reports an active drag.
            if !event.dragging() {
                self.code_toolbar.picker.is_selecting = false;
                cx.notify();
                return;
            }
            cx.stop_propagation();
            self.select_code_language_to(
                self.code_language_index_for_mouse_position(event.position),
                cx,
            );
        }
    }

    pub fn on_code_language_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let keystroke = &event.keystroke;
        let ctrl = keystroke.modifiers.control || keystroke.modifiers.platform;

        if ctrl && !keystroke.modifiers.alt {
            match keystroke.key.as_str() {
                "a" | "A" => {
                    self.code_toolbar.picker.selected_range = 0..self.code_toolbar.picker.query.len();
                    self.code_toolbar.picker.selection_reversed = false;
                    cx.notify();
                    cx.stop_propagation();
                    return;
                }
                "c" | "C" => {
                    let range = self.code_toolbar.picker.selected_range.clone();
                    if !range.is_empty() && range.end <= self.code_toolbar.picker.query.len() {
                        let text = self.code_toolbar.picker.query[range].to_string();
                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                    }
                    cx.stop_propagation();
                    return;
                }
                "v" | "V" => {
                    if let Some(clipboard) = cx.read_from_clipboard()
                        && let Some(text) = clipboard.text()
                    {
                        let sanitized = text.replace(['\r', '\n'], "");
                        self.replace_code_language_text_in_range(
                            self.code_toolbar.picker.selected_range.clone(),
                            &sanitized,
                            None,
                            true,
                            cx,
                        );
                    }
                    cx.stop_propagation();
                    return;
                }
                "x" | "X" => {
                    let range = self.code_toolbar.picker.selected_range.clone();
                    if !range.is_empty() && range.end <= self.code_toolbar.picker.query.len() {
                        let text = self.code_toolbar.picker.query[range.clone()].to_string();
                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                        self.replace_code_language_text_in_range(
                            range,
                            "",
                            None,
                            true,
                            cx,
                        );
                    }
                    cx.stop_propagation();
                    return;
                }
                _ => {}
            }
        }

        match keystroke.key.as_str() {
            "escape" => {
                self.code_toolbar.picker.close();
                self.focus_handle.focus(window, cx);
                cx.notify();
                cx.stop_propagation();
            }
            "enter" => {
                if self.code_toolbar.picker.is_open {
                    let value = code_language_options_matching(&self.code_toolbar.picker.query)
                        .first()
                        .map(|option| option.value);
                    if let Some(value) = value {
                        self.choose_code_language(value, cx);
                    } else {
                        self.code_toolbar.picker.close();
                    }
                }
                self.focus_handle.focus(window, cx);
                cx.notify();
                cx.stop_propagation();
            }
            "backspace" => {
                if self.code_toolbar.picker.selected_range.is_empty() {
                    let previous = self.previous_code_language_boundary(self.code_toolbar.picker.cursor_offset());
                    self.select_code_language_to(previous, cx);
                }
                self.replace_code_language_text_in_range(
                    self.code_toolbar.picker.selected_range.clone(),
                    "",
                    None,
                    false,
                    cx,
                );
                cx.stop_propagation();
            }
            "delete" => {
                if self.code_toolbar.picker.selected_range.is_empty() {
                    let next = self.next_code_language_boundary(self.code_toolbar.picker.cursor_offset());
                    self.select_code_language_to(next, cx);
                }
                self.replace_code_language_text_in_range(
                    self.code_toolbar.picker.selected_range.clone(),
                    "",
                    None,
                    false,
                    cx,
                );
                cx.stop_propagation();
            }
            "left" | "arrowleft" => {
                self.move_code_language_to(
                    self.previous_code_language_boundary(self.code_toolbar.picker.cursor_offset()),
                    cx,
                );
                cx.stop_propagation();
            }
            "right" | "arrowright" => {
                self.move_code_language_to(
                    self.next_code_language_boundary(self.code_toolbar.picker.cursor_offset()),
                    cx,
                );
                cx.stop_propagation();
            }
            "home" => {
                self.move_code_language_to(0, cx);
                cx.stop_propagation();
            }
            "end" => {
                self.move_code_language_to(self.code_toolbar.picker.query.len(), cx);
                cx.stop_propagation();
            }
            "space" => {
                self.replace_code_language_text_in_range(
                    self.code_toolbar.picker.selected_range.clone(),
                    " ",
                    None,
                    true,
                    cx,
                );
                cx.stop_propagation();
            }
            key => {
                if !ctrl && !keystroke.modifiers.alt && !key.is_empty() {
                    let mut chars = key.chars();
                    if let Some(first) = chars.next() {
                        if chars.next().is_none() && !first.is_control() {
                            self.replace_code_language_text_in_range(
                                self.code_toolbar.picker.selected_range.clone(),
                                key,
                                None,
                                true,
                                cx,
                            );
                            cx.stop_propagation();
                        }
                    }
                }
            }
        }
    }
}
