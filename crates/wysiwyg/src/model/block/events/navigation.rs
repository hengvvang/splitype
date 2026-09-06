//! Cursor navigation and selection action handlers on a focused block:
//! movement, word/line selection, and vertical block traversal.

use gpui::*;

use crate::model::block::Block;
use crate::model::protocol::BlockEvent;
use crate::pane::actions::{
    BlockDown, BlockUp, End, FocusNext, FocusPrevious, Home, MoveLeft, MoveRight, SelectEnd,
    SelectHome, SelectLeft, SelectRight, WordMoveLeft, WordMoveRight, WordSelectLeft,
    WordSelectRight,
};
impl Block {
    pub fn on_focus_previous(
        &mut self,
        _: &FocusPrevious,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.code_language_focus_handle.is_focused(window) {
            self.on_code_language_focus_content(&FocusPrevious, window, cx);
            return;
        }

        let preferred_x = self.vertical_anchor_x();
        if !self.move_cursor_vertically(-1, preferred_x, cx) {
            if self.is_table_cell() {
                cx.emit(BlockEvent::RequestTableCellMoveVertical { delta: -1 });
                return;
            }
            cx.emit(BlockEvent::RequestFocusPrevious {
                preferred_x: Some(f32::from(preferred_x)),
            });
        }
    }

    pub fn on_focus_next(&mut self, _: &FocusNext, window: &mut Window, cx: &mut Context<Self>) {
        if self.code_language_focus_handle.is_focused(window) {
            self.on_code_language_focus_next(&FocusNext, window, cx);
            return;
        }

        let preferred_x = self.vertical_anchor_x();
        if !self.move_cursor_vertically(1, preferred_x, cx) {
            if self.is_table_cell() {
                cx.emit(BlockEvent::RequestTableCellMoveVertical { delta: 1 });
                return;
            }
            // In a code block, Down from the last content line steps into the
            // language field rather than leaving the block, so the language is
            // reachable by keyboard. A further Down there exits the block.
            if self.kind().is_code_block() && !self.code_language_focus_handle.is_focused(window) {
                self.code_language_focus_handle.focus(window, cx);
                cx.notify();
                return;
            }
            cx.emit(BlockEvent::RequestFocusNext {
                preferred_x: Some(f32::from(preferred_x)),
            });
        }
    }

    pub fn on_move_left(&mut self, _: &MoveLeft, window: &mut Window, cx: &mut Context<Self>) {
        if self.code_language_focus_handle.is_focused(window) {
            self.on_code_language_move_left(&MoveLeft, window, cx);
            return;
        }

        if self.selected_range.is_empty() {
            if let Some((target, affinity)) = self.projected_move_left_target(self.cursor_offset())
            {
                self.assign_collapsed_selection_offset(target, affinity, None);
                self.cursor_blink_epoch = std::time::Instant::now();
                cx.notify();
            } else {
                let previous = self.previous_boundary(self.cursor_offset());
                // At the start of a table cell, step into the previous cell
                // rather than stalling at the edge (same path as Shift+Tab).
                if previous == self.cursor_offset() && self.is_table_cell() {
                    cx.emit(BlockEvent::RequestTableCellMoveHorizontal { delta: -1 });
                    return;
                }
                self.move_to(previous, cx);
            }
        } else {
            self.move_to(self.selected_range.start, cx);
        }
    }

    pub fn on_move_right(&mut self, _: &MoveRight, window: &mut Window, cx: &mut Context<Self>) {
        if self.code_language_focus_handle.is_focused(window) {
            self.on_code_language_move_right(&MoveRight, window, cx);
            return;
        }

        if self.selected_range.is_empty() {
            if let Some((target, affinity)) =
                self.projected_move_right_target(self.selected_range.end)
            {
                self.assign_collapsed_selection_offset(target, affinity, None);
                self.cursor_blink_epoch = std::time::Instant::now();
                cx.notify();
            } else {
                let next = self.next_boundary(self.selected_range.end);
                // At the end of a table cell, step into the next cell rather
                // than stalling at the edge (same path as Tab).
                if next == self.selected_range.end && self.is_table_cell() {
                    cx.emit(BlockEvent::RequestTableCellMoveHorizontal { delta: 1 });
                    return;
                }
                self.move_to(next, cx);
            }
        } else {
            self.move_to(self.selected_range.end, cx);
        }
    }

    pub fn on_home(&mut self, _: &Home, window: &mut Window, cx: &mut Context<Self>) {
        if self.code_language_focus_handle.is_focused(window) {
            self.on_code_language_home(&Home, window, cx);
            return;
        }

        self.move_to(0, cx);
    }

    pub fn on_end(&mut self, _: &End, window: &mut Window, cx: &mut Context<Self>) {
        if self.code_language_focus_handle.is_focused(window) {
            self.on_code_language_end(&End, window, cx);
            return;
        }

        self.move_to(self.display_len(), cx);
    }

    pub fn on_select_left(&mut self, _: &SelectLeft, window: &mut Window, cx: &mut Context<Self>) {
        if self.code_language_focus_handle.is_focused(window) {
            self.on_code_language_select_left(&SelectLeft, window, cx);
            return;
        }

        if let Some((target, _)) = self.projected_move_left_target(self.cursor_offset()) {
            self.select_to(target, cx);
        } else {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx);
        }
    }

    pub fn on_select_right(
        &mut self,
        _: &SelectRight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.code_language_focus_handle.is_focused(window) {
            self.on_code_language_select_right(&SelectRight, window, cx);
            return;
        }

        if let Some((target, _)) = self.projected_move_right_target(self.cursor_offset()) {
            self.select_to(target, cx);
        } else {
            self.select_to(self.next_boundary(self.cursor_offset()), cx);
        }
    }

    pub fn on_word_move_left(
        &mut self,
        _: &WordMoveLeft,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.code_language_focus_handle.is_focused(window) {
            return;
        }

        self.move_to(self.previous_word_start(self.cursor_offset()), cx);
    }

    pub fn on_word_move_right(
        &mut self,
        _: &WordMoveRight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.code_language_focus_handle.is_focused(window) {
            return;
        }

        self.move_to(self.next_word_start(self.cursor_offset()), cx);
    }

    pub fn on_word_select_left(
        &mut self,
        _: &WordSelectLeft,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.code_language_focus_handle.is_focused(window) {
            return;
        }

        self.select_to(self.previous_word_start(self.cursor_offset()), cx);
    }

    pub fn on_word_select_right(
        &mut self,
        _: &WordSelectRight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.code_language_focus_handle.is_focused(window) {
            return;
        }

        self.select_to(self.next_word_start(self.cursor_offset()), cx);
    }

    pub fn on_block_up(&mut self, _: &BlockUp, window: &mut Window, cx: &mut Context<Self>) {
        if self.code_language_focus_handle.is_focused(window) {
            return;
        }

        cx.emit(BlockEvent::RequestBlockUp);
    }

    pub fn on_block_down(&mut self, _: &BlockDown, window: &mut Window, cx: &mut Context<Self>) {
        if self.code_language_focus_handle.is_focused(window) {
            return;
        }

        cx.emit(BlockEvent::RequestBlockDown);
    }

    pub fn select_all_text(&mut self, cx: &mut Context<Self>) {
        self.move_to(0, cx);
        self.select_to(self.display_len(), cx);
    }

    pub fn on_select_home(&mut self, _: &SelectHome, window: &mut Window, cx: &mut Context<Self>) {
        if self.code_language_focus_handle.is_focused(window) {
            return;
        }

        self.select_to(0, cx);
    }

    pub fn on_select_end(&mut self, _: &SelectEnd, window: &mut Window, cx: &mut Context<Self>) {
        if self.code_language_focus_handle.is_focused(window) {
            return;
        }

        self.select_to(self.display_len(), cx);
    }
}
