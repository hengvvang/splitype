//! Preview pane input: mouse selection state transitions.
//!
//! Pure state operations on [`PreviewState`]; the coordinating crate
//! routes events here and notifies after a change.

use gpui::{Pixels, Point};

use crate::selection::{PreviewEndpoint, PreviewSelectionRange};
use crate::state::PreviewState;

/// Mouse-down on the preview block at `block_index`: start (or restart) a
/// drag selection anchor at the clicked offset.
pub fn handle_mouse_down(state: &mut PreviewState, block_index: usize, position: Point<Pixels>) {
    let Some(block) = state.blocks.get(block_index) else {
        return;
    };
    let offset = block.index_for_mouse_position(position);
    state.drag_anchor = Some(PreviewEndpoint {
        block_index,
        offset,
    });
    state.selection = None;
}

/// Mouse-move while dragging over the preview block at `block_index`:
/// extend the drag selection to the current offset.
pub fn handle_mouse_move(state: &mut PreviewState, block_index: usize, position: Point<Pixels>) {
    let Some(anchor) = state.drag_anchor else {
        return;
    };
    let Some(block) = state.blocks.get(block_index) else {
        return;
    };
    let offset = block.index_for_mouse_position(position);
    let focus = PreviewEndpoint {
        block_index,
        offset,
    };
    let selection = PreviewSelectionRange::new(anchor, focus);
    if selection.is_empty() {
        state.selection = None;
    } else {
        state.selection = Some(selection);
    }
}

/// Mouse-up ends the drag selection session.
pub fn handle_mouse_up(state: &mut PreviewState) {
    state.drag_anchor = None;
}

/// The text covered by the current selection, if any (for the Copy
/// command), joining block slices with a blank line.
pub fn selected_text(state: &PreviewState) -> Option<String> {
    let selection = state.selection?;
    if selection.is_empty() {
        return None;
    }

    let mut lines = Vec::new();
    for (index, block) in state.blocks.iter().enumerate() {
        let len = block.display_len();
        if let Some(range) = selection.range_for_block(index, len) {
            let text = block.display_text();
            let start = range.start.min(text.len());
            let end = range.end.min(text.len());
            if start < end {
                lines.push(text[start..end].to_string());
            }
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n\n"))
    }
}
