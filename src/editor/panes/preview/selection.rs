//! Independent selection engine for the Preview pane.
//!
//! Standard-First: Works strictly on the read-only CommonMark AST snapshot.

use std::ops::Range;

/// Represents a character boundary point inside a specific preview block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PreviewEndpoint {
    pub(crate) block_index: usize,
    pub(crate) offset: usize,
}

/// Continuous read-only text selection across preview blocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PreviewSelectionRange {
    pub(crate) start: PreviewEndpoint,
    pub(crate) end: PreviewEndpoint,
    pub(crate) is_reversed: bool,
}

impl PreviewSelectionRange {
    pub(crate) fn new(anchor: PreviewEndpoint, focus: PreviewEndpoint) -> Self {
        let is_reversed = focus.block_index < anchor.block_index
            || (focus.block_index == anchor.block_index && focus.offset < anchor.offset);
        let (start, end) = if is_reversed {
            (focus, anchor)
        } else {
            (anchor, focus)
        };
        Self {
            start,
            end,
            is_reversed,
        }
    }

    /// Whether this selection is empty (collapsed).
    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.start.block_index == self.end.block_index && self.start.offset == self.end.offset
    }

    /// Returns the character selection range for a specific block index, if it falls within the selection.
    pub(crate) fn range_for_block(&self, block_index: usize, block_len: usize) -> Option<Range<usize>> {
        if block_index < self.start.block_index || block_index > self.end.block_index {
            return None;
        }

        let start = if block_index == self.start.block_index {
            self.start.offset.min(block_len)
        } else {
            0
        };

        let end = if block_index == self.end.block_index {
            self.end.offset.min(block_len)
        } else {
            block_len
        };

        if start < end {
            Some(start..end)
        } else {
            None
        }
    }
}

use gpui::*;
use crate::editor::engine::controller::{Editor, PaneId};

impl Editor {
    pub(crate) fn on_preview_mouse_down(
        &mut self,
        pane_id: PaneId,
        block_index: usize,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.pane_state_mut(pane_id) else {
            return;
        };
        let Some(block) = state.preview.blocks.get(block_index) else {
            return;
        };
        let offset = block.read(cx).index_for_mouse_position(position);
        state.preview.drag_anchor = Some(PreviewEndpoint { block_index, offset });
        state.preview.selection = None;
        cx.notify();
    }

    pub(crate) fn on_preview_mouse_move(
        &mut self,
        pane_id: PaneId,
        block_index: usize,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.pane_state_mut(pane_id) else {
            return;
        };
        let Some(anchor) = state.preview.drag_anchor else {
            return;
        };
        let Some(block) = state.preview.blocks.get(block_index) else {
            return;
        };
        let offset = block.read(cx).index_for_mouse_position(position);
        let focus = PreviewEndpoint { block_index, offset };
        let selection = PreviewSelectionRange::new(anchor, focus);
        if selection.is_empty() {
            state.preview.selection = None;
        } else {
            state.preview.selection = Some(selection);
        }
        cx.notify();
    }

    pub(crate) fn on_preview_mouse_up(
        &mut self,
        pane_id: PaneId,
        cx: &mut Context<Self>,
    ) {
        if let Some(state) = self.pane_state_mut(pane_id) {
            state.preview.drag_anchor = None;
            cx.notify();
        }
    }

    pub(crate) fn preview_selected_text(&self, cx: &App) -> Option<String> {
        let pane_id = self.active_pane_id();
        let state = self.pane_state_ref(pane_id)?;
        let selection = state.preview.selection?;
        if selection.is_empty() {
            return None;
        }

        let mut lines = Vec::new();
        for (index, entity) in state.preview.blocks.iter().enumerate() {
            let block = entity.read(cx);
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preview_selection_single_block_forward() {
        let anchor = PreviewEndpoint { block_index: 0, offset: 2 };
        let focus = PreviewEndpoint { block_index: 0, offset: 8 };
        let sel = PreviewSelectionRange::new(anchor, focus);
        assert!(!sel.is_reversed);
        assert_eq!(sel.range_for_block(0, 10), Some(2..8));
        assert_eq!(sel.range_for_block(1, 10), None);
    }

    #[test]
    fn test_preview_selection_single_block_reversed() {
        let anchor = PreviewEndpoint { block_index: 0, offset: 8 };
        let focus = PreviewEndpoint { block_index: 0, offset: 2 };
        let sel = PreviewSelectionRange::new(anchor, focus);
        assert!(sel.is_reversed);
        assert_eq!(sel.range_for_block(0, 10), Some(2..8));
    }

    #[test]
    fn test_preview_selection_multi_block() {
        let anchor = PreviewEndpoint { block_index: 1, offset: 5 };
        let focus = PreviewEndpoint { block_index: 3, offset: 4 };
        let sel = PreviewSelectionRange::new(anchor, focus);
        assert_eq!(sel.range_for_block(0, 10), None);
        assert_eq!(sel.range_for_block(1, 10), Some(5..10));
        assert_eq!(sel.range_for_block(2, 10), Some(0..10));
        assert_eq!(sel.range_for_block(3, 10), Some(0..4));
        assert_eq!(sel.range_for_block(4, 10), None);
    }
}


