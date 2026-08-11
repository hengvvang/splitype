//! Code-language picker handlers on a focused block: the transient
//! language field above a code block, its editing, and dismissal.

use gpui::*;

use crate::editor::block_protocol::BlockAction;
use crate::editor::editing::input::actions::{
    Copy, Cut, Delete, DeleteBack, DismissTransientUi, End, FocusNext, FocusPrev, Home,
    IndentBlock, MoveLeft, MoveRight, Newline, OutdentBlock, Paste, SelectAll, SelectLeft,
    SelectRight,
};
use crate::editor::render::code_highlight::options::code_language_options_matching;
use crate::editor::tree::block::Block;
impl Block {
    pub(crate) fn on_code_block_hover(
        &mut self,
        hovered: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let should_show = *hovered || self.code_language_picker_open;
        if self.code_toolbar_hovered != should_show {
            self.code_toolbar_hovered = should_show;
            cx.notify();
        }
    }

    pub(crate) fn on_code_language_picker_toggle(
        &mut self,
        _: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        self.code_language_picker_open = !self.code_language_picker_open;
        self.code_toolbar_hovered = self.code_language_picker_open;
        self.code_language_query.clear();
        self.code_language_selected_range = 0..0;
        self.code_language_selection_reversed = false;
        self.code_language_marked_range = None;
        if self.code_language_picker_open {
            self.code_language_focus_handle.focus(window);
        } else {
            self.focus_handle.focus(window);
        }
        cx.notify();
    }

    pub(crate) fn on_code_copy_button_mouse_down(
        &mut self,
        _: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        cx.write_to_clipboard(ClipboardItem::new_string(self.record.text.plain_text()));
    }

    pub(crate) fn on_code_language_newline(
        &mut self,
        _: &Newline,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        if self.code_language_picker_open {
            let value = code_language_options_matching(&self.code_language_query)
                .first()
                .map(|option| option.value);
            if let Some(value) = value {
                self.choose_code_language(value, cx);
            } else {
                self.code_language_picker_open = false;
                self.code_language_query.clear();
            }
        }
        self.focus_handle.focus(window);
        cx.notify();
    }

    pub(crate) fn on_code_language_dismiss(
        &mut self,
        _: &DismissTransientUi,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        self.code_language_picker_open = false;
        self.code_language_query.clear();
        self.focus_handle.focus(window);
        cx.notify();
    }

    pub(crate) fn on_code_language_delete_back(
        &mut self,
        _: &DeleteBack,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        if self.code_language_selected_range.is_empty() {
            let previous = self.previous_code_language_boundary(self.code_language_cursor_offset());
            self.select_code_language_to(previous, cx);
        }
        self.replace_code_language_text_in_range(
            self.code_language_selected_range.clone(),
            "",
            None,
            false,
            cx,
        );
    }

    pub(crate) fn on_code_language_delete(
        &mut self,
        _: &Delete,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        if self.code_language_selected_range.is_empty() {
            let next = self.next_code_language_boundary(self.code_language_cursor_offset());
            self.select_code_language_to(next, cx);
        }
        self.replace_code_language_text_in_range(
            self.code_language_selected_range.clone(),
            "",
            None,
            false,
            cx,
        );
    }

    pub(crate) fn on_code_language_move_left(
        &mut self,
        _: &MoveLeft,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        if self.code_language_selected_range.is_empty() {
            self.move_code_language_to(
                self.previous_code_language_boundary(self.code_language_cursor_offset()),
                cx,
            );
        } else {
            self.move_code_language_to(self.code_language_selected_range.start, cx);
        }
    }

    pub(crate) fn on_code_language_move_right(
        &mut self,
        _: &MoveRight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        if self.code_language_selected_range.is_empty() {
            self.move_code_language_to(
                self.next_code_language_boundary(self.code_language_cursor_offset()),
                cx,
            );
        } else {
            self.move_code_language_to(self.code_language_selected_range.end, cx);
        }
    }

    pub(crate) fn on_code_language_home(
        &mut self,
        _: &Home,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        self.move_code_language_to(0, cx);
    }

    pub(crate) fn on_code_language_end(
        &mut self,
        _: &End,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        self.move_code_language_to(self.code_language_input_text().len(), cx);
    }

    pub(crate) fn on_code_language_select_left(
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
            self.previous_code_language_boundary(self.code_language_cursor_offset()),
            cx,
        );
    }

    pub(crate) fn on_code_language_select_right(
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
            self.next_code_language_boundary(self.code_language_cursor_offset()),
            cx,
        );
    }

    pub(crate) fn on_code_language_select_all(
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

    pub(crate) fn on_code_language_copy(
        &mut self,
        _: &Copy,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        if !self.code_language_selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.code_language_input_text()[self.code_language_selected_range.clone()]
                    .to_string(),
            ));
        }
    }

    pub(crate) fn on_code_language_cut(
        &mut self,
        _: &Cut,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        if !self.code_language_selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.code_language_input_text()[self.code_language_selected_range.clone()]
                    .to_string(),
            ));
            self.replace_code_language_text_in_range(
                self.code_language_selected_range.clone(),
                "",
                None,
                false,
                cx,
            );
        }
    }

    pub(crate) fn on_code_language_paste(
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
                self.code_language_selected_range.clone(),
                &text,
                None,
                false,
                cx,
            );
        }
    }

    pub(crate) fn on_code_language_focus_content(
        &mut self,
        _: &FocusPrev,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        self.code_language_picker_open = false;
        self.code_language_query.clear();
        self.focus_handle.focus(window);
        cx.notify();
    }

    pub(crate) fn on_code_language_focus_next(
        &mut self,
        _: &FocusNext,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.code_language_focus_handle.is_focused(window) {
            return;
        }
        cx.stop_propagation();
        if self.code_language_picker_open {
            return;
        }
        // Down from the language field leaves the code block: the editor focuses
        // the block below, creating a trailing paragraph first when the code
        // block is the last block. Enter does not exit (see on_code_language_newline).
        cx.emit(BlockAction::RequestFocusNext { preferred_x: None });
    }

    pub(crate) fn on_code_language_indent(
        &mut self,
        _: &IndentBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.code_language_focus_handle.is_focused(window) {
            cx.stop_propagation();
        }
    }

    pub(crate) fn on_code_language_outdent(
        &mut self,
        _: &OutdentBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.code_language_focus_handle.is_focused(window) {
            cx.stop_propagation();
        }
    }

    pub(crate) fn on_code_language_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        self.code_language_is_selecting = true;
        self.code_language_focus_handle.focus(window);
        let offset = self.code_language_index_for_mouse_position(event.position);
        if event.modifiers.shift {
            self.select_code_language_to(offset, cx);
        } else {
            self.move_code_language_to(offset, cx);
        }
    }

    pub(crate) fn on_code_language_mouse_up(
        &mut self,
        _: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        self.code_language_is_selecting = false;
    }

    pub(crate) fn on_code_language_mouse_up_out(
        &mut self,
        _: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // GPUI dispatches mouse_up_out during capture; do not stop propagation
        // here, or controls under the pointer cannot synthesize on_click.
        if self.code_language_is_selecting {
            self.code_language_is_selecting = false;
            cx.notify();
        }
    }

    pub(crate) fn on_code_language_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.code_language_is_selecting {
            // A stale selecting flag can survive a missed mouse-up. Only extend
            // the selection while the platform still reports an active drag.
            if !event.dragging() {
                self.code_language_is_selecting = false;
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
}
