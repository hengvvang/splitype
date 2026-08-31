//! Block focus helpers and cursor resetting.

use std::time::Instant;

use gpui::*;

use crate::model::block::Block;

/// Resets the cursor position within a block and marks cursor blink epoch.
pub fn reset_block_cursor(block: &Entity<Block>, cursor: usize, cx: &mut App) {
    block.update(cx, move |block, cx| {
        block.selected_range = cursor..cursor;
        block.selection_reversed = false;
        block.marked_range = None;
        block.vertical_motion_x = None;
        block.cursor_blink_epoch = Instant::now();
        cx.notify();
    });
}

/// Sets the selection range within a block and marks cursor blink epoch.
pub fn set_block_selected_range(
    block: &Entity<Block>,
    range: std::ops::Range<usize>,
    cx: &mut App,
) {
    block.update(cx, move |block, cx| {
        block.selected_range = range.clone();
        block.selection_reversed = false;
        block.marked_range = None;
        block.vertical_motion_x = None;
        block.cursor_blink_epoch = Instant::now();
        cx.notify();
    });
}

