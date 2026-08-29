//! Text editing events handler: newlines, tabs, inline formatting, and edit modes.

use gpui::*;

use editor_wysiwyg::document::protocol::BlockEvent;
use crate::editor::engine::controller::*;

impl Editor {
    pub(crate) fn on_text_edit_event(
        &mut self,
        block: &Entity<editor_wysiwyg::document::block::Block>,
        event: &BlockEvent,
        current_entry_index: usize,
        entries_before: &[editor_wysiwyg::document::BlockEntry],
        cx: &mut Context<Self>,
    ) {
        match event {
            BlockEvent::RequestNewline {
                trailing,
                source_already_mutated,
            } => {
                // Typing a setext underline (`=====`/`-----`) under a paragraph
                // and pressing Enter turns that paragraph into a heading, the
                // same way the importer treats the two adjacent lines.
                if self.try_form_setext_heading_on_newline(block, cx) {
                    return;
                }
                // Typing a delimiter row under a header forms a native table,
                // and typing further pipe rows below the table absorbs them.
                if self.try_form_or_extend_table_on_newline(block, cx) {
                    return;
                }
                let Some(location) = self.doc().find_block_location(block.entity_id()) else {
                    return;
                };
                if !source_already_mutated {
                    self.prepare_undo_capture(
                        editor_wysiwyg::document::protocol::UndoCaptureKind::NonCoalescible,
                        cx,
                    );
                }
                let current_kind = block.read(cx).kind();
                let new_block = Self::new_block(
                    cx,
                    BlockData::new(current_kind.newline_sibling_kind(), trailing.clone()),
                );
                if self.is_source_code() {
                    new_block.update(cx, |block, _cx| block.set_source_document_mode());
                }
                self.doc_mut().insert_blocks_at(
                    location.parent,
                    location.index + 1,
                    vec![new_block.clone()],
                    cx,
                );
                self.rebuild_reference_registries(cx);
                self.focus_block(new_block.entity_id());
                if current_kind.is_quote_container() {
                    self.normalize_rendered_quote_structure(cx);
                }
                self.mark_dirty(cx);
                self.finalize_pending_undo_capture(cx);
                cx.notify();
            }
            BlockEvent::RequestNewlineAbove => {
                let Some(location) = self.doc().find_block_location(block.entity_id()) else {
                    return;
                };
                self.prepare_undo_capture(
                    editor_wysiwyg::document::protocol::UndoCaptureKind::NonCoalescible,
                    cx,
                );
                let new_block = Self::new_block(
                    cx,
                    BlockData::new(BlockKind::Paragraph, BlockText::plain(String::new())),
                );
                if self.is_source_code() {
                    new_block.update(cx, |block, _cx| block.set_source_document_mode());
                }
                self.doc_mut().insert_blocks_at(
                    location.parent,
                    location.index,
                    vec![new_block],
                    cx,
                );
                self.rebuild_reference_registries(cx);
                self.focus_block(block.entity_id());
                self.mark_dirty(cx);
                self.finalize_pending_undo_capture(cx);
                cx.notify();
            }
            BlockEvent::RequestMergeIntoPrevious { content } => {
                if current_entry_index == 0 {
                    return;
                }
                let prev = entries_before[current_entry_index - 1].entity.clone();
                let quote_related = self.is_block_quote_structure_related(block, cx)
                    || self.is_block_quote_structure_related(&prev, cx);
                self.prepare_undo_capture(
                    editor_wysiwyg::document::protocol::UndoCaptureKind::NonCoalescible,
                    cx,
                );

                let cursor_pos = prev.read(cx).display_text().len();
                let current_content = content.clone();
                prev.update(cx, move |prev, cx| {
                    let mut text = prev.data.text.clone();
                    text.append(current_content);
                    prev.data.set_text(text);
                    prev.sync_render_cache();
                    prev.cursor_blink_epoch = Instant::now();
                    cx.notify();
                });
                Self::reset_block_cursor(&prev, cursor_pos, cx);

                let is_task_list_item = block.read(cx).kind().is_task_list_item();
                let adopted_children =
                    editor_wysiwyg::document::Document::take_children(block, cx);
                let removed = self.doc_mut().with_structure_mutation(cx, |document, cx| {
                    let (_, location) = document.remove_block_unindexed(block.entity_id(), cx)?;
                    if !adopted_children.is_empty() {
                        let insert_parent = if is_task_list_item {
                            Some(prev.clone())
                        } else {
                            location.parent
                        };
                        let insert_index = if is_task_list_item {
                            prev.read(cx).children.len()
                        } else {
                            location.index
                        };
                        document.insert_blocks_unindexed(
                            insert_parent,
                            insert_index,
                            adopted_children.clone(),
                            cx,
                        );
                    }
                    Some(())
                });

                if removed.is_none() {
                    return;
                }

                self.focus_block(prev.entity_id());
                self.rebuild_reference_registries(cx);
                if quote_related {
                    self.normalize_rendered_quote_structure(cx);
                }
                self.mark_dirty(cx);
                self.finalize_pending_undo_capture(cx);
                cx.notify();
            }
            BlockEvent::RequestPasteMultiline {
                leading,
                lines,
                trailing,
                split_physical_lines,
            } => {
                if lines.is_empty() {
                    return;
                }
                let quote_related = self.is_block_quote_structure_related(block, cx);
                self.prepare_undo_capture(
                    editor_wysiwyg::document::protocol::UndoCaptureKind::NonCoalescible,
                    cx,
                );

                let current_kind = block.read(cx).kind();
                // Structural Markdown (tables, fences, containers) must be parsed
                // as whole blocks. The plain-text path folds the first pasted line
                // into the current paragraph, which would strip a table's header
                // row, so structural pastes hand every line to the importer and
                // leave the pre-cursor text in place.
                let structural = !*split_physical_lines;
                let leading_empty = leading.plain_len() == 0;
                let (mut first_text, tail_lines) = if structural {
                    (leading.clone(), lines.clone())
                } else {
                    let mut first_text = leading.clone();
                    first_text.append(BlockText::from_markdown(&lines[0]));
                    (first_text, lines[1..].to_vec())
                };
                if tail_lines.is_empty() {
                    first_text.append(trailing.clone());
                    let cursor = first_text.plain_len();
                    Self::set_block_text_and_kind(block, current_kind, first_text, cursor, cx);
                    self.focus_block(block.entity_id());
                    if quote_related {
                        self.normalize_rendered_quote_structure(cx);
                    } else {
                        self.rebuild_reference_registries(cx);
                    }
                    self.mark_dirty(cx);
                    self.finalize_pending_undo_capture(cx);
                    cx.notify();
                    return;
                }

                let cursor = first_text.plain_len();
                Self::set_block_text_and_kind(block, current_kind, first_text, cursor, cx);

                let Some(location) = self.doc().find_block_location(block.entity_id()) else {
                    return;
                };

                // Physical-line paste is for plain rendered text snippets. If
                // the classifier saw structural Markdown, delegate the tail to
                // the normal importer so tables, fences, and containers stay
                // intact instead of becoming paragraphs.
                let mut inserted_roots = if *split_physical_lines {
                    Self::build_plain_paste_blocks_from_lines(cx, &tail_lines)
                } else {
                    Self::build_wysiwyg_blocks_from_lines(cx, &tail_lines)
                };
                if structural && trailing.plain_len() > 0 {
                    inserted_roots.push(Self::new_block(cx, BlockData::paragraph(String::new())));
                }
                self.doc_mut().insert_blocks_at(
                    location.parent,
                    location.index + 1,
                    inserted_roots.clone(),
                    cx,
                );
                self.rebuild_table_grids(cx);

                // A structural block pasted at the very end of the document leaves
                // no line below it; remember that so a trailing paragraph can be
                // added once the paste (and any quote normalization) settles.
                let inserted_at_doc_end = inserted_roots.last().is_some_and(|last| {
                    self.doc()
                        .find_block_location(last.entity_id())
                        .is_some_and(|location| {
                            location.parent.is_none()
                                && location.index + 1 >= self.doc().root_count()
                        })
                });

                if let Some(last_root) = inserted_roots.last() {
                    let focus_block = if last_root.read(cx).kind() == BlockKind::Table {
                        last_root.read(cx).table_grid.as_ref().and_then(|grid| {
                            grid.rows
                                .last()
                                .and_then(|row| row.last())
                                .cloned()
                                .or_else(|| grid.header.last().cloned())
                        })
                    } else {
                        self.doc().last_descendant(last_root.entity_id())
                    };
                    let Some(focus_block) = focus_block else {
                        return;
                    };
                    focus_block.update(cx, {
                        let trailing = trailing.clone();
                        move |focus_block, cx| {
                            let mut next_text = focus_block.data.text.clone();
                            next_text.append(trailing.clone());
                            focus_block.data.set_text(next_text);
                            focus_block.sync_render_cache();
                            focus_block.cursor_blink_epoch = Instant::now();
                            cx.notify();
                        }
                    });
                    let cursor = focus_block.read(cx).display_text().len();
                    Self::reset_block_cursor(&focus_block, cursor, cx);
                    self.rebuild_reference_registries(cx);
                    if let Some(binding) = self.table_cell_binding(focus_block.entity_id()) {
                        self.sync_table_data_from_grid(&binding.table_block, cx);
                    }
                    self.focus_block(focus_block.entity_id());
                }

                // When structural content is pasted onto an empty line there is
                // no pre-cursor text to keep, so drop the now-empty paragraph
                // rather than leaving a blank line above the pasted blocks.
                if structural && leading_empty {
                    self.doc_mut().with_structure_mutation(cx, |document, cx| {
                        document.remove_block_unindexed(block.entity_id(), cx);
                    });
                }

                if quote_related {
                    self.normalize_rendered_quote_structure(cx);
                }

                // Quote normalization rebuilds roots from Markdown, so resolve the
                // landing block from the live tree rather than the pasted handles.
                if inserted_at_doc_end {
                    if let Some(last_root) = self.doc().root_blocks().last().cloned() {
                        self.ensure_trailing_paragraph_after_structural(&last_root, cx);
                    }
                }
                self.mark_dirty(cx);
                self.finalize_pending_undo_capture(cx);
                cx.notify();
            }
            _ => {}
        }
    }
}
