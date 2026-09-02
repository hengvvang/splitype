//! Self-contained WYSIWYG document controller — autonomous block editor engine.

use std::sync::Arc;

use editor_contracts::OutlineNode;
use editor_contracts::{CursorHint, EditTransaction, PaneOutlineHost, PaneRenderContext};
use editor_contracts::{SearchMatch, SearchQuery};
use gpui::{
    AnyElement, App, AppContext, Context, Div, ElementId, Entity, FocusHandle, InteractiveElement,
    IntoElement, ParentElement, StatefulInteractiveElement, Styled, Window, div, px,
};
use theme::Theme;
use theme::ThemeManager;

use crate::model::Document;
use crate::model::block::{Block, CollapsedCaretAffinity};
use crate::model::protocol::BlockEvent;
use crate::pane::state::{ReferenceRegistries, TableGrids};
use markdown_parser::inline::text::BlockText;
use markdown_parser::parse::{BlockData, BlockKind};

/// Autonomous controller for a WYSIWYG editor pane.
pub struct WysiwygDocumentController {
    pub host: Option<Arc<dyn editor_contracts::PaneHost>>,
    pub document: Option<Document>,
    pub synced_revision: Option<u64>,
    /// Local edits exist that the host's next snapshot has not acknowledged
    /// yet; `sync_document` consumes this instead of rebuilding.
    pub pending_edit: bool,
    pub active_entity: Option<Entity<Block>>,
    pub tables: TableGrids,
    pub references: ReferenceRegistries,
    /// Text of the document after the previous commit, used to detect
    /// typing-run continuations (single-character insertions at the same
    /// position) for undo grouping.
    last_committed_text: Option<String>,
    /// Insert position of the previous typing commit, when it was a
    /// single-character insertion.
    last_typing_insert_at: Option<usize>,
    /// Caret hint captured at the start of the current typing run.
    typing_run_start_hint: Option<CursorHint>,
    /// Caret hint after the previous commit.
    last_cursor_hint: Option<CursorHint>,
}

impl WysiwygDocumentController {
    pub fn new(document: &editor_contracts::DocumentSnapshot, cx: &mut Context<Self>) -> Self {
        let mut controller = Self {
            host: None,
            document: None,
            synced_revision: None,
            pending_edit: false,
            active_entity: None,
            tables: TableGrids::default(),
            references: ReferenceRegistries {
                base_dir: document
                    .base_dir
                    .as_deref()
                    .map(std::path::Path::to_path_buf),
                ..ReferenceRegistries::default()
            },
            last_committed_text: None,
            last_typing_insert_at: None,
            typing_run_start_hint: None,
            last_cursor_hint: None,
        };
        controller.rebuild_from_markdown(&document.text, document.revision, cx);
        controller
    }

    /// Creates a new block entity and subscribes this controller to its `BlockEvent` stream.
    pub fn new_block(cx: &mut Context<Self>, data: BlockData) -> Entity<Block> {
        let block = cx.new(|cx| Block::with_data(cx, data));
        cx.subscribe(&block, Self::on_block_event).detach();
        block
    }

    /// Rebuilds document tree and event subscriptions from raw Markdown text.
    pub fn rebuild_from_markdown(&mut self, text: &str, revision: u64, cx: &mut Context<Self>) {
        let parsed = markdown_parser::parse::parse_wysiwyg_document(text);
        let block_count = parsed.len();
        let mut entities: std::collections::HashMap<uuid::Uuid, Entity<Block>> =
            std::collections::HashMap::with_capacity(block_count);

        for block_data in &parsed {
            let entity = Self::new_block(cx, block_data.clone());
            entities.insert(block_data.id.0, entity);
        }

        for block_data in &parsed {
            if block_data.children.is_empty() {
                continue;
            }
            if let Some(parent_entity) = entities.get(&block_data.id.0) {
                let mut child_entities: Vec<Entity<Block>> =
                    Vec::with_capacity(block_data.children.len());
                for child_id in &block_data.children {
                    if let Some(child_entity) = entities.get(&child_id.0) {
                        child_entities.push(child_entity.clone());
                    }
                }
                if !child_entities.is_empty() {
                    parent_entity.update(cx, |parent, _cx| {
                        parent.children.extend(child_entities);
                    });
                }
            }
        }

        let mut roots: Vec<Entity<Block>> = parsed
            .iter()
            .filter(|block| block.parent.is_none())
            .filter_map(|block| entities.get(&block.id.0).cloned())
            .collect();

        if roots.is_empty() {
            let empty_block = Self::new_block(
                cx,
                BlockData::new(BlockKind::Paragraph, BlockText::plain(String::new())),
            );
            roots.push(empty_block);
        }

        let mut doc = Document::new(roots);
        doc.rebuild_metadata_and_snapshot(cx);
        self.active_entity = doc.blocks().first().map(|b| b.entity.clone());
        self.document = Some(doc);
        self.synced_revision = Some(revision);
        self.pending_edit = false;
        self.last_committed_text = None;
        self.last_typing_insert_at = None;
        self.typing_run_start_hint = None;
        self.last_cursor_hint = None;
        self.sync_reference_context(cx);
    }

    fn sync_reference_context(&self, cx: &mut App) {
        let Some(document) = &self.document else {
            return;
        };
        for entry in document.blocks() {
            crate::model::references::sync_reference_context_for_block(
                &entry.entity,
                self.references.base_dir.as_deref(),
                self.references.image.clone(),
                self.references.link.clone(),
                self.references.footnotes.clone(),
                cx,
            );
        }
    }

    /// Handles events emitted by child blocks.
    pub fn on_block_event(
        &mut self,
        block: Entity<Block>,
        event: &BlockEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            BlockEvent::Changed => {
                self.pending_edit = true;
                let merge = self
                    .document_text(cx)
                    .map(|text| self.is_typing_continuation(&text, cx))
                    .unwrap_or(false);
                self.commit_document_edit(merge, cx);
                cx.notify();
            }
            BlockEvent::RequestFocus => {
                self.active_entity = Some(block.clone());
                cx.notify();
            }
            BlockEvent::RequestNewline {
                trailing,
                source_already_mutated: _,
            } => {
                if let Some(doc) = &mut self.document {
                    let Some(location) = doc.find_block_location(block.entity_id()) else {
                        return;
                    };
                    let current_kind = block.read(cx).kind();
                    let new_block = Self::new_block(
                        cx,
                        BlockData::new(current_kind.newline_sibling_kind(), trailing.clone()),
                    );
                    doc.insert_blocks_at(
                        location.parent,
                        location.index + 1,
                        vec![new_block.clone()],
                        cx,
                    );
                    self.active_entity = Some(new_block.clone());
                    new_block.update(cx, |b, cx| {
                        b.start_cursor_blink(cx);
                        cx.notify();
                    });
                    self.pending_edit = true;
                    self.commit_document_edit(false, cx);
                    cx.notify();
                }
            }
            BlockEvent::RequestNewlineAbove => {
                if let Some(doc) = &mut self.document {
                    let Some(location) = doc.find_block_location(block.entity_id()) else {
                        return;
                    };
                    let new_block = Self::new_block(
                        cx,
                        BlockData::new(BlockKind::Paragraph, BlockText::plain(String::new())),
                    );
                    doc.insert_blocks_at(
                        location.parent,
                        location.index,
                        vec![new_block.clone()],
                        cx,
                    );
                    self.active_entity = Some(new_block.clone());
                    new_block.update(cx, |b, cx| {
                        b.start_cursor_blink(cx);
                        cx.notify();
                    });
                    self.pending_edit = true;
                    self.commit_document_edit(false, cx);
                    cx.notify();
                }
            }
            BlockEvent::RequestMergeIntoPrevious { content } => {
                if let Some(doc) = &mut self.document {
                    let blocks = doc.blocks();
                    let current_idx = doc.index_for_entity_id(block.entity_id()).unwrap_or(0);
                    if current_idx > 0 {
                        let prev = blocks[current_idx - 1].entity.clone();
                        let cursor_pos = prev.read(cx).display_text().len();
                        let current_content = content.clone();
                        prev.update(cx, move |prev, cx| {
                            let mut text = prev.data.text.clone();
                            text.append(current_content);
                            prev.data.set_text(text);
                            prev.sync_render_cache();
                            prev.assign_collapsed_selection_offset(
                                cursor_pos,
                                CollapsedCaretAffinity::Default,
                                None,
                            );
                            prev.cursor_blink_epoch = std::time::Instant::now();
                            cx.notify();
                        });
                        doc.remove_block(block.entity_id(), cx);
                        self.active_entity = Some(prev.clone());
                        self.pending_edit = true;
                        self.commit_document_edit(false, cx);
                        cx.notify();
                    }
                }
            }
            BlockEvent::RequestDelete => {
                if let Some(doc) = &mut self.document {
                    let blocks = doc.blocks();
                    let count = blocks.len();
                    let current_idx = doc.index_for_entity_id(block.entity_id()).unwrap_or(0);
                    if count > 1 {
                        let target_idx = if current_idx > 0 { current_idx - 1 } else { 0 };
                        let target = blocks[target_idx].entity.clone();
                        doc.remove_block(block.entity_id(), cx);
                        self.active_entity = Some(target.clone());
                        target.update(cx, |t, cx| {
                            t.start_cursor_blink(cx);
                            cx.notify();
                        });
                    } else {
                        block.update(cx, |b, cx| {
                            b.data.text = BlockText::plain(String::new());
                            b.sync_render_cache();
                            cx.notify();
                        });
                    }
                    self.pending_edit = true;
                    self.commit_document_edit(false, cx);
                    cx.notify();
                }
            }
            BlockEvent::RequestFocusPrevious { preferred_x } => {
                if let Some(doc) = &self.document {
                    let blocks = doc.blocks();
                    let current_idx = doc.index_for_entity_id(block.entity_id()).unwrap_or(0);
                    if current_idx > 0 {
                        let prev = blocks[current_idx - 1].entity.clone();
                        prev.update(cx, |prev, cx| {
                            let offset =
                                prev.entry_offset_for_vertical_focus(true, preferred_x.map(px));
                            prev.move_to_with_preferred_x(offset, preferred_x.map(px), cx);
                            prev.start_cursor_blink(cx);
                            cx.notify();
                        });
                        self.active_entity = Some(prev.clone());
                        cx.notify();
                    }
                }
            }
            BlockEvent::RequestFocusNext { preferred_x } => {
                if let Some(doc) = &self.document {
                    let blocks = doc.blocks();
                    let current_idx = doc.index_for_entity_id(block.entity_id()).unwrap_or(0);
                    if current_idx + 1 < blocks.len() {
                        let next = blocks[current_idx + 1].entity.clone();
                        next.update(cx, |next, cx| {
                            let offset =
                                next.entry_offset_for_vertical_focus(false, preferred_x.map(px));
                            next.move_to_with_preferred_x(offset, preferred_x.map(px), cx);
                            next.start_cursor_blink(cx);
                            cx.notify();
                        });
                        self.active_entity = Some(next.clone());
                        cx.notify();
                    }
                }
            }
            BlockEvent::RequestIndent => {
                if let Some(doc) = &mut self.document {
                    let blocks = doc.blocks();
                    let current_idx = doc.index_for_entity_id(block.entity_id()).unwrap_or(0);
                    if current_idx > 0 {
                        if let Some(location) = doc.find_block_location(block.entity_id()) {
                            let target_parent = blocks[current_idx - 1].entity.clone();
                            if location.parent.as_ref().map(|p| p.entity_id())
                                != Some(target_parent.entity_id())
                            {
                                doc.remove_block(block.entity_id(), cx);
                                let child_index = target_parent.read(cx).children.len();
                                doc.insert_blocks_at(
                                    Some(target_parent.clone()),
                                    child_index,
                                    vec![block.clone()],
                                    cx,
                                );
                                self.active_entity = Some(block.clone());
                                self.pending_edit = true;
                                self.commit_document_edit(false, cx);
                                cx.notify();
                            }
                        }
                    }
                }
            }
            BlockEvent::RequestOutdent => {
                if let Some(doc) = &mut self.document {
                    if let Some(location) = doc.find_block_location(block.entity_id()) {
                        if let Some(parent) = location.parent.clone() {
                            if let Some(parent_location) =
                                doc.find_block_location(parent.entity_id())
                            {
                                doc.remove_block(block.entity_id(), cx);
                                doc.insert_blocks_at(
                                    parent_location.parent,
                                    parent_location.index + 1,
                                    vec![block.clone()],
                                    cx,
                                );
                                self.active_entity = Some(block.clone());
                            }
                        } else {
                            block.update(cx, |b, cx| b.convert_to_paragraph(cx));
                        }
                        self.pending_edit = true;
                        self.commit_document_edit(false, cx);
                        cx.notify();
                    }
                }
            }
            BlockEvent::RequestToggleTaskChecked => {
                block.update(cx, |b, cx| {
                    let checked = match b.kind() {
                        BlockKind::TaskListItem { checked } => checked,
                        _ => return,
                    };
                    b.data.kind = BlockKind::TaskListItem { checked: !checked };
                    b.sync_edit_mode_from_kind();
                    b.sync_render_cache();
                    b.cursor_blink_epoch = std::time::Instant::now();
                    cx.notify();
                });
                self.pending_edit = true;
                self.commit_document_edit(false, cx);
                cx.notify();
            }
            _ => {}
        }
    }

    pub fn sync_document(
        &mut self,
        document: &editor_contracts::DocumentSnapshot,
        cx: &mut Context<Self>,
    ) {
        let next_base_dir = document
            .base_dir
            .as_deref()
            .map(std::path::Path::to_path_buf);
        let base_dir_changed = self.references.base_dir != next_base_dir;
        self.references.base_dir = next_base_dir;

        if self.synced_revision == Some(document.revision) && self.document.is_some() {
            if base_dir_changed {
                self.sync_reference_context(cx);
            }
            return;
        }
        if self.pending_edit {
            self.synced_revision = Some(document.revision);
            self.pending_edit = false;
            if base_dir_changed {
                self.sync_reference_context(cx);
            }
            return;
        }
        self.rebuild_from_markdown(&document.text, document.revision, cx);
        if let Some(hint) = document.restore_cursor {
            self.restore_cursor_hint(hint, cx);
        }
    }

    pub fn document_text(&self, cx: &App) -> Option<String> {
        self.document.as_ref().map(|d| d.serialize_markdown(cx))
    }

    /// Current caret position of the active block as a document-level
    /// cursor hint (1-based line/column in the serialized Markdown).
    pub fn cursor_hint(&self, cx: &App) -> CursorHint {
        let Some(doc) = &self.document else {
            return CursorHint::new(1, 1);
        };
        let Some(active) = &self.active_entity else {
            return CursorHint::new(1, 1);
        };
        let doc_text = doc.serialize_markdown(cx);
        let markdown_offset = doc
            .markdown_offset_of_block(active.entity_id(), cx)
            .map(|block_start| {
                let block = active.read(cx);
                let caret = block.cursor_offset();
                let intra = block.display_range_to_source_range(caret..caret);
                let block_markdown_len = block.data.text.serialize_markdown().len();
                block_start + intra.start.min(block_markdown_len)
            })
            .unwrap_or(0);
        CursorHint::from_offset(&doc_text, markdown_offset)
    }

    /// Moves the active caret to the document position described by a
    /// cursor hint (used to apply `restore_cursor` after undo/redo).
    pub fn restore_cursor_hint(&mut self, hint: CursorHint, cx: &mut Context<Self>) {
        let Some(doc) = &self.document else {
            return;
        };
        let doc_text = doc.serialize_markdown(cx);
        let target_offset = hint.to_offset(&doc_text);

        // Pick the last block starting at or before the target offset.
        let blocks = doc.blocks();
        let mut best: Option<(Entity<Block>, usize)> = None;
        for entry in blocks.iter() {
            if let Some(block_start) = doc.markdown_offset_of_block(entry.entity.entity_id(), cx) {
                if block_start <= target_offset
                    && best
                        .as_ref()
                        .map(|(_, start)| block_start >= *start)
                        .unwrap_or(true)
                {
                    best = Some((entry.entity.clone(), block_start));
                }
            }
        }
        let Some((block, block_start)) = best else {
            return;
        };
        let intra = target_offset.saturating_sub(block_start);
        self.active_entity = Some(block.clone());
        block.update(cx, |b, cx| {
            let display = b.source_range_to_display_range(intra..intra);
            let caret = display.start.min(b.display_len());
            b.selected_range = caret..caret;
            b.selection_reversed = false;
            b.marked_range = None;
            b.start_cursor_blink(cx);
            cx.notify();
        });
        cx.notify();
    }

    /// Commits the current document as one edit transaction.
    ///
    /// `merge` marks continuation of the previous undo transaction (typing
    /// runs). The caret hints anchor the buffer-level undo/redo restore.
    pub fn commit_document_edit(&mut self, merge: bool, cx: &mut App) {
        if let Some(edit) = self.take_edit_transaction(merge, cx) {
            if let Some(host) = self.host.clone() {
                host.commit_edit(edit, cx);
            }
        }
    }

    /// Builds the edit transaction for the current document state and
    /// updates the typing-run bookkeeping. Used both by direct commits and
    /// by contract methods that hand the transaction to the editor.
    pub fn take_edit_transaction(&mut self, merge: bool, cx: &App) -> Option<EditTransaction> {
        let text = self.document_text(cx)?;
        let cursor_after = self.cursor_hint(cx);
        let cursor_before = if merge {
            self.typing_run_start_hint.unwrap_or(cursor_after)
        } else {
            self.last_cursor_hint.unwrap_or(cursor_after)
        };

        if merge {
            if self.typing_run_start_hint.is_none() {
                self.typing_run_start_hint = self.last_cursor_hint;
            }
            self.last_typing_insert_at = self.single_char_insert_at(&text);
        } else {
            self.typing_run_start_hint = self.last_cursor_hint;
            self.last_typing_insert_at = None;
        }

        self.last_cursor_hint = Some(cursor_after);
        self.last_committed_text = Some(text.clone());
        Some(EditTransaction::new(
            text,
            merge,
            cursor_before,
            cursor_after,
        ))
    }

    /// Whether committing `new_text` continues the previous typing run: a
    /// single-character insertion at exactly the previous insertion point,
    /// or an update while an IME composition is active.
    fn is_typing_continuation(&self, new_text: &str, cx: &App) -> bool {
        if let Some(active) = &self.active_entity {
            if active.read(cx).marked_range.is_some() {
                return true;
            }
        }
        let Some(insert_at) = self.last_typing_insert_at else {
            return false;
        };
        self.single_char_insert_at(new_text) == Some(insert_at)
    }

    /// Insert position when `new_text` is a single-character insertion into
    /// the previously committed text.
    fn single_char_insert_at(&self, new_text: &str) -> Option<usize> {
        let old_text = self.last_committed_text.as_ref()?;
        if new_text.len() != old_text.len() + 1 {
            return None;
        }
        let old = old_text.as_bytes();
        let new = new_text.as_bytes();
        let mut prefix = 0;
        while prefix < old.len() && prefix < new.len() && old[prefix] == new[prefix] {
            prefix += 1;
        }
        if new_text.is_char_boundary(prefix)
            && prefix < new.len()
            && old[prefix..] == new[prefix + 1..]
        {
            Some(prefix)
        } else {
            None
        }
    }

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

    pub fn render(
        &mut self,
        ctx: &PaneRenderContext,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.host = Some(ctx.host.clone());

        let theme = cx.global::<theme::ThemeManager>().current_arc();
        let d = &theme.dimensions;
        let c = &theme.colors;
        let pane_id = ctx.pane_id;

        if let Some(doc) = &self.document {
            let blocks = doc.blocks();
            let plans = crate::render::viewport::plan_document_rows(blocks, d, cx);
            let centered_width = crate::render::layout::centered_column_width(
                f32::from(ctx.scroll.bounds().size.width).max(600.0),
                d,
            );

            let row_elements: Vec<AnyElement> = plans
                .iter()
                .map(|plan| {
                    crate::render::viewport::build_planned_row_element(
                        plan,
                        blocks,
                        centered_width,
                        &theme,
                        d,
                        |row: Div, _id| row,
                    )
                })
                .collect();

            let headings = self.outline_headings(cx);
            let scroll_y = -f32::from(ctx.scroll.offset().y);
            let active_index = headings
                .iter()
                .rposition(|node| {
                    let node_y = self.calculate_block_scroll_offset(node.block_index, &theme, cx);
                    node_y <= scroll_y + 30.0
                })
                .or(if headings.is_empty() { None } else { Some(0) });

            let outline_host: std::sync::Arc<dyn editor_contracts::OutlineHost> =
                std::sync::Arc::new(PaneOutlineHost {
                    pane_id: ctx.pane_id,
                    host: ctx.host.clone(),
                });

            let outline_hud = ui::render_floating_outline_hud(
                ctx.pane_id.0,
                &headings,
                active_index,
                ctx.is_outline_hovered,
                &theme,
                &outline_host,
            );

            div()
                .id(ElementId::Name(
                    format!("tiled-wysiwyg-editor-{pane_id}").into(),
                ))
                .key_context("Wysiwyg")
                .w_full()
                .h_full()
                .relative()
                .bg(c.editor_background)
                .child(
                    div()
                        .id(ElementId::Name(
                            format!("tiled-wysiwyg-scroll-{pane_id}").into(),
                        ))
                        .w_full()
                        .h_full()
                        .flex()
                        .flex_col()
                        .items_center()
                        .overflow_y_scroll()
                        .track_scroll(ctx.scroll)
                        .p(px(d.editor_padding))
                        .pb(px(d.editor_padding + 200.0))
                        .children(row_elements),
                )
                .child(outline_hud)
                .into_any_element()
        } else {
            div().into_any_element()
        }
    }
}
