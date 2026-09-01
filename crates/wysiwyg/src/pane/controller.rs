//! Self-contained WYSIWYG document controller — autonomous block editor engine.

use std::sync::Arc;

use editor_contracts::OutlineNode;
use editor_contracts::{PaneId, PaneOutlineHost, PaneRenderContext};
use editor_contracts::{SearchMatch, SearchQuery};
use gpui::{
    AnyElement, App, AppContext, Context, Div, ElementId, Entity, FocusHandle, InteractiveElement,
    IntoElement, ParentElement, StatefulInteractiveElement, Styled, Window, div, px,
};
use theme::Theme;

use crate::model::Document;
use crate::model::block::{Block, CollapsedCaretAffinity};
use crate::model::protocol::BlockEvent;
use crate::pane::state::{FocusState, ReferenceRegistries, TableGrids};
use markdown_parser::inline::text::BlockText;
use markdown_parser::parse::{BlockData, BlockKind};

/// Autonomous controller for a WYSIWYG editor pane.
pub struct WysiwygDocumentController {
    pub pane_id: PaneId,
    pub host: Option<Arc<dyn editor_contracts::PaneHost>>,
    pub document: Option<Document>,
    pub synced_revision: Option<u64>,
    pub text_stale: bool,
    pub active_entity: Option<Entity<Block>>,
    pub focus: FocusState,
    pub tables: TableGrids,
    pub references: ReferenceRegistries,
}

impl WysiwygDocumentController {
    pub fn new(document: &editor_contracts::DocumentSnapshot, cx: &mut Context<Self>) -> Self {
        let mut controller = Self {
            pane_id: PaneId(0),
            host: None,
            document: None,
            synced_revision: None,
            text_stale: false,
            active_entity: None,
            focus: FocusState::default(),
            tables: TableGrids::default(),
            references: ReferenceRegistries {
                base_dir: document
                    .base_dir
                    .as_deref()
                    .map(std::path::Path::to_path_buf),
                ..ReferenceRegistries::default()
            },
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
        if let Some(active) = &self.active_entity {
            self.focus.active_entity = Some(active.entity_id());
        }
        self.document = Some(doc);
        self.synced_revision = Some(revision);
        self.text_stale = false;
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
                self.text_stale = true;
                if let Some(host) = &self.host {
                    if let Some(text) = self.serialize_text(cx) {
                        host.sync_source_text(self.pane_id, text, cx);
                    }
                }
                cx.notify();
            }
            BlockEvent::RequestFocus => {
                self.active_entity = Some(block.clone());
                self.focus.active_entity = Some(block.entity_id());
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
                    self.focus.active_entity = Some(new_block.entity_id());
                    new_block.update(cx, |b, cx| {
                        b.start_cursor_blink(cx);
                        cx.notify();
                    });
                    self.text_stale = true;
                    if let Some(host) = &self.host {
                        if let Some(text) = self.serialize_text(cx) {
                            host.sync_source_text(self.pane_id, text, cx);
                        }
                    }
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
                    self.focus.active_entity = Some(new_block.entity_id());
                    new_block.update(cx, |b, cx| {
                        b.start_cursor_blink(cx);
                        cx.notify();
                    });
                    self.text_stale = true;
                    if let Some(host) = &self.host {
                        if let Some(text) = self.serialize_text(cx) {
                            host.sync_source_text(self.pane_id, text, cx);
                        }
                    }
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
                        self.focus.active_entity = Some(prev.entity_id());
                        self.text_stale = true;
                        if let Some(host) = &self.host {
                            if let Some(text) = self.serialize_text(cx) {
                                host.sync_source_text(self.pane_id, text, cx);
                            }
                        }
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
                        self.focus.active_entity = Some(target.entity_id());
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
                    self.text_stale = true;
                    if let Some(host) = &self.host {
                        if let Some(text) = self.serialize_text(cx) {
                            host.sync_source_text(self.pane_id, text, cx);
                        }
                    }
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
                        self.focus.active_entity = Some(prev.entity_id());
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
                        self.focus.active_entity = Some(next.entity_id());
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
                                self.focus.active_entity = Some(block.entity_id());
                                self.text_stale = true;
                                if let Some(host) = &self.host {
                                    if let Some(text) = self.serialize_text(cx) {
                                        host.sync_source_text(self.pane_id, text, cx);
                                    }
                                }
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
                                self.focus.active_entity = Some(block.entity_id());
                            }
                        } else {
                            block.update(cx, |b, cx| b.convert_to_paragraph(cx));
                        }
                        self.text_stale = true;
                        if let Some(host) = &self.host {
                            if let Some(text) = self.serialize_text(cx) {
                                host.sync_source_text(self.pane_id, text, cx);
                            }
                        }
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
                self.text_stale = true;
                if let Some(host) = &self.host {
                    if let Some(text) = self.serialize_text(cx) {
                        host.sync_source_text(self.pane_id, text, cx);
                    }
                }
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
        if self.text_stale {
            self.synced_revision = Some(document.revision);
            self.text_stale = false;
            if base_dir_changed {
                self.sync_reference_context(cx);
            }
            return;
        }
        self.rebuild_from_markdown(&document.text, document.revision, cx);
    }

    pub fn serialize_text(&self, cx: &App) -> Option<String> {
        self.document.as_ref().map(|d| d.serialize_markdown(cx))
    }

    pub fn notify_document_changed(&mut self, cx: &mut App) {
        self.text_stale = true;
        if let Some(host) = &self.host {
            if let Some(text) = self.serialize_text(cx) {
                host.sync_source_text(self.pane_id, text, cx);
            }
        }
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
                self.focus.active_entity = Some(entity_id);
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
    ) -> Option<String> {
        if let Some(doc) = &self.document {
            if let Some(entity_id) = match_item.entity_id {
                crate::pane::search::replace_in_block_entity(
                    doc,
                    entity_id,
                    match_item.byte_range.clone(),
                    replace_with,
                    cx,
                );
                self.text_stale = true;
                cx.notify();
                return self.serialize_text(cx);
            }
        }
        None
    }

    pub fn navigate_to_search_match(&mut self, match_item: &SearchMatch, cx: &mut Context<Self>) {
        if let Some(doc) = &self.document {
            if let Some(entity_id) = match_item.entity_id {
                self.focus.active_entity = Some(entity_id);
                if let Some(block) = doc.block_entity_by_id(entity_id) {
                    self.active_entity = Some(block.clone());
                    block.update(cx, |b, cx| {
                        b.selected_range = match_item.byte_range.clone();
                        b.selection_reversed = false;
                        b.start_cursor_blink(cx);
                        cx.notify();
                    });
                }
            }
        }
    }

    pub fn apply_line_prefix(&mut self, prefix: &str, cx: &mut Context<Self>) -> Option<String> {
        if let Some(active) = self.active_entity.clone() {
            active.update(cx, |block, cx| {
                if !block.edits_verbatim_text() {
                    block.data.kind = BlockKind::Paragraph;
                }
                let cursor = block.cursor_offset();
                let text = block.display_text();
                let line_start = text[..cursor.min(text.len())]
                    .rfind('\n')
                    .map(|i| i + 1)
                    .unwrap_or(0);
                let line_end = text[cursor.min(text.len())..]
                    .find('\n')
                    .map(|i| cursor + i)
                    .unwrap_or(text.len());
                let line = &text[line_start..line_end];
                let stripped = line.trim_start_matches(|c| {
                    c == '#'
                        || c == '>'
                        || c == '-'
                        || c == '*'
                        || c == '+'
                        || c == ' '
                        || c == '\t'
                });
                let new_line = format!("{prefix}{stripped}");
                let prefix_len = prefix.len();
                block.replace_text_in_display_range(
                    line_start..line_end,
                    &new_line,
                    Some(prefix_len..prefix_len),
                    false,
                    cx,
                );
            });
            self.text_stale = true;
            cx.notify();
            self.serialize_text(cx)
        } else {
            None
        }
    }

    pub fn apply_heading_level(&mut self, level: usize, cx: &mut Context<Self>) {
        let prefix = match level {
            1 => "# ",
            2 => "## ",
            3 => "### ",
            4 => "#### ",
            5 => "##### ",
            6 => "###### ",
            _ => "",
        };
        self.apply_line_prefix(prefix, cx);
    }

    pub fn apply_snippet(
        &mut self,
        snippet: &str,
        caret_offset: usize,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        if let Some(active) = self.active_entity.clone() {
            active.update(cx, |block, cx| {
                let cursor = block.cursor_offset();
                let range = block.selected_range.clone();
                let len = snippet.len();
                let offset = caret_offset.min(len);
                if range.is_empty() {
                    block.replace_text_in_display_range(
                        cursor..cursor,
                        snippet,
                        Some(offset..offset),
                        false,
                        cx,
                    );
                } else {
                    block.replace_text_in_display_range(
                        range,
                        snippet,
                        Some(offset..offset),
                        false,
                        cx,
                    );
                }
            });
            self.text_stale = true;
            cx.notify();
            self.serialize_text(cx)
        } else {
            None
        }
    }

    pub fn apply_wrapped_or_template(
        &mut self,
        empty_template: &str,
        caret_offset_in_empty: usize,
        wrap_prefix: &str,
        wrap_suffix: &str,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        if let Some(active) = self.active_entity.clone() {
            active.update(cx, |block, cx| {
                let range = block.selected_range.clone();
                if range.is_empty() {
                    let cursor = block.cursor_offset();
                    block.replace_text_in_display_range(
                        cursor..cursor,
                        empty_template,
                        Some(caret_offset_in_empty..caret_offset_in_empty),
                        false,
                        cx,
                    );
                } else {
                    let text = block.selected_text();
                    let inner_len = text.len();
                    let replacement = format!("{wrap_prefix}{text}{wrap_suffix}");
                    let prefix_len = wrap_prefix.len();
                    block.replace_text_in_display_range(
                        range,
                        &replacement,
                        Some(prefix_len..prefix_len + inner_len),
                        false,
                        cx,
                    );
                }
            });
            self.text_stale = true;
            cx.notify();
            self.serialize_text(cx)
        } else {
            None
        }
    }

    pub fn apply_clear_format(&mut self, cx: &mut Context<Self>) -> Option<String> {
        if let Some(active) = self.active_entity.clone() {
            active.update(cx, |b, cx| {
                let range = b.selected_range.clone();
                if !range.is_empty() {
                    let (target_range, plain) = {
                        let text = b.display_text();
                        let start = range.start.min(text.len());
                        let end = range.end.min(text.len());
                        let selected = &text[start..end];
                        let plain = selected
                            .trim_matches(|c| {
                                c == '*' || c == '_' || c == '~' || c == '`' || c == '=' || c == '$'
                            })
                            .to_string();
                        (range, plain)
                    };
                    let plain_len = plain.len();
                    b.replace_text_in_display_range(
                        target_range,
                        &plain,
                        Some(0..plain_len),
                        false,
                        cx,
                    );
                }
            });
            self.text_stale = true;
            cx.notify();
            self.serialize_text(cx)
        } else {
            None
        }
    }

    pub fn render(
        &mut self,
        ctx: &PaneRenderContext,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.pane_id = ctx.pane_id;
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
