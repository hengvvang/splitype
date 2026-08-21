//! Structural mutation events handler: block split, merge, backspace demotion, and indent.

use gpui::*;

use crate::editor::block_protocol::BlockEvent;
use crate::editor::controller::*;

impl Editor {
    pub(crate) fn on_structure_event(
        &mut self,
        block: &Entity<crate::editor::tree::block::Block>,
        event: &BlockEvent,
        current_entry_index: usize,
        entries_before: &[crate::editor::tree::document::BlockEntry],
        cx: &mut Context<Self>,
    ) {
        match event {
            BlockEvent::RequestEnterCalloutBody => {
                let needs_body = block.read(cx).children.is_empty();
                if needs_body {
                    self.prepare_undo_capture(
                        crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
                        cx,
                    );
                }
                let created = self.ensure_callout_body_entry(block, cx);
                if let Some(body) = created {
                    self.focus_block(body.entity_id());
                    self.rebuild_reference_registries(cx);
                    if needs_body {
                        self.mark_dirty(cx);
                        self.finalize_pending_undo_capture(cx);
                    }
                    cx.notify();
                }
            }
            BlockEvent::RequestQuoteBreak => {
                let Some((parent, insert_index)) =
                    self.quote_break_insertion_target(block.entity_id(), cx)
                else {
                    return;
                };

                self.prepare_undo_capture(
                    crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
                    cx,
                );

                let new_quote = Self::new_block(
                    cx,
                    BlockData::new(BlockKind::Blockquote, BlockText::plain(String::new())),
                );
                let blocks = if parent.is_none() {
                    vec![new_quote.clone()]
                } else {
                    vec![
                        Self::new_block(cx, BlockData::paragraph(String::new())),
                        new_quote.clone(),
                    ]
                };
                self.doc_mut()
                    .insert_blocks_at(parent, insert_index, blocks, cx);
                self.focus_block(new_quote.entity_id());
                self.normalize_rendered_quote_structure(cx);
                self.mark_dirty(cx);
                self.finalize_pending_undo_capture(cx);
                cx.notify();
            }
            BlockEvent::RequestCalloutBreak => {
                let Some((parent, insert_index)) =
                    self.callout_break_insertion_target(block.entity_id(), cx)
                else {
                    return;
                };

                self.prepare_undo_capture(
                    crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
                    cx,
                );
                let plain = Self::new_block(cx, BlockData::paragraph(String::new()));
                let blocks = if parent.is_none() {
                    vec![plain.clone()]
                } else {
                    vec![
                        Self::new_block(cx, BlockData::paragraph(String::new())),
                        plain.clone(),
                    ]
                };
                self.doc_mut()
                    .insert_blocks_at(parent, insert_index, blocks, cx);
                self.focus_block(plain.entity_id());
                self.rebuild_reference_registries(cx);
                self.mark_dirty(cx);
                self.finalize_pending_undo_capture(cx);
                cx.notify();
            }
            BlockEvent::RequestIndent => {
                if current_entry_index == 0 {
                    return;
                }

                let Some(location) = self.doc().find_block_location(block.entity_id()) else {
                    return;
                };
                let current_kind = block.read(cx).kind();
                let target_parent = entries_before[current_entry_index - 1].entity.clone();
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
                self.prepare_undo_capture(
                    crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
                    cx,
                );

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

                let Some(moved) = moved else {
                    return;
                };

                self.focus_block(moved.entity_id());
                self.mark_dirty(cx);
                self.finalize_pending_undo_capture(cx);
                cx.notify();
            }
            BlockEvent::RequestOutdent => {
                let Some(location) = self.doc().find_block_location(block.entity_id()) else {
                    return;
                };
                self.prepare_undo_capture(
                    crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
                    cx,
                );

                if let Some(parent) = location.parent.clone() {
                    let Some(parent_location) = self.doc().find_block_location(parent.entity_id())
                    else {
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

                    let Some(moved) = moved else {
                        return;
                    };
                    self.focus_block(moved.entity_id());
                } else {
                    block.update(cx, |block, cx| block.convert_to_paragraph(cx));
                    self.focus_block(block.entity_id());
                }

                self.mark_dirty(cx);
                self.finalize_pending_undo_capture(cx);
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

                self.prepare_undo_capture(
                    crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
                    cx,
                );

                let downgraded = self.doc_mut().with_structure_mutation(cx, |document, cx| {
                    let (moved, removed_location) =
                        document.remove_block_unindexed(block.entity_id(), cx)?;
                    moved.update(cx, |block, cx| {
                        block.data.kind = BlockKind::Paragraph;
                        block.data.raw_source = None;
                        block.sync_edit_mode_from_kind();
                        block.sync_render_cache();
                        block.cursor_blink_epoch = Instant::now();
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

                let Some(downgraded) = downgraded else {
                    return;
                };

                self.focus_block(downgraded.entity_id());
                self.rebuild_reference_registries(cx);
                self.mark_dirty(cx);
                self.finalize_pending_undo_capture(cx);
                cx.notify();
            }
            BlockEvent::RequestToggleTaskChecked => {
                self.prepare_undo_capture(
                    crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
                    cx,
                );
                block.update(cx, |block, cx| {
                    let checked = match block.kind() {
                        BlockKind::TaskListItem { checked } => checked,
                        _ => return,
                    };
                    block.data.kind = BlockKind::TaskListItem { checked: !checked };
                    block.sync_edit_mode_from_kind();
                    block.sync_render_cache();
                    block.cursor_blink_epoch = Instant::now();
                    cx.notify();
                });
                self.mark_dirty(cx);
                self.request_active_block_scroll_into_view(self.active_pane_id(), cx);
                self.finalize_pending_undo_capture(cx);
                cx.notify();
            }
            BlockEvent::RequestDelete => {
                if self.downgrade_empty_callout_body_to_quote(block, cx) {
                    return;
                }
                let quote_related = self.is_block_quote_structure_related(block, cx);
                let is_last_visible_leaf =
                    entries_before.len() == 1 && block.read(cx).children.is_empty();
                if is_last_visible_leaf {
                    if block.read(cx).kind() == BlockKind::Paragraph {
                        Self::reset_block_cursor(block, 0, cx);
                    } else {
                        block.update(cx, |block, cx| block.convert_to_paragraph(cx));
                    }
                    self.focus_block(block.entity_id());
                    cx.notify();
                    return;
                }
                self.prepare_undo_capture(
                    crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
                    cx,
                );

                let entries_before_ids = entries_before
                    .iter()
                    .map(|entry| entry.entity.entity_id())
                    .collect::<Vec<_>>();
                let focus_candidate = if current_entry_index > 0 {
                    Some(entries_before_ids[current_entry_index - 1])
                } else {
                    entries_before_ids.get(current_entry_index + 1).copied()
                };

                let adopted_children =
                    crate::editor::tree::document::Document::take_children(block, cx);
                let removed = self.doc_mut().with_structure_mutation(cx, |document, cx| {
                    let (_, location) = document.remove_block_unindexed(block.entity_id(), cx)?;
                    if !adopted_children.is_empty() {
                        document.insert_blocks_unindexed(
                            location.parent.clone(),
                            location.index,
                            adopted_children.clone(),
                            cx,
                        );
                    }
                    Some(location)
                });

                if removed.is_none() {
                    return;
                }

                if let Some(focus_id) = focus_candidate {
                    self.focus_block(focus_id);
                } else if let Some(first_root) = self.doc().first_root() {
                    self.focus_block(first_root.entity_id());
                }

                if quote_related {
                    self.normalize_rendered_quote_structure(cx);
                }
                self.mark_dirty(cx);
                self.finalize_pending_undo_capture(cx);
                cx.notify();
            }
            _ => {}
        }
    }
}
