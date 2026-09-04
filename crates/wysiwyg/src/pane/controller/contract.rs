//! WysiwygDocumentController — contract handlers.

use editor_contracts::{EditTransaction, OutlineNode, SearchMatch, SearchQuery};
use gpui::{App, Context, FocusHandle};
use theme::{Theme, ThemeManager};

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

    pub fn outline_headings(&self, cx: &App) -> Vec<OutlineNode> {
        if let Some(doc) = &self.document {
            crate::pane::outline::extract_outline_headings(doc, cx)
        } else {
            Vec::new()
        }
    }

    pub fn calculate_block_scroll_offset(
        &self,
        target_block_idx: usize,
        theme: &Theme,
        cx: &App,
    ) -> f32 {
        let Some(doc) = &self.document else {
            return 0.0;
        };
        let blocks = doc.blocks();
        let font_size = theme.typography.text_size.max(14.0);
        let line_height = (font_size * theme.typography.text_line_height)
            .round()
            .max(22.0);
        let mut y = 0.0;
        for (i, entry) in blocks.iter().enumerate() {
            if i >= target_block_idx {
                break;
            }
            let block = entry.entity.read(cx);
            let est_h = match block.kind() {
                markdown_parser::parse::BlockKind::Heading { level } => match level {
                    1 => line_height * 2.2 + 16.0,
                    2 => line_height * 1.8 + 14.0,
                    3 => line_height * 1.5 + 12.0,
                    _ => line_height * 1.3 + 10.0,
                },
                markdown_parser::parse::BlockKind::Paragraph => {
                    let len = block.data.text.plain_len();
                    let lines = (len / 60).max(1);
                    (lines as f32) * line_height + 10.0
                }
                markdown_parser::parse::BlockKind::CodeBlock { .. } => {
                    let lines = block.data.text.plain_text().lines().count().max(1);
                    (lines as f32) * line_height + 24.0
                }
                markdown_parser::parse::BlockKind::Table => line_height * 4.0 + 16.0,
                markdown_parser::parse::BlockKind::ThematicBreak => line_height * 1.0 + 8.0,
                _ => line_height * 1.5 + 8.0,
            };
            y += est_h;
        }
        y
    }

    pub fn navigate_to_outline(
        &mut self,
        index: usize,
        theme: &Theme,
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
            let target_y = self.calculate_block_scroll_offset(node.block_index, theme, cx);
            Some(target_y)
        } else {
            None
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
                    let theme = cx.global::<ThemeManager>().current_arc();
                    target_y = Some(self.calculate_block_scroll_offset(block_index, &theme, cx));
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
