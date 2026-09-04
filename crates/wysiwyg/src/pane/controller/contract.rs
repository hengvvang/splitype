//! WysiwygDocumentController — contract handlers.

use editor_contracts::{EditTransaction, OutlineNode, SearchMatch, SearchQuery};
use gpui::{App, Context, FocusHandle};
use theme::Theme;

use crate::model::block::CollapsedCaretAffinity;

use super::WysiwygDocumentController;
impl WysiwygDocumentController {
    pub fn notify_document_changed(&mut self, cx: &mut App) {
        self.pending_edit = true;
        self.commit_document_edit(false, cx);
    }

    pub fn focus_handle(&self, cx: &App) -> Option<FocusHandle> {
        if let Some(active) = &self.active_entity {
            return Some(active.read(cx).focus_handle.clone());
        }
        if let Some(doc) = &self.document {
            if let Some(first) = doc.blocks().first() {
                return Some(first.entity.read(cx).focus_handle.clone());
            }
        }
        None
    }

    pub fn outline_headings(&self, _cx: &App) -> Vec<OutlineNode> {
        self.document
            .as_ref()
            .map(|doc| {
                doc.index
                    .headings
                    .iter()
                    .map(|heading| OutlineNode {
                        id: format!("outline:{}", heading.entity_id),
                        label: heading.label.clone(),
                        level: heading.level,
                        block_index: heading.block_index,
                        block_id: Some(heading.entity_id),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The estimated pixel offset of `target_block_idx`, from the index's
    /// cached cumulative heights. O(1).
    pub fn calculate_block_scroll_offset(&self, target_block_idx: usize) -> f32 {
        self.document
            .as_ref()
            .map(|doc| {
                doc.index
                    .cumulative_heights
                    .get(target_block_idx)
                    .copied()
                    .unwrap_or(0.0)
            })
            .unwrap_or(0.0)
    }

    pub fn navigate_to_outline(
        &mut self,
        index: usize,
        _theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Option<f32> {
        let headings = self.outline_headings(cx);
        if let Some(node) = headings.get(index) {
            if let Some(entity_id) = node.block_id {
                if let Some(doc) = &self.document {
                    if let Some(block) = doc.block_entity_by_id(entity_id) {
                        self.active_entity = Some(block.clone());
                        block.update(cx, |b, cx| {
                            b.assign_collapsed_selection_offset(
                                0,
                                CollapsedCaretAffinity::Default,
                                None,
                            );
                            b.start_cursor_blink(cx);
                            cx.notify();
                        });
                    }
                }
            }
            let target_y = self.calculate_block_scroll_offset(node.block_index);
            Some(target_y)
        } else {
            None
        }
    }

    /// Keeps the block at `block_index` visible: the virtualized render
    /// mounts rows only around the viewport, so cross-block caret movement
    /// scrolls the pane to the block's estimated position through the host.
    pub fn scroll_block_into_view(&mut self, block_index: usize, cx: &mut App) {
        let target_y = self.calculate_block_scroll_offset(block_index);
        if let (Some(host), Some(pane_id)) = (self.host.clone(), self.pane_id) {
            host.scroll_pane_to_y(pane_id, target_y, cx);
        }
    }

    pub fn search_matches(&self, query: &SearchQuery, cx: &App) -> Vec<SearchMatch> {
        if let Some(doc) = &self.document {
            crate::pane::search::search_in_document(doc, query, cx)
        } else {
            Vec::new()
        }
    }

    pub fn replace_match(
        &mut self,
        match_item: &SearchMatch,
        replace_with: &str,
        cx: &mut Context<Self>,
    ) -> Option<EditTransaction> {
        if let Some(doc) = &self.document {
            if let Some(entity_id) = match_item.entity_id {
                crate::pane::search::replace_in_block_entity(
                    doc,
                    entity_id,
                    match_item.byte_range.clone(),
                    replace_with,
                    cx,
                );
                self.pending_edit = true;
                cx.notify();
                return self.take_edit_transaction(false, cx);
            }
        }
        None
    }

    pub fn navigate_to_search_match(
        &mut self,
        match_item: &SearchMatch,
        cx: &mut Context<Self>,
    ) -> Option<f32> {
        let mut target_y = None;
        if let Some(doc) = &self.document {
            if let Some(entity_id) = match_item.entity_id {
                if let Some(block) = doc.block_entity_by_id(entity_id) {
                    self.active_entity = Some(block.clone());
                    block.update(cx, |b, cx| {
                        b.selected_range = match_item.byte_range.clone();
                        b.selection_reversed = false;
                        b.start_cursor_blink(cx);
                        cx.notify();
                    });
                }
                if let Some(block_index) = doc.index_for_entity_id(entity_id) {
                    target_y = Some(self.calculate_block_scroll_offset(block_index));
                }
            }
        }
        target_y
    }

    /// The selected display text of the active block, when non-empty.
    pub fn selected_text(&self, cx: &App) -> Option<String> {
        self.active_entity.as_ref().and_then(|active| {
            let block = active.read(cx);
            if block.selected_range.is_empty() {
                None
            } else {
                Some(block.selected_text())
            }
        })
    }

    /// Deletes the active block's selection and returns the resulting edit
    /// transaction. The block's change event also commits it — the buffer
    /// dedupes the identical text, so the caller's commit is the one that
    /// lands with correct caret hints.
    pub fn delete_selection(&mut self, cx: &mut Context<Self>) -> Option<EditTransaction> {
        let active = self.active_entity.clone()?;
        if active.read(cx).selected_range.is_empty() {
            return None;
        }
        active.update(cx, |block, cx| {
            let range = block.selected_range.clone();
            block.replace_text_in_display_range(range, "", Some(0..0), false, cx);
        });
        self.pending_edit = true;
        cx.notify();
        self.take_edit_transaction(false, cx)
    }

    /// Inserts text at the active block's selection/caret and returns the
    /// resulting edit transaction. The block's change event also commits
    /// it — the buffer dedupes the identical text, so the caller's commit
    /// is the one that lands with correct caret hints.
    pub fn insert_text(&mut self, text: &str, cx: &mut Context<Self>) -> Option<EditTransaction> {
        let active = self.active_entity.clone()?;
        active.update(cx, |block, cx| {
            let range = block.selected_range.clone();
            if range.is_empty() {
                let cursor = block.cursor_offset();
                block.replace_text_in_display_range(cursor..cursor, text, None, false, cx);
            } else {
                block.replace_text_in_display_range(range, text, None, false, cx);
            }
        });
        self.pending_edit = true;
        cx.notify();
        self.take_edit_transaction(false, cx)
    }

    /// Selects the whole active block.
    pub fn select_all(&mut self, cx: &mut Context<Self>) {
        if let Some(active) = self.active_entity.clone() {
            active.update(cx, |block, cx| {
                block.select_all_text(cx);
            });
        }
    }
}
