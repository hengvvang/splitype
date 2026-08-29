//! Cross-block selection representation and visual state synchronization.

use gpui::*;

use crate::editor_scheduler::engine::controller::{
    CrossBlockSelection, CrossBlockSelectionEndpoint, Editor, EditorSelection,
};

/// Cross-block selection with endpoints ordered by document position.
#[derive(Clone, Copy)]
pub(crate) struct NormalizedCrossBlockSelection {
    pub(crate) start: CrossBlockSelectionEndpoint,
    pub(crate) end: CrossBlockSelectionEndpoint,
    pub(crate) start_index: usize,
    pub(crate) end_index: usize,
    pub(crate) reversed: bool,
}

impl NormalizedCrossBlockSelection {
    /// Whether this selection is contained within a single block.
    #[inline]
    pub(crate) const fn is_single_block(&self) -> bool {
        self.start_index == self.end_index
    }

    /// Inclusive range of block indices spanned by this selection.
    #[inline]
    pub(crate) const fn block_index_range(&self) -> std::ops::RangeInclusive<usize> {
        self.start_index..=self.end_index
    }

    /// Whether the specified block index falls within this selection.
    #[inline]
    pub(crate) fn contains_block(&self, index: usize) -> bool {
        self.block_index_range().contains(&index)
    }
}

impl Editor {
    /// Return the currently active editor selection representation.
    pub(crate) fn active_selection(&self, cx: &App) -> EditorSelection {
        if let Some(cross_block) = self.active_pane_selection().cross_block {
            return EditorSelection::CrossBlock(cross_block);
        }
        if let Some(axis) = self.tab().tables.axis_selection {
            return EditorSelection::TableAxis(axis);
        }
        if let Some(active_id) = self.active_pane_focus().active_entity {
            if let Some(block) = self.doc().block_entity_by_id(active_id) {
                let block_ref = block.read(cx);
                if !block_ref.selected_range.is_empty() {
                    return EditorSelection::IntraBlock {
                        block_id: active_id,
                        range: block_ref.selected_range.clone(),
                        reversed: block_ref.selection_reversed,
                    };
                }
            }
        }
        EditorSelection::None
    }
    pub(crate) fn clear_cross_block_selection_visuals(&mut self, cx: &mut App) -> bool {
        let mut changed = false;
        for entries in self.doc().blocks() {
            entries.entity.update(cx, |block, cx| {
                if block.editor_selection_range.take().is_some() {
                    changed = true;
                    cx.notify();
                }
            });
        }
        changed
    }

    pub(crate) fn clear_cross_block_selection(&mut self, cx: &mut App) {
        let had_selection = self
            .active_pane_state()
            .selection_mut()
            .map(|s| s.clear_cross_block())
            .unwrap_or(false);
        // Visual ranges are only ever written while a cross-block selection
        // (or drag) is active, so when there was none the all-blocks scan
        // below has nothing to clear. Skipping it removes an O(blocks) entity
        // update from every keystroke while editing without a selection.
        let changed_visuals = if had_selection {
            self.clear_cross_block_selection_visuals(cx)
        } else {
            false
        };
        let changed = had_selection || changed_visuals;
        if changed {
            cx.notify(self.entity_id);
        }
    }

    pub(crate) fn is_cross_block_selection_empty(&self, selection: CrossBlockSelection) -> bool {
        let Some(anchor_index) = self.doc().index_for_entity_id(selection.anchor.entity_id) else {
            return true;
        };
        let Some(focus_index) = self.doc().index_for_entity_id(selection.focus.entity_id) else {
            return true;
        };
        anchor_index == focus_index && selection.anchor.offset == selection.focus.offset
    }

    pub(crate) fn normalized_cross_block_selection(
        &self,
        cx: &App,
    ) -> Option<NormalizedCrossBlockSelection> {
        let selection = match self.active_selection(cx) {
            EditorSelection::CrossBlock(cb) => cb,
            _ => return None,
        };
        let anchor = self.clamp_cross_block_endpoint(selection.anchor, cx)?;
        let focus = self.clamp_cross_block_endpoint(selection.focus, cx)?;
        let anchor_index = self.doc().index_for_entity_id(anchor.entity_id)?;
        let focus_index = self.doc().index_for_entity_id(focus.entity_id)?;
        let reversed = focus_index < anchor_index
            || (focus_index == anchor_index && focus.offset < anchor.offset);
        let (start, end, start_index, end_index) = if reversed {
            (focus, anchor, focus_index, anchor_index)
        } else {
            (anchor, focus, anchor_index, focus_index)
        };
        if start_index == end_index && start.offset == end.offset {
            return None;
        }
        Some(NormalizedCrossBlockSelection {
            start,
            end,
            start_index,
            end_index,
            reversed,
        })
    }

    pub(crate) fn clamp_cross_block_endpoint(
        &self,
        endpoint: CrossBlockSelectionEndpoint,
        cx: &App,
    ) -> Option<CrossBlockSelectionEndpoint> {
        let entity = self.doc().block_entity_by_id(endpoint.entity_id)?;
        let len = entity.read(cx).display_len();
        Some(CrossBlockSelectionEndpoint {
            entity_id: endpoint.entity_id,
            offset: endpoint.offset.min(len),
        })
    }

    pub(crate) fn sync_cross_block_selection_visuals(&mut self, cx: &mut Context<Self>) {
        let normalized = self.normalized_cross_block_selection(cx);
        for (index, entries) in self.doc().blocks().iter().enumerate() {
            let next_range = normalized.and_then(|selection| {
                if !selection.contains_block(index) {
                    return None;
                }
                let block = entries.entity.read(cx);
                let len = block.display_len();
                let range = if selection.is_single_block() {
                    selection.start.offset.min(len)..selection.end.offset.min(len)
                } else if index == selection.start_index {
                    selection.start.offset.min(len)..len
                } else if index == selection.end_index {
                    0..selection.end.offset.min(len)
                } else {
                    0..len
                };
                (!range.is_empty()).then_some(range)
            });

            entries.entity.update(cx, |block, cx| {
                if block.editor_selection_range != next_range {
                    block.editor_selection_range = next_range.clone();
                    cx.notify();
                }
            });
        }
    }
}
