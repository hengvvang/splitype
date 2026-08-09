//! Block focus management — pending focus, cursor reset, and range focus.
//!
//! Focus state lives on the active tab (`FocusState`); these helpers route
//! focus requests from block input handlers and structural edits.

use std::time::Instant;

use gpui::*;

use crate::editor::controller::Editor;
use crate::editor::tree::block::Block;

impl Editor {
    pub(crate) fn focus_block(&mut self, entity_id: EntityId) {
        self.tab_mut().focus.pending = Some(entity_id);
        self.tab_mut().focus.active_entity = Some(entity_id);
        self.tab_mut().focus.pending_scroll_active_block_into_view = true;
    }

    pub(crate) fn reset_block_cursor(block: &Entity<Block>, cursor: usize, cx: &mut Context<Self>) {
        block.update(cx, move |block, cx| {
            block.selected_range = cursor..cursor;
            block.selection_reversed = false;
            block.marked_range = None;
            block.vertical_motion_x = None;
            block.cursor_blink_epoch = Instant::now();
            cx.notify();
        });
    }

    pub(crate) fn focus_block_range(
        &mut self,
        block: &Entity<Block>,
        range: std::ops::Range<usize>,
        cx: &mut Context<Self>,
    ) {
        block.update(cx, move |block, cx| {
            block.selected_range = range.clone();
            block.selection_reversed = false;
            block.marked_range = None;
            block.vertical_motion_x = None;
            block.cursor_blink_epoch = Instant::now();
            cx.notify();
        });
        self.focus_block(block.entity_id());
    }
}
