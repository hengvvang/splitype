//! Block focus management — pending focus, cursor reset, and range focus.
//!
//! Focus state lives on the active pane (`PaneState::focus`); these helpers
//! route focus requests from block input handlers and structural edits to
//! the pane that owns the window keyboard focus.

use std::time::Instant;

use gpui::*;

use crate::engine::controller::Editor;
use editor_wysiwyg::document::block::Block;

impl Editor {
    pub(crate) fn focus_block(&mut self, entity_id: EntityId) {
        let pane = self.active_pane_state();
        if let Some(wysiwyg) = pane.as_wysiwyg_mut() {
            wysiwyg.focus.pending = Some(entity_id);
            wysiwyg.focus.active_entity = Some(entity_id);
        }
        pane.scroll.pending_autoscroll = Some(crate::engine::controller::AutoscrollStrategy::Fit {
            margin: px(20.0),
        });
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
