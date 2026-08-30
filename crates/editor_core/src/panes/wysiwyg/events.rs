//! WYSIWYG block event handler and router.

use gpui::*;

use crate::engine::controller::*;
use editor_wysiwyg::document::block::Block;
use editor_wysiwyg::document::protocol::BlockEvent;
use editor_wysiwyg::markdown::inline::text::BlockText;
use editor_wysiwyg::markdown::parse::{BlockData, BlockKind};

impl Editor {
    /// Subscribes this editor to all block events in the active document.
    pub fn subscribe_document_blocks(&mut self, cx: &mut Context<Self>) {
        if let Some(doc) = self.active_doc() {
            let blocks: Vec<Entity<Block>> = doc.blocks().iter().map(|e| e.entity.clone()).collect();
            for block in blocks {
                cx.subscribe(&block, Self::on_block_event).detach();
            }
        }
    }

    /// Creates a new block entity and subscribes this editor to its event stream.
    pub fn new_block(cx: &mut Context<Self>, data: BlockData) -> Entity<Block> {
        let block = cx.new(|cx| Block::with_data(cx, data));
        cx.subscribe(&block, Self::on_block_event).detach();
        block
    }

    /// Handles a block event emitted by one of the document blocks.
    pub fn on_block_event(
        &mut self,
        block: Entity<Block>,
        event: &BlockEvent,
        cx: &mut Context<Self>,
    ) {
        if let Some(binding) = self.table_cell_binding(block.entity_id()) {
            self.on_table_cell_event(binding, event, cx);
            return;
        }

        match event {
            BlockEvent::RequestFocus => {
                self.focus_wysiwyg_block(block.entity_id());
                cx.notify();
            }
            BlockEvent::Changed => {
                self.tab_mut().file.dirty = true;
                self.tab_mut().text_stale = true;
                self.request_autoscroll_active_pane(
                    crate::engine::controller::AutoscrollStrategy::Fit { margin: px(20.0) },
                    cx,
                );
                cx.notify();
            }
            BlockEvent::RequestNewline { trailing, .. } => {
                let Some(location) = self.doc().find_block_location(block.entity_id()) else {
                    return;
                };
                let current_kind = block.read(cx).kind();
                let new_block = Self::new_block(
                    cx,
                    BlockData::new(current_kind.newline_sibling_kind(), trailing.clone()),
                );
                self.doc_mut().insert_blocks_at(
                    location.parent,
                    location.index + 1,
                    vec![new_block.clone()],
                    cx,
                );
                self.focus_wysiwyg_block(new_block.entity_id());
                self.tab_mut().file.dirty = true;
                self.tab_mut().text_stale = true;
                cx.notify();
            }
            BlockEvent::RequestNewlineAbove => {
                let Some(location) = self.doc().find_block_location(block.entity_id()) else {
                    return;
                };
                let new_block = Self::new_block(
                    cx,
                    BlockData::new(BlockKind::Paragraph, BlockText::plain(String::new())),
                );
                self.doc_mut().insert_blocks_at(
                    location.parent,
                    location.index,
                    vec![new_block],
                    cx,
                );
                self.focus_wysiwyg_block(block.entity_id());
                self.tab_mut().file.dirty = true;
                self.tab_mut().text_stale = true;
                cx.notify();
            }
            BlockEvent::RequestMergeIntoPrevious { content } => {
                let entries = self.doc().blocks().to_vec();
                let current_entry_index = entries
                    .iter()
                    .position(|e| e.entity.entity_id() == block.entity_id())
                    .unwrap_or(0);
                if current_entry_index > 0 {
                    let prev = entries[current_entry_index - 1].entity.clone();
                    let prev_len = prev.read(cx).display_len();
                    prev.update(cx, |prev, cx| {
                        let mut cur_text = prev.data.text.clone();
                        cur_text.append(content.clone());
                        prev.data.set_text(cur_text);
                        prev.sync_render_cache();
                        prev.assign_collapsed_selection_offset(
                            prev_len,
                            editor_wysiwyg::document::block::CollapsedCaretAffinity::Default,
                            None,
                        );
                        cx.notify();
                    });
                    self.doc_mut().remove_block(block.entity_id(), cx);
                    self.focus_wysiwyg_block(prev.entity_id());
                    self.tab_mut().file.dirty = true;
                    self.tab_mut().text_stale = true;
                    cx.notify();
                }
            }
            BlockEvent::RequestDelete => {
                let entries = self.doc().blocks().to_vec();
                let current_entry_index = entries
                    .iter()
                    .position(|e| e.entity.entity_id() == block.entity_id())
                    .unwrap_or(0);
                if entries.len() > 1 {
                    let target = if current_entry_index > 0 {
                        entries[current_entry_index - 1].entity.clone()
                    } else {
                        entries[1].entity.clone()
                    };
                    self.doc_mut().remove_block(block.entity_id(), cx);
                    self.focus_wysiwyg_block(target.entity_id());
                    self.tab_mut().file.dirty = true;
                    self.tab_mut().text_stale = true;
                    cx.notify();
                }
            }
            BlockEvent::RequestFocusPrevious { preferred_x } => {
                let entries = self.doc().blocks().to_vec();
                let current_entry_index = entries
                    .iter()
                    .position(|e| e.entity.entity_id() == block.entity_id())
                    .unwrap_or(0);
                if current_entry_index > 0 {
                    let target = entries[current_entry_index - 1].entity.clone();
                    self.focus_wysiwyg_block(target.entity_id());
                    let offset = target
                        .read(cx)
                        .entry_offset_for_vertical_focus(true, preferred_x.map(px));
                    target.update(cx, |target, cx| {
                        target.move_to_with_preferred_x(offset, preferred_x.map(px), cx);
                        cx.notify();
                    });
                    cx.notify();
                }
            }
            BlockEvent::RequestFocusNext { preferred_x } => {
                let entries = self.doc().blocks().to_vec();
                let current_entry_index = entries
                    .iter()
                    .position(|e| e.entity.entity_id() == block.entity_id())
                    .unwrap_or(0);
                if current_entry_index + 1 < entries.len() {
                    let target = entries[current_entry_index + 1].entity.clone();
                    self.focus_wysiwyg_block(target.entity_id());
                    let offset = target
                        .read(cx)
                        .entry_offset_for_vertical_focus(false, preferred_x.map(px));
                    target.update(cx, |target, cx| {
                        target.move_to_with_preferred_x(offset, preferred_x.map(px), cx);
                        cx.notify();
                    });
                    cx.notify();
                }
            }
            BlockEvent::RequestIndent => {
                let Some(location) = self.doc().find_block_location(block.entity_id()) else {
                    return;
                };
                let entries = self.doc().cloned_entries();
                let current_entry_index = self.doc().index_for_entity_id(block.entity_id()).unwrap_or(0);
                if current_entry_index == 0 {
                    return;
                }
                let target_parent = entries[current_entry_index - 1].entity.clone();
                let current_kind = block.read(cx).kind();
                if !current_kind.can_nest_under(&target_parent.read(cx).kind()) {
                    return;
                }
                if location
                    .parent
                    .as_ref()
                    .is_some_and(|parent| parent.entity_id() == target_parent.entity_id())
                {
                    return;
                }
                let moved = self.doc_mut().with_structure_mutation(cx, |document, cx| {
                    let moved = document.remove_block_unindexed(block.entity_id(), cx)?.0;
                    let child_index = target_parent.read(cx).children.len();
                    document.insert_blocks_unindexed(
                        Some(target_parent.clone()),
                        child_index,
                        vec![moved.clone()],
                        cx,
                    );
                    Some(moved)
                });
                if let Some(moved) = moved {
                    self.focus_wysiwyg_block(moved.entity_id());
                    self.tab_mut().file.dirty = true;
                    self.tab_mut().text_stale = true;
                    cx.notify();
                }
            }
            BlockEvent::RequestOutdent => {
                let Some(location) = self.doc().find_block_location(block.entity_id()) else {
                    return;
                };
                if let Some(parent) = location.parent.clone() {
                    let Some(parent_location) = self.doc().find_block_location(parent.entity_id()) else {
                        return;
                    };
                    let moved = self.doc_mut().with_structure_mutation(cx, |document, cx| {
                        let moved = document.remove_block_unindexed(block.entity_id(), cx)?.0;
                        document.insert_blocks_unindexed(
                            parent_location.parent,
                            parent_location.index + 1,
                            vec![moved.clone()],
                            cx,
                        );
                        Some(moved)
                    });
                    if let Some(moved) = moved {
                        self.focus_wysiwyg_block(moved.entity_id());
                    }
                } else {
                    block.update(cx, |block, cx| block.convert_to_paragraph(cx));
                    self.focus_wysiwyg_block(block.entity_id());
                }
                self.tab_mut().file.dirty = true;
                self.tab_mut().text_stale = true;
                cx.notify();
            }
            BlockEvent::RequestDowngradeNestedListItemToChildParagraph => {
                let Some(location) = self.doc().find_block_location(block.entity_id()) else {
                    return;
                };
                let Some(parent) = location.parent.clone() else {
                    return;
                };
                if !block.read(cx).kind().is_list_item() || !parent.read(cx).kind().is_list_item() {
                    return;
                }
                let downgraded = self.doc_mut().with_structure_mutation(cx, |document, cx| {
                    let (moved, removed_location) =
                        document.remove_block_unindexed(block.entity_id(), cx)?;
                    moved.update(cx, |block, cx| {
                        block.data.kind = BlockKind::Paragraph;
                        block.data.raw_source = None;
                        block.sync_edit_mode_from_kind();
                        block.sync_render_cache();
                        block.cursor_blink_epoch = std::time::Instant::now();
                        cx.notify();
                    });
                    document.insert_blocks_unindexed(
                        Some(parent.clone()),
                        removed_location.index,
                        vec![moved.clone()],
                        cx,
                    );
                    Some(moved)
                });
                if let Some(downgraded) = downgraded {
                    self.focus_wysiwyg_block(downgraded.entity_id());
                    self.tab_mut().file.dirty = true;
                    self.tab_mut().text_stale = true;
                    cx.notify();
                }
            }
            BlockEvent::RequestToggleTaskChecked => {
                block.update(cx, |block, cx| {
                    if let BlockKind::TaskListItem { checked } = block.kind() {
                        block.data.kind = BlockKind::TaskListItem { checked: !checked };
                        block.sync_edit_mode_from_kind();
                        cx.notify();
                    }
                });
                self.tab_mut().file.dirty = true;
                self.tab_mut().text_stale = true;
                cx.notify();
            }
            BlockEvent::RequestOpenLink { prompt_target, open_target } => {
                self.request_open_link_prompt(prompt_target.clone(), open_target.clone(), cx);
            }
            _ => {}
        }
    }
}
