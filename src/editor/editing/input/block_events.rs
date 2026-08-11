//! Block-originated event routing: every `BlockAction` emitted by a block
//! is dispatched here against the cached entry-order snapshot.

use gpui::*;

use crate::editor::block_protocol::BlockAction;
use crate::editor::controller::*;

impl Editor {
    /// Whether the given action should dismiss any active cross-block text
    /// selection (typing, structural edits, and paste all replace it).
    pub(crate) fn block_event_clears_cross_block_selection(event: &BlockAction) -> bool {
        matches!(
            event,
            BlockAction::Changed
                | BlockAction::RequestNewline { .. }
                | BlockAction::RequestNewlineAbove
                | BlockAction::RequestEnterCalloutBody
                | BlockAction::RequestQuoteBreak
                | BlockAction::RequestCalloutBreak
                | BlockAction::RequestMergeIntoPrev { .. }
                | BlockAction::RequestPasteMultiline { .. }
                | BlockAction::RequestPasteImage { .. }
                | BlockAction::RequestIndent
                | BlockAction::RequestOutdent
                | BlockAction::RequestDowngradeNestedListItemToChildParagraph
                | BlockAction::ToggleTaskChecked
                | BlockAction::RequestAppendTableColumn
                | BlockAction::RequestAppendTableRow
                | BlockAction::RequestExpandTable
                | BlockAction::RequestDelete
        )
    }

    pub(crate) fn on_block_event(
        &mut self,
        block: Entity<crate::editor::tree::block::Block>,
        event: &BlockAction,
        cx: &mut Context<Self>,
    ) {
        if let BlockAction::PrepareUndo { kind } = event {
            self.prepare_undo_capture_from_stable_snapshot(*kind);
            return;
        }

        if let BlockAction::RequestReplaceCrossBlockSelection {
            text,
            selected_range_relative,
            mark_inserted_text,
            undo_kind,
        } = event
            && self.replace_cross_block_selection_with_text(
                text,
                selected_range_relative.clone(),
                *mark_inserted_text,
                *undo_kind,
                cx,
            )
        {
            return;
        }

        if matches!(event, BlockAction::RequestRenderedSelectAll) {
            self.on_rendered_select_all_press(block, cx);
            return;
        }

        if let BlockAction::RequestPasteImage {
            leading,
            source,
            trailing,
        } = event
        {
            self.handle_paste_image_request(block, leading, source, trailing, cx);
            return;
        }

        if let Some(binding) = self.table_cell_binding(block.entity_id()) {
            self.on_table_cell_event(binding, event, cx);
            return;
        }

        if Self::block_event_clears_cross_block_selection(event) {
            self.tab_mut().selection.select_all_cycle = None;
            self.clear_cross_block_selection(cx);
        }

        let entries_before = self.doc().flatten_entries();
        let current_entry_index = entries_before
            .iter()
            .position(|entry| entry.entity.entity_id() == block.entity_id())
            .unwrap_or(0);

        match event {
            BlockAction::Changed => {
                let should_restart_numbered_list = block.update(cx, |block, _cx| {
                    block.take_numbered_list_restart_requested()
                });
                if should_restart_numbered_list {
                    self.insert_list_group_separator_before(block.entity_id(), cx);
                }

                let callout_focus_target = self.materialize_empty_callout_shortcut(&block, cx);

                let should_normalize_quote =
                    block.update(cx, |block, _cx| {
                        let requested = block.take_quote_reparse_requested();
                        requested && block.marked_range.is_none()
                    }) || Self::rendered_quote_text_requires_reparse(&block, cx);

                self.refresh_rendered_quote_metadata_if_needed(&block, cx);
                if should_normalize_quote {
                    self.normalize_rendered_quote_structure(cx);
                } else {
                    self.sync_references_after_block_change(&block, cx);
                }
                if let Some(focus_id) = callout_focus_target {
                    self.focus_block(focus_id);
                }
                self.mark_dirty(cx);
                self.request_active_block_scroll_into_view(cx);
                self.finalize_pending_undo_capture(cx);
            }
            BlockAction::RequestNewline {
                trailing,
                source_already_mutated,
            } => {
                // Typing a setext underline (`=====`/`-----`) under a paragraph
                // and pressing Enter turns that paragraph into a heading, the
                // same way the importer treats the two adjacent lines.
                if self.try_form_setext_heading_on_newline(&block, cx) {
                    return;
                }
                // Typing a delimiter row under a header forms a native table,
                // and typing further pipe rows below the table absorbs them.
                if self.try_form_or_extend_table_on_newline(&block, cx) {
                    return;
                }
                let Some(location) = self.doc().find_block_location(block.entity_id()) else {
                    return;
                };
                if !source_already_mutated {
                    self.prepare_undo_capture(
                        crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
                        cx,
                    );
                }
                let current_kind = block.read(cx).kind();
                let new_block = Self::new_block(
                    cx,
                    BlockData::new(current_kind.newline_sibling_kind(), trailing.clone()),
                );
                if self.tab().mode == crate::editor::controller::EditorMode::SourceCode {
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
            BlockAction::RequestNewlineAbove => {
                let Some(location) = self.doc().find_block_location(block.entity_id()) else {
                    return;
                };
                self.prepare_undo_capture(
                    crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
                    cx,
                );
                let new_block = Self::new_block(
                    cx,
                    BlockData::new(BlockKind::Paragraph, RichText::plain(String::new())),
                );
                if self.tab().mode == crate::editor::controller::EditorMode::SourceCode {
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
            BlockAction::RequestEnterCalloutBody => {
                let needs_body = block.read(cx).children.is_empty();
                if needs_body {
                    self.prepare_undo_capture(
                        crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
                        cx,
                    );
                }
                let created = self.ensure_callout_body_entry(&block, cx);
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
            BlockAction::RequestQuoteBreak => {
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
                    BlockData::new(BlockKind::Blockquote, RichText::plain(String::new())),
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
            BlockAction::RequestCalloutBreak => {
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
            BlockAction::RequestMergeIntoPrev { content } => {
                if current_entry_index == 0 {
                    return;
                }
                let prev = entries_before[current_entry_index - 1].entity.clone();
                let quote_related = self.block_is_quote_structure_related(&block, cx)
                    || self.block_is_quote_structure_related(&prev, cx);
                self.prepare_undo_capture(
                    crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
                    cx,
                );

                let cursor_pos = prev.read(cx).display_text().len();
                let adopted_children =
                    crate::editor::tree::document::Document::take_children(&block, cx);
                let removed_entity_id = block.entity_id();

                self.doc_mut().with_structure_mutation(cx, |document, cx| {
                    prev.update(cx, {
                        let content = content.clone();
                        let adopted_children = adopted_children.clone();
                        move |prev, cx| {
                            let mut next_text = prev.record.text.clone();
                            next_text.append_tree(content.clone());
                            prev.record.set_text(next_text);
                            prev.sync_render_cache();
                            prev.children.extend(adopted_children.clone());
                            prev.selected_range = cursor_pos..cursor_pos;
                            prev.selection_reversed = false;
                            prev.marked_range = None;
                            prev.vertical_motion_x = None;
                            prev.cursor_blink_epoch = Instant::now();
                            cx.notify();
                        }
                    });
                    let _ = document.remove_block_by_id_raw(removed_entity_id, cx);
                });

                self.focus_block(prev.entity_id());
                if quote_related {
                    self.normalize_rendered_quote_structure(cx);
                } else {
                    self.rebuild_reference_registries(cx);
                }
                self.mark_dirty(cx);
                self.finalize_pending_undo_capture(cx);
                cx.notify();
            }
            BlockAction::RequestPasteMultiline {
                leading,
                lines,
                trailing,
                split_physical_lines,
            } => {
                if lines.is_empty() {
                    return;
                }
                let quote_related = self.block_is_quote_structure_related(&block, cx);
                self.prepare_undo_capture(
                    crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
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
                    first_text.append_tree(RichText::from_markdown(&lines[0]));
                    (first_text, lines[1..].to_vec())
                };
                if tail_lines.is_empty() {
                    first_text.append_tree(trailing.clone());
                    let cursor = first_text.plain_len();
                    Self::set_block_text_and_kind(&block, current_kind, first_text, cursor, cx);
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
                Self::set_block_text_and_kind(&block, current_kind, first_text, cursor, cx);

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
                    Self::build_blocks_from_lines(cx, &tail_lines)
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
                            let mut next_text = focus_block.record.text.clone();
                            next_text.append_tree(trailing.clone());
                            focus_block.record.set_text(next_text);
                            focus_block.sync_render_cache();
                            focus_block.cursor_blink_epoch = Instant::now();
                            cx.notify();
                        }
                    });
                    let cursor = focus_block.read(cx).display_text().len();
                    Self::reset_block_cursor(&focus_block, cursor, cx);
                    self.rebuild_reference_registries(cx);
                    if let Some(binding) = self.table_cell_binding(focus_block.entity_id()) {
                        self.sync_table_record_from_grid(&binding.table_block, cx);
                    }
                    self.focus_block(focus_block.entity_id());
                }

                // When structural content is pasted onto an empty line there is
                // no pre-cursor text to keep, so drop the now-empty paragraph
                // rather than leaving a blank line above the pasted blocks.
                if structural && leading_empty {
                    self.doc_mut().with_structure_mutation(cx, |document, cx| {
                        document.remove_block_by_id_raw(block.entity_id(), cx);
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
            BlockAction::RequestPasteImage { .. }
            | BlockAction::RequestReplaceCrossBlockSelection { .. } => {}
            BlockAction::RequestIndent => {
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
                    let moved = document.remove_block_by_id_raw(block.entity_id(), cx)?.0;
                    let child_index = target_parent.read(cx).children.len();
                    document.insert_blocks_at_raw(
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
            BlockAction::RequestOutdent => {
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
                        let moved = document.remove_block_by_id_raw(block.entity_id(), cx)?.0;
                        document.insert_blocks_at_raw(
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
            BlockAction::RequestDowngradeNestedListItemToChildParagraph => {
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
                        document.remove_block_by_id_raw(block.entity_id(), cx)?;
                    moved.update(cx, |block, cx| {
                        block.record.kind = BlockKind::Paragraph;
                        block.record.raw_source = None;
                        block.sync_edit_mode_from_kind();
                        block.sync_render_cache();
                        block.cursor_blink_epoch = Instant::now();
                        cx.notify();
                    });
                    document.insert_blocks_at_raw(
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
            BlockAction::ToggleTaskChecked => {
                self.prepare_undo_capture(
                    crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
                    cx,
                );
                block.update(cx, |block, cx| {
                    let checked = match block.kind() {
                        BlockKind::TaskListItem { checked } => checked,
                        _ => return,
                    };
                    block.record.kind = BlockKind::TaskListItem { checked: !checked };
                    block.sync_edit_mode_from_kind();
                    block.sync_render_cache();
                    block.cursor_blink_epoch = Instant::now();
                    cx.notify();
                });
                self.mark_dirty(cx);
                self.request_active_block_scroll_into_view(cx);
                self.finalize_pending_undo_capture(cx);
                cx.notify();
            }
            BlockAction::RequestOpenLink {
                prompt_target,
                open_target,
            } => {
                self.request_open_link_prompt(prompt_target.clone(), open_target.clone(), cx);
            }
            BlockAction::RequestJumpToFootnoteDefinition { id, .. } => {
                let _ = self.jump_to_footnote_definition(id, cx);
                cx.notify();
            }
            BlockAction::RequestJumpToFootnoteBackref { id } => {
                let _ = self.jump_to_footnote_backref(id, cx);
                cx.notify();
            }
            BlockAction::RequestAppendTableColumn => {
                if block.read(cx).kind() == BlockKind::Table {
                    self.prepare_undo_capture(
                        crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
                        cx,
                    );
                    self.append_table_column(&block, cx);
                    self.finalize_pending_undo_capture(cx);
                }
            }
            BlockAction::RequestAppendTableRow => {
                if block.read(cx).kind() == BlockKind::Table {
                    self.prepare_undo_capture(
                        crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
                        cx,
                    );
                    self.append_table_row(&block, cx);
                    self.finalize_pending_undo_capture(cx);
                }
            }
            BlockAction::RequestExpandTable => {
                if block.read(cx).kind() == BlockKind::Table {
                    self.prepare_undo_capture(
                        crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
                        cx,
                    );
                    self.expand_table_block(&block, cx);
                    self.finalize_pending_undo_capture(cx);
                }
            }
            BlockAction::RequestTableAxisPreview {
                kind,
                index,
                hovered,
            } => {
                if block.read(cx).kind() == BlockKind::Table {
                    self.preview_table_axis(block.entity_id(), *kind, *index, *hovered, cx);
                }
            }
            BlockAction::RequestSelectTableAxis { kind, index } => {
                if block.read(cx).kind() == BlockKind::Table {
                    self.select_table_axis(block.entity_id(), *kind, *index, cx);
                }
            }
            BlockAction::RequestOpenTableAxisMenu {
                kind,
                index,
                position,
            } => {
                if block.read(cx).kind() == BlockKind::Table {
                    self.open_table_axis_menu(block.entity_id(), *kind, *index, *position, cx);
                }
            }
            BlockAction::RequestTableCellMoveHorizontal { .. }
            | BlockAction::RequestTableCellMoveVertical { .. } => {}
            BlockAction::RequestFocusPrev { preferred_x } => {
                if current_entry_index == 0 {
                    return;
                }

                let target = entries_before[current_entry_index - 1].entity.clone();
                // Entering a table from below lands in a body cell instead of
                // the non-editable table container.
                if target.read(cx).kind() == BlockKind::Table
                    && self.focus_table_entry_cell(&target, false, cx)
                {
                    return;
                }
                let target_x = preferred_x.map(px);
                let offset = target
                    .read(cx)
                    .entry_offset_for_vertical_focus(true, target_x);
                self.focus_block(target.entity_id());
                target.update(cx, move |target, cx| {
                    target.move_to_with_preferred_x(offset, target_x, cx);
                });
                cx.notify();
            }
            BlockAction::RequestFocusNext { preferred_x } => {
                if current_entry_index + 1 >= entries_before.len() {
                    // A trailing multi-line block (code, math, ...) has nowhere
                    // below to move to, so give it a paragraph to land on and
                    // focus that, matching how a trailing table behaves.
                    if block.read(cx).kind().is_multiline_text_block() {
                        self.ensure_trailing_paragraph_after_structural(&block, cx);
                        let entry = self.doc().flatten_entries();
                        if let Some(landing) = entry
                            .iter()
                            .position(|v| v.entity.entity_id() == block.entity_id())
                            .and_then(|index| entry.get(index + 1))
                            .map(|v| v.entity.clone())
                        {
                            self.focus_block(landing.entity_id());
                            landing.update(cx, |landing, cx| landing.move_to(0, cx));
                            cx.notify();
                        }
                    }
                    return;
                }

                let target = entries_before[current_entry_index + 1].entity.clone();
                // Entering a table from above lands in a header cell instead of
                // the non-editable table container.
                if target.read(cx).kind() == BlockKind::Table
                    && self.focus_table_entry_cell(&target, true, cx)
                {
                    return;
                }
                let target_x = preferred_x.map(px);
                let offset = target
                    .read(cx)
                    .entry_offset_for_vertical_focus(false, target_x);
                self.focus_block(target.entity_id());
                target.update(cx, move |target, cx| {
                    target.move_to_with_preferred_x(offset, target_x, cx);
                });
                cx.notify();
            }
            BlockAction::RequestBlockUp => {
                if current_entry_index == 0 {
                    return;
                }

                let target = entries_before[current_entry_index - 1].entity.clone();
                if target.read(cx).kind() == BlockKind::Table
                    && self.focus_table_entry_cell(&target, false, cx)
                {
                    return;
                }
                self.focus_block(target.entity_id());
                target.update(cx, |target, cx| target.move_to(0, cx));
                cx.notify();
            }
            BlockAction::RequestBlockDown => {
                if current_entry_index + 1 >= entries_before.len() {
                    return;
                }

                let target = entries_before[current_entry_index + 1].entity.clone();
                if target.read(cx).kind() == BlockKind::Table
                    && self.focus_table_entry_cell(&target, true, cx)
                {
                    return;
                }
                self.focus_block(target.entity_id());
                target.update(cx, |target, cx| target.move_to(0, cx));
                cx.notify();
            }
            BlockAction::RequestDelete => {
                if self.downgrade_empty_callout_body_to_quote(&block, cx) {
                    return;
                }
                let quote_related = self.block_is_quote_structure_related(&block, cx);
                let is_last_visible_leaf =
                    entries_before.len() == 1 && block.read(cx).children.is_empty();
                if is_last_visible_leaf {
                    if block.read(cx).kind() == BlockKind::Paragraph {
                        Self::reset_block_cursor(&block, 0, cx);
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
                    crate::editor::tree::document::Document::take_children(&block, cx);
                let removed = self.doc_mut().with_structure_mutation(cx, |document, cx| {
                    let (_, location) = document.remove_block_by_id_raw(block.entity_id(), cx)?;
                    if !adopted_children.is_empty() {
                        document.insert_blocks_at_raw(
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
            BlockAction::RequestFocus => {
                self.clear_table_axis_preview(cx);
                self.clear_table_axis_selection(cx);
                self.focus_block(block.entity_id());
                for entry in self.doc().flatten_entries() {
                    entry.entity.update(cx, |_, cx| cx.notify());
                }
                cx.notify();
            }
            BlockAction::RequestRenderedSelectAll => {}
            BlockAction::PrepareUndo { .. } => {}
        }
    }
}
