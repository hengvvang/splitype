//! Block-originated event routing: every `BlockAction` emitted by a block
//! is dispatched here against the cached visible-order snapshot.

use gpui::*;

use crate::editor::actions::BlockAction;
use crate::editor::controller::*;


impl Editor {
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
            self.selection.select_all_cycle = None;
            self.clear_cross_block_selection(cx);
        }

        let visible_before = self.document.flatten_visible_blocks();
        let current_visible_index = visible_before
            .iter()
            .position(|visible| visible.entity.entity_id() == block.entity_id())
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
                    self.rebuild_image_runtimes(cx);
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
                let Some(location) = self.document.find_block_location(block.entity_id()) else {
                    return;
                };
                if !source_already_mutated {
                    self.prepare_undo_capture(
                        crate::editor::actions::UndoCaptureKind::NonCoalescible,
                        cx,
                    );
                }
                let current_kind = block.read(cx).kind();
                let new_block = Self::new_block(
                    cx,
                    BlockData::new(current_kind.newline_sibling_kind(), trailing.clone()),
                );
                if self.mode == crate::editor::controller::EditorMode::Source {
                    new_block.update(cx, |block, _cx| block.set_source_document_mode());
                }
                self.document.insert_blocks_at(
                    location.parent,
                    location.index + 1,
                    vec![new_block.clone()],
                    cx,
                );
                self.rebuild_image_runtimes(cx);
                self.focus_block(new_block.entity_id());
                if current_kind.is_quote_container() {
                    self.normalize_rendered_quote_structure(cx);
                }
                self.mark_dirty(cx);
                self.finalize_pending_undo_capture(cx);
                cx.notify();
            }
            BlockAction::RequestNewlineAbove => {
                let Some(location) = self.document.find_block_location(block.entity_id()) else {
                    return;
                };
                self.prepare_undo_capture(
                    crate::editor::actions::UndoCaptureKind::NonCoalescible,
                    cx,
                );
                let new_block = Self::new_block(
                    cx,
                    BlockData::new(BlockKind::Paragraph, RichText::plain(String::new())),
                );
                if self.mode == crate::editor::controller::EditorMode::Source {
                    new_block.update(cx, |block, _cx| block.set_source_document_mode());
                }
                self.document.insert_blocks_at(
                    location.parent,
                    location.index,
                    vec![new_block],
                    cx,
                );
                self.rebuild_image_runtimes(cx);
                self.focus_block(block.entity_id());
                self.mark_dirty(cx);
                self.finalize_pending_undo_capture(cx);
                cx.notify();
            }
            BlockAction::RequestEnterCalloutBody => {
                let needs_body = block.read(cx).children.is_empty();
                if needs_body {
                    self.prepare_undo_capture(
                        crate::editor::actions::UndoCaptureKind::NonCoalescible,
                        cx,
                    );
                }
                let created = self.ensure_callout_body_entry(&block, cx);
                if let Some(body) = created {
                    self.focus_block(body.entity_id());
                    self.rebuild_image_runtimes(cx);
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
                    crate::editor::actions::UndoCaptureKind::NonCoalescible,
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
                self.document
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
                    crate::editor::actions::UndoCaptureKind::NonCoalescible,
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
                self.document
                    .insert_blocks_at(parent, insert_index, blocks, cx);
                self.focus_block(plain.entity_id());
                self.rebuild_image_runtimes(cx);
                self.mark_dirty(cx);
                self.finalize_pending_undo_capture(cx);
                cx.notify();
            }
            BlockAction::RequestMergeIntoPrev { content } => {
                if current_visible_index == 0 {
                    return;
                }
                let prev = visible_before[current_visible_index - 1].entity.clone();
                let quote_related = self.block_is_quote_structure_related(&block, cx)
                    || self.block_is_quote_structure_related(&prev, cx);
                self.prepare_undo_capture(
                    crate::editor::actions::UndoCaptureKind::NonCoalescible,
                    cx,
                );

                let cursor_pos = prev.read(cx).display_text().len();
                let adopted_children = crate::editor::tree::document::Document::take_children(&block, cx);
                let removed_entity_id = block.entity_id();

                self.document.with_structure_mutation(cx, |document, cx| {
                    prev.update(cx, {
                        let content = content.clone();
                        let adopted_children = adopted_children.clone();
                        move |prev, cx| {
                            let mut next_title = prev.record.text.clone();
                            next_title.append_tree(content.clone());
                            prev.record.set_text(next_title);
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
                    self.rebuild_image_runtimes(cx);
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
                    crate::editor::actions::UndoCaptureKind::NonCoalescible,
                    cx,
                );

                let current_kind = block.read(cx).kind();
                // Structural Markdown (tables, fences, containers) must be parsed
                // as whole blocks. The plain-text path folds the first pasted line
                // into the current paragraph, which would strip a table's header
                // row, so structural pastes hand every line to the importer and
                // leave the pre-cursor text in place.
                let structural = !*split_physical_lines;
                let leading_empty = leading.visible_len() == 0;
                let (mut first_title, tail_lines) = if structural {
                    (leading.clone(), lines.clone())
                } else {
                    let mut first_title = leading.clone();
                    first_title.append_tree(RichText::from_markdown(&lines[0]));
                    (first_title, lines[1..].to_vec())
                };
                if tail_lines.is_empty() {
                    first_title.append_tree(trailing.clone());
                    let cursor = first_title.visible_len();
                    Self::set_block_title_and_kind(&block, current_kind, first_title, cursor, cx);
                    self.focus_block(block.entity_id());
                    if quote_related {
                        self.normalize_rendered_quote_structure(cx);
                    } else {
                        self.rebuild_image_runtimes(cx);
                    }
                    self.mark_dirty(cx);
                    self.finalize_pending_undo_capture(cx);
                    cx.notify();
                    return;
                }

                let cursor = first_title.visible_len();
                Self::set_block_title_and_kind(&block, current_kind, first_title, cursor, cx);

                let Some(location) = self.document.find_block_location(block.entity_id()) else {
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
                if structural && trailing.visible_len() > 0 {
                    inserted_roots.push(Self::new_block(cx, BlockData::paragraph(String::new())));
                }
                self.document.insert_blocks_at(
                    location.parent,
                    location.index + 1,
                    inserted_roots.clone(),
                    cx,
                );
                self.rebuild_table_runtimes(cx);

                // A structural block pasted at the very end of the document leaves
                // no line below it; remember that so a trailing paragraph can be
                // added once the paste (and any quote normalization) settles.
                let inserted_at_doc_end = inserted_roots.last().is_some_and(|last| {
                    self.document
                        .find_block_location(last.entity_id())
                        .is_some_and(|location| {
                            location.parent.is_none()
                                && location.index + 1 >= self.document.root_count()
                        })
                });

                if let Some(last_root) = inserted_roots.last() {
                    let focus_block = if last_root.read(cx).kind() == BlockKind::Table {
                        last_root
                            .read(cx)
                            .table_runtime
                            .as_ref()
                            .and_then(|runtime| {
                                runtime
                                    .rows
                                    .last()
                                    .and_then(|row| row.last())
                                    .cloned()
                                    .or_else(|| runtime.header.last().cloned())
                            })
                    } else {
                        self.document.last_visible_descendant(last_root.entity_id())
                    };
                    let Some(focus_block) = focus_block else {
                        return;
                    };
                    focus_block.update(cx, {
                        let trailing = trailing.clone();
                        move |focus_block, cx| {
                            let mut next_title = focus_block.record.text.clone();
                            next_title.append_tree(trailing.clone());
                            focus_block.record.set_text(next_title);
                            focus_block.sync_render_cache();
                            focus_block.cursor_blink_epoch = Instant::now();
                            cx.notify();
                        }
                    });
                    let cursor = focus_block.read(cx).display_text().len();
                    Self::reset_block_cursor(&focus_block, cursor, cx);
                    self.rebuild_image_runtimes(cx);
                    if let Some(binding) = self.table_cell_binding(focus_block.entity_id()) {
                        self.sync_table_record_from_runtime(&binding.table_block, cx);
                    }
                    self.focus_block(focus_block.entity_id());
                }

                // When structural content is pasted onto an empty line there is
                // no pre-cursor text to keep, so drop the now-empty paragraph
                // rather than leaving a blank line above the pasted blocks.
                if structural && leading_empty {
                    self.document.with_structure_mutation(cx, |document, cx| {
                        document.remove_block_by_id_raw(block.entity_id(), cx);
                    });
                }

                if quote_related {
                    self.normalize_rendered_quote_structure(cx);
                }

                // Quote normalization rebuilds roots from Markdown, so resolve the
                // landing block from the live tree rather than the pasted handles.
                if inserted_at_doc_end {
                    if let Some(last_root) = self.document.root_blocks().last().cloned() {
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
                if current_visible_index == 0 {
                    return;
                }

                let Some(location) = self.document.find_block_location(block.entity_id()) else {
                    return;
                };
                let current_kind = block.read(cx).kind();
                let target_parent = visible_before[current_visible_index - 1].entity.clone();
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
                    crate::editor::actions::UndoCaptureKind::NonCoalescible,
                    cx,
                );

                let moved = self.document.with_structure_mutation(cx, |document, cx| {
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
                let Some(location) = self.document.find_block_location(block.entity_id()) else {
                    return;
                };
                self.prepare_undo_capture(
                    crate::editor::actions::UndoCaptureKind::NonCoalescible,
                    cx,
                );

                if let Some(parent) = location.parent.clone() {
                    let Some(parent_location) =
                        self.document.find_block_location(parent.entity_id())
                    else {
                        return;
                    };

                    let moved = self.document.with_structure_mutation(cx, |document, cx| {
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
                let Some(location) = self.document.find_block_location(block.entity_id()) else {
                    return;
                };
                let Some(parent) = location.parent.clone() else {
                    return;
                };
                if !block.read(cx).kind().is_list_item() || !parent.read(cx).kind().is_list_item() {
                    return;
                }

                self.prepare_undo_capture(
                    crate::editor::actions::UndoCaptureKind::NonCoalescible,
                    cx,
                );

                let downgraded = self.document.with_structure_mutation(cx, |document, cx| {
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
                self.rebuild_image_runtimes(cx);
                self.mark_dirty(cx);
                self.finalize_pending_undo_capture(cx);
                cx.notify();
            }
            BlockAction::ToggleTaskChecked => {
                self.prepare_undo_capture(
                    crate::editor::actions::UndoCaptureKind::NonCoalescible,
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
                        crate::editor::actions::UndoCaptureKind::NonCoalescible,
                        cx,
                    );
                    self.append_table_column(&block, cx);
                    self.finalize_pending_undo_capture(cx);
                }
            }
            BlockAction::RequestAppendTableRow => {
                if block.read(cx).kind() == BlockKind::Table {
                    self.prepare_undo_capture(
                        crate::editor::actions::UndoCaptureKind::NonCoalescible,
                        cx,
                    );
                    self.append_table_row(&block, cx);
                    self.finalize_pending_undo_capture(cx);
                }
            }
            BlockAction::RequestExpandTable => {
                if block.read(cx).kind() == BlockKind::Table {
                    self.prepare_undo_capture(
                        crate::editor::actions::UndoCaptureKind::NonCoalescible,
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
                if current_visible_index == 0 {
                    return;
                }

                let target = visible_before[current_visible_index - 1].entity.clone();
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
                if current_visible_index + 1 >= visible_before.len() {
                    // A trailing multi-line block (code, math, ...) has nowhere
                    // below to move to, so give it a paragraph to land on and
                    // focus that, matching how a trailing table behaves.
                    if block.read(cx).kind().is_multiline_text_block() {
                        self.ensure_trailing_paragraph_after_structural(&block, cx);
                        let visible = self.document.flatten_visible_blocks();
                        if let Some(landing) = visible
                            .iter()
                            .position(|v| v.entity.entity_id() == block.entity_id())
                            .and_then(|index| visible.get(index + 1))
                            .map(|v| v.entity.clone())
                        {
                            self.focus_block(landing.entity_id());
                            landing.update(cx, |landing, cx| landing.move_to(0, cx));
                            cx.notify();
                        }
                    }
                    return;
                }

                let target = visible_before[current_visible_index + 1].entity.clone();
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
                if current_visible_index == 0 {
                    return;
                }

                let target = visible_before[current_visible_index - 1].entity.clone();
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
                if current_visible_index + 1 >= visible_before.len() {
                    return;
                }

                let target = visible_before[current_visible_index + 1].entity.clone();
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
                    visible_before.len() == 1 && block.read(cx).children.is_empty();
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
                    crate::editor::actions::UndoCaptureKind::NonCoalescible,
                    cx,
                );

                let visible_before_ids = visible_before
                    .iter()
                    .map(|visible| visible.entity.entity_id())
                    .collect::<Vec<_>>();
                let focus_candidate = if current_visible_index > 0 {
                    Some(visible_before_ids[current_visible_index - 1])
                } else {
                    visible_before_ids.get(current_visible_index + 1).copied()
                };

                let adopted_children = crate::editor::tree::document::Document::take_children(&block, cx);
                let removed = self.document.with_structure_mutation(cx, |document, cx| {
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
                } else if let Some(first_root) = self.document.first_root() {
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
                self.close_menu_bar(cx);
                self.clear_table_axis_preview(cx);
                self.clear_table_axis_selection(cx);
                self.focus_block(block.entity_id());
                for visible in self.document.flatten_visible_blocks() {
                    visible.entity.update(cx, |_, cx| cx.notify());
                }
                cx.notify();
            }
            BlockAction::RequestRenderedSelectAll => {}
            BlockAction::PrepareUndo { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Editor;
    use crate::model::inline::text::RichText;
    use crate::editor::actions::BlockAction;
use crate::model::block::{BlockData, BlockKind, CalloutKind};
        use crate::editor::editing::input::shortcuts::ExitCodeBlock;
    use crate::editor::editing::input::shortcuts::{Delete, DeleteBack, Newline};
    use gpui::{App, AppContext, Entity, TestAppContext};

    #[gpui::test]
    async fn request_quote_break_creates_new_root_leaf_quote_group(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| Editor::from_markdown(cx, "> first".to_string(), None));

        editor.update(cx, |editor, cx| {
            let quote = editor.document.first_root().expect("root quote").clone();
            editor.on_block_event(quote, &BlockAction::RequestQuoteBreak, cx);

            let visible = editor.document.blocks();
            assert_eq!(visible.len(), 2);
            assert_eq!(visible[0].entity.read(cx).kind(), BlockKind::Blockquote);
            assert_eq!(visible[0].entity.read(cx).display_text(), "first");
            assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::Blockquote);
            assert_eq!(visible[1].entity.read(cx).display_text(), "");
            assert_eq!(visible[1].entity.read(cx).quote_depth, 1);
            assert_eq!(editor.document.to_markdown(cx), "> first\n\n> ");
            assert_eq!(editor.focus.pending, Some(visible[1].entity.entity_id()));
        });
    }

    #[gpui::test]
    async fn typing_quote_shortcut_immediately_refreshes_rendered_quote_metadata(
        cx: &mut TestAppContext,
    ) {
        let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));

        editor.update(cx, |editor, cx| {
            let paragraph = editor
                .document
                .first_root()
                .expect("root paragraph")
                .clone();
            paragraph.update(cx, |block, cx| {
                block.prepare_undo_capture(
                    crate::editor::actions::UndoCaptureKind::CoalescibleText,
                    cx,
                );
                block.replace_text_in_visible_range(0..0, "> ", None, false, cx);
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.document.blocks();
            assert_eq!(visible.len(), 1);
            assert_eq!(visible[0].entity.read(cx).kind(), BlockKind::Blockquote);
            assert_eq!(visible[0].entity.read(cx).display_text(), "");
            assert_eq!(visible[0].entity.read(cx).quote_depth, 1);
            assert_eq!(editor.document.to_markdown(cx), "> ");
        });
    }

    #[gpui::test]
    async fn footnote_reference_jump_and_backref_follow_in_place_definition(
        cx: &mut TestAppContext,
    ) {
        let markdown = "alpha[^note]\n\n[^note]: Footnote body".to_string();
        let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

        editor.update(cx, |editor, cx| {
            let paragraph = editor
                .document
                .first_root()
                .expect("reference paragraph")
                .clone();
            let definition = editor
                .document
                .blocks()
                .iter()
                .find(|visible| visible.entity.read(cx).kind() == BlockKind::FootnoteDefinition)
                .expect("footnote definition block")
                .entity
                .clone();

            editor.on_block_event(
                paragraph.clone(),
                &BlockAction::RequestJumpToFootnoteDefinition {
                    id: "note".to_string(),
                },
                cx,
            );
            assert_eq!(editor.focus.pending, Some(definition.entity_id()));
            assert_eq!(definition.read(cx).selected_range, 0..0);

            let expected_backref_range = paragraph
                .read(cx)
                .current_range_for_footnote_occurrence(0)
                .expect("resolved footnote occurrence");
            editor.on_block_event(
                definition.clone(),
                &BlockAction::RequestJumpToFootnoteBackref {
                    id: "note".to_string(),
                },
                cx,
            );
            assert_eq!(editor.focus.pending, Some(paragraph.entity_id()));
            assert_eq!(paragraph.read(cx).selected_range, expected_backref_range);
        });
    }

    #[gpui::test]
    async fn image_block_insert_preserves_surrounding_paragraph_text(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| Editor::from_markdown(cx, "beforeafter".to_string(), None));

        editor.update(cx, |editor, cx| {
            let paragraph = editor.document.first_root().expect("paragraph").clone();
            editor.insert_image_block_after_paragraph(
                &paragraph,
                &RichText::plain("before"),
                "![image](./assets/image.png)",
                &RichText::plain("after"),
                cx,
            );

            let visible = editor.document.blocks();
            assert_eq!(visible.len(), 3);
            assert_eq!(visible[0].entity.read(cx).display_text(), "before");
            assert_eq!(
                visible[1].entity.read(cx).display_text(),
                "![image](./assets/image.png)"
            );
            assert!(visible[1].entity.read(cx).image_runtime().is_some());
            assert_eq!(visible[2].entity.read(cx).display_text(), "after");
        });
    }

    #[gpui::test]
    async fn image_paste_text_in_code_block_stays_inside_block(cx: &mut TestAppContext) {
        let editor =
            cx.new(|cx| Editor::from_markdown(cx, "```\nbeforeafter\n```".to_string(), None));

        editor.update(cx, |editor, cx| {
            let block = editor.document.first_root().expect("code block").clone();
            editor.replace_current_block_selection_with_image_text(
                &block,
                &RichText::plain("before"),
                "![image](./assets/image.png)",
                &RichText::plain("after"),
                cx,
            );

            let visible = editor.document.blocks();
            assert_eq!(visible.len(), 1);
            assert_eq!(
                visible[0].entity.read(cx).kind(),
                BlockKind::CodeBlock { language: None }
            );
            assert_eq!(
                visible[0].entity.read(cx).display_text(),
                "before![image](./assets/image.png)after"
            );
        });
    }

    #[gpui::test]
    async fn typing_callout_shortcut_materializes_body_and_focuses_it(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));

        editor.update(cx, |editor, cx| {
            let paragraph = editor
                .document
                .first_root()
                .expect("root paragraph")
                .clone();
            paragraph.update(cx, |block, cx| {
                block.prepare_undo_capture(
                    crate::editor::actions::UndoCaptureKind::CoalescibleText,
                    cx,
                );
                block.replace_text_in_visible_range(0..0, "> [!NOTE]", None, false, cx);
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.document.blocks();
            assert_eq!(visible.len(), 2);
            assert_eq!(
                visible[0].entity.read(cx).kind(),
                BlockKind::Callout(CalloutKind::Note)
            );
            assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[1].entity.read(cx).display_text(), "");
            assert_eq!(visible[1].entity.read(cx).quote_depth, 1);
            assert_eq!(editor.document.to_markdown(cx), "> [!NOTE]\n> ");
            assert_eq!(editor.focus.pending, Some(visible[1].entity.entity_id()));
        });
    }

    #[gpui::test]
    async fn typing_numbered_list_shortcut_after_separator_preserves_group_boundary(
        cx: &mut TestAppContext,
    ) {
        let editor =
            cx.new(|cx| Editor::from_markdown(cx, "1. aa\n2. bb\n3. cc".to_string(), None));

        let separator_id = editor.update(cx, |editor, cx| {
            let separator = Editor::new_block(cx, BlockData::paragraph(String::new()));
            editor.document.insert_blocks_at(
                None,
                editor.document.root_count(),
                vec![separator.clone()],
                cx,
            );
            separator.entity_id()
        });

        editor.update(cx, |editor, cx| {
            let separator = editor
                .document
                .block_entity_by_id(separator_id)
                .expect("separator paragraph");
            assert!(separator.read(cx).list_group_separator_candidate);
            separator.update(cx, |block, cx| {
                block.prepare_undo_capture(
                    crate::editor::actions::UndoCaptureKind::CoalescibleText,
                    cx,
                );
                block.replace_text_in_visible_range(0..0, "1. ", None, false, cx);
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.document.blocks();
            assert_eq!(visible.len(), 5);
            assert_eq!(visible[0].entity.read(cx).list_ordinal, Some(1));
            assert_eq!(visible[1].entity.read(cx).list_ordinal, Some(2));
            assert_eq!(visible[2].entity.read(cx).list_ordinal, Some(3));
            assert_eq!(visible[3].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[3].entity.read(cx).display_text(), "");
            assert_eq!(visible[4].entity.entity_id(), separator_id);
            assert_eq!(
                visible[4].entity.read(cx).kind(),
                BlockKind::NumberedListItem
            );
            assert_eq!(visible[4].entity.read(cx).display_text(), "");
            assert_eq!(visible[4].entity.read(cx).list_ordinal, Some(1));
            assert_eq!(
                editor.document.to_markdown(cx),
                "1. aa\n2. bb\n3. cc\n\n1. "
            );
        });
    }

    #[gpui::test]
    async fn request_indent_nests_non_empty_list_item(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| Editor::from_markdown(cx, "- a\n- b".to_string(), None));

        editor.update(cx, |editor, cx| {
            let second = editor.document.blocks()[1].entity.clone();
            editor.on_block_event(second, &BlockAction::RequestIndent, cx);

            let visible = editor.document.blocks();
            assert_eq!(visible.len(), 2);
            assert_eq!(
                visible[0].entity.read(cx).kind(),
                BlockKind::BulletListItem
            );
            assert_eq!(
                visible[1].entity.read(cx).kind(),
                BlockKind::BulletListItem
            );
            assert_eq!(visible[1].entity.read(cx).render_depth, 1);
            assert_eq!(editor.document.to_markdown(cx), "- a\n  - b");
        });
    }

    #[gpui::test]
    async fn request_outdent_lifts_list_child_paragraph_after_parent(cx: &mut TestAppContext) {
        let editor =
            cx.new(|cx| Editor::from_markdown(cx, "- item\n\n  child text".to_string(), None));

        let child_id = editor.update(cx, |editor, cx| {
            let child = editor.document.blocks()[1].entity.clone();
            editor.on_block_event(child.clone(), &BlockAction::RequestOutdent, cx);
            child.entity_id()
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.document.blocks();
            assert_eq!(visible.len(), 2);
            assert_eq!(
                visible[0].entity.read(cx).kind(),
                BlockKind::BulletListItem
            );
            assert_eq!(visible[0].entity.read(cx).display_text(), "item");
            assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[1].entity.read(cx).display_text(), "child text");
            assert_eq!(visible[1].entity.read(cx).render_depth, 0);
            assert_eq!(visible[1].entity.entity_id(), child_id);
            assert_eq!(editor.document.to_markdown(cx), "- item\n\nchild text");
        });
    }

    #[gpui::test]
    async fn empty_list_child_paragraph_backspace_outdents_to_root(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let editor = cx.new(|cx| Editor::from_markdown(cx, "- item\n\n  child".to_string(), None));

        let child_id = editor.update(cx, |editor, _cx| {
            editor.document.blocks()[1].entity.entity_id()
        });

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let child = editor.document.blocks()[1].entity.clone();
                child.update(cx, |block, block_cx| {
                    block.prepare_undo_capture(
                        crate::editor::actions::UndoCaptureKind::NonCoalescible,
                        block_cx,
                    );
                    block.replace_text_in_visible_range(
                        0..block.visible_len(),
                        "",
                        None,
                        false,
                        block_cx,
                    );
                    block.move_to(0, block_cx);
                    block.on_delete_back(&DeleteBack, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.document.blocks();
            assert_eq!(visible.len(), 2);
            assert_eq!(
                visible[0].entity.read(cx).kind(),
                BlockKind::BulletListItem
            );
            assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[1].entity.read(cx).display_text(), "");
            assert_eq!(visible[1].entity.entity_id(), child_id);
            assert_eq!(visible[1].entity.read(cx).render_depth, 0);
            assert_eq!(editor.document.to_markdown(cx), "- item\n\n");
        });
    }

    #[gpui::test]
    async fn empty_list_child_paragraph_enter_continues_same_level(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let editor = cx.new(|cx| Editor::from_markdown(cx, "- item\n\n  child".to_string(), None));

        let child_id = editor.update(cx, |editor, _cx| {
            editor.document.blocks()[1].entity.entity_id()
        });

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let child = editor.document.blocks()[1].entity.clone();
                child.update(cx, |block, block_cx| {
                    block.prepare_undo_capture(
                        crate::editor::actions::UndoCaptureKind::NonCoalescible,
                        block_cx,
                    );
                    block.replace_text_in_visible_range(
                        0..block.visible_len(),
                        "",
                        None,
                        false,
                        block_cx,
                    );
                    block.move_to(0, block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.document.blocks();
            assert_eq!(visible.len(), 3);
            assert_eq!(
                visible[0].entity.read(cx).kind(),
                BlockKind::BulletListItem
            );
            assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[1].entity.read(cx).display_text(), "");
            assert_eq!(visible[1].entity.entity_id(), child_id);
            assert_eq!(visible[1].entity.read(cx).render_depth, 1);
            assert_eq!(visible[2].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[2].entity.read(cx).display_text(), "");
            assert_eq!(visible[2].entity.read(cx).render_depth, 1);
            assert_eq!(editor.document.to_markdown(cx), "- item\n  \n  ");
        });
    }

    #[gpui::test]
    async fn enter_inside_script_paragraph_creates_new_block(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let editor = cx.new(|cx| Editor::from_markdown(cx, "H~2~O".to_string(), None));

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let block = editor.document.blocks()[0].entity.clone();
                block.update(cx, |block, block_cx| {
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.document.blocks();
            assert_eq!(visible.len(), 2);
            assert_eq!(visible[0].entity.read(cx).display_text(), "H2O");
            assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[1].entity.read(cx).display_text(), "");
            assert_eq!(editor.document.to_markdown(cx), "H~2~O\n\n");
        });
    }

    #[gpui::test]
    async fn enter_inside_inline_math_paragraph_creates_new_block(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let editor = cx.new(|cx| Editor::from_markdown(cx, "$n^2$".to_string(), None));

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let block = editor.document.blocks()[0].entity.clone();
                block.update(cx, |block, block_cx| {
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.document.blocks();
            assert_eq!(visible.len(), 2);
            assert_eq!(visible[0].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[0].entity.read(cx).display_text(), "$n^2$");
            assert!(!visible[0].entity.read(cx).uses_raw_text_editing());
            assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[1].entity.read(cx).display_text(), "");
            assert_eq!(editor.document.to_markdown(cx), "$n^2$\n\n");
        });
    }

    #[gpui::test]
    async fn trailing_fence_line_enter_closes_code_block(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let editor =
            cx.new(|cx| Editor::from_markdown(cx, "```rust\nlet x = 1;\n```".to_string(), None));

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let block = editor.document.blocks()[0].entity.clone();
                block.update(cx, |block, block_cx| {
                    // Type a closing fence on a fresh last line, then Enter.
                    let end = block.visible_len();
                    block.replace_text_in_visible_range(end..end, "\n```", None, false, block_cx);
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.document.blocks();
            assert_eq!(visible.len(), 2);
            assert_eq!(
                visible[0].entity.read(cx).kind(),
                BlockKind::CodeBlock {
                    language: Some("rust".into())
                }
            );
            assert_eq!(visible[0].entity.read(cx).display_text(), "let x = 1;");
            assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[1].entity.read(cx).display_text(), "");
            assert_eq!(
                editor.document.to_markdown(cx),
                "```rust\nlet x = 1;\n```\n\n"
            );
        });
    }

    #[gpui::test]
    async fn setext_equals_underline_enter_promotes_previous_paragraph_to_h1(
        cx: &mut TestAppContext,
    ) {
        let cx = cx.add_empty_window();
        let editor = cx.new(|cx| Editor::from_markdown(cx, "Title\n\n=====".to_string(), None));

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let underline = editor.document.blocks()[1].entity.clone();
                underline.update(cx, |block, block_cx| {
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.document.blocks();
            assert_eq!(visible.len(), 2);
            assert_eq!(
                visible[0].entity.read(cx).kind(),
                BlockKind::Heading { level: 1 }
            );
            assert_eq!(visible[0].entity.read(cx).display_text(), "Title");
            assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[1].entity.read(cx).display_text(), "");
            assert_eq!(editor.document.to_markdown(cx), "# Title\n\n");
        });

        // Reversible: undo restores the two original paragraphs.
        editor.update(cx, |editor, cx| {
            editor.undo_document(cx);
            assert_eq!(editor.document.to_markdown(cx), "Title\n\n=====");
        });
    }

    #[gpui::test]
    async fn setext_dash_underline_enter_promotes_previous_paragraph_to_h2(
        cx: &mut TestAppContext,
    ) {
        let cx = cx.add_empty_window();
        // A bare "-----" in source parses as a thematic break, so simulate the
        // user typing the underline into the paragraph below the title instead.
        let editor = cx.new(|cx| Editor::from_markdown(cx, "Title\n\nx".to_string(), None));

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let underline = editor.document.blocks()[1].entity.clone();
                underline.update(cx, |block, block_cx| {
                    let end = block.visible_len();
                    block.replace_text_in_visible_range(0..end, "-----", None, false, block_cx);
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.document.blocks();
            assert_eq!(
                visible[0].entity.read(cx).kind(),
                BlockKind::Heading { level: 2 }
            );
            assert_eq!(visible[0].entity.read(cx).display_text(), "Title");
            assert_eq!(editor.document.to_markdown(cx), "## Title\n\n");
        });
    }

    #[gpui::test]
    async fn dash_underline_without_heading_target_stays_a_separator(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let block = editor.document.blocks()[0].entity.clone();
                block.update(cx, |block, block_cx| {
                    block.replace_text_in_visible_range(0..0, "-----", None, false, block_cx);
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.document.blocks();
            assert_eq!(visible[0].entity.read(cx).kind(), BlockKind::ThematicBreak);
        });
    }

    #[gpui::test]
    async fn equals_underline_without_heading_target_stays_a_paragraph(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let block = editor.document.blocks()[0].entity.clone();
                block.update(cx, |block, block_cx| {
                    block.replace_text_in_visible_range(0..0, "=====", None, false, block_cx);
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.document.blocks();
            assert_eq!(visible[0].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[0].entity.read(cx).display_text(), "=====");
        });
    }

    #[gpui::test]
    async fn delimiter_row_enter_forms_native_table(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let editor = cx.new(|cx| {
            Editor::from_markdown(cx, "| Name | Score |\n\n| --- | --- |".to_string(), None)
        });

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let delimiter = editor.document.root_blocks()[1].clone();
                delimiter.update(cx, |block, block_cx| {
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let roots = editor.document.root_blocks();
            assert_eq!(roots.len(), 2);
            assert_eq!(roots[0].read(cx).kind(), BlockKind::Table);
            let table = roots[0].read(cx).record.table.clone().expect("table");
            assert_eq!(table.header.len(), 2);
            assert_eq!(table.header[0].serialize_markdown(), "Name");
            assert!(table.rows.is_empty());
            assert_eq!(roots[1].read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(
                editor.document.to_markdown(cx),
                "| Name | Score |\n| --- | --- |\n\n"
            );
        });

        // Reversible in one step back to the two source paragraphs.
        editor.update(cx, |editor, cx| {
            editor.undo_document(cx);
            assert_eq!(
                editor.document.to_markdown(cx),
                "| Name | Score |\n\n| --- | --- |"
            );
        });
    }

    #[gpui::test]
    async fn pipe_row_below_table_is_absorbed_as_a_row(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let editor = cx.new(|cx| {
            Editor::from_markdown(cx, "| Name | Score |\n\n| --- | --- |".to_string(), None)
        });

        // Form the table.
        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let delimiter = editor.document.root_blocks()[1].clone();
                delimiter.update(cx, |block, block_cx| {
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        // Type a body row into the paragraph below the table and press Enter.
        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let row = editor.document.root_blocks()[1].clone();
                row.update(cx, |block, block_cx| {
                    block.replace_text_in_visible_range(
                        0..0,
                        "| Alice | 10 |",
                        None,
                        false,
                        block_cx,
                    );
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let roots = editor.document.root_blocks();
            assert_eq!(roots[0].read(cx).kind(), BlockKind::Table);
            let table = roots[0].read(cx).record.table.clone().expect("table");
            assert_eq!(table.rows.len(), 1);
            assert_eq!(table.rows[0][0].serialize_markdown(), "Alice");
            assert_eq!(table.rows[0][1].serialize_markdown(), "10");
            assert_eq!(roots[1].read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(roots[1].read(cx).display_text(), "");
        });
    }

    #[gpui::test]
    async fn pipeless_delimiter_row_enter_forms_native_table(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let editor =
            cx.new(|cx| Editor::from_markdown(cx, "Name | Score\n\n---- | ----".to_string(), None));

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let delimiter = editor.document.root_blocks()[1].clone();
                delimiter.update(cx, |block, block_cx| {
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let roots = editor.document.root_blocks();
            assert_eq!(roots.len(), 2);
            assert_eq!(roots[0].read(cx).kind(), BlockKind::Table);
            let table = roots[0].read(cx).record.table.clone().expect("table");
            assert_eq!(table.header.len(), 2);
            assert_eq!(table.header[0].serialize_markdown(), "Name");
            assert_eq!(table.header[1].serialize_markdown(), "Score");
            assert!(table.rows.is_empty());
            assert_eq!(roots[1].read(cx).kind(), BlockKind::Paragraph);
        });
    }

    #[gpui::test]
    async fn pipeless_row_below_table_is_absorbed_as_a_row(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let editor =
            cx.new(|cx| Editor::from_markdown(cx, "Name | Score\n\n---- | ----".to_string(), None));

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let delimiter = editor.document.root_blocks()[1].clone();
                delimiter.update(cx, |block, block_cx| {
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        // A pipeless body row with the table's column count is absorbed.
        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let row = editor.document.root_blocks()[1].clone();
                row.update(cx, |block, block_cx| {
                    block.replace_text_in_visible_range(0..0, "Alice | 10", None, false, block_cx);
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let roots = editor.document.root_blocks();
            assert_eq!(roots[0].read(cx).kind(), BlockKind::Table);
            let table = roots[0].read(cx).record.table.clone().expect("table");
            assert_eq!(table.rows.len(), 1);
            assert_eq!(table.rows[0][0].serialize_markdown(), "Alice");
            assert_eq!(table.rows[0][1].serialize_markdown(), "10");
            assert_eq!(roots[1].read(cx).kind(), BlockKind::Paragraph);
        });
    }

    #[gpui::test]
    async fn ragged_pipeless_row_below_table_is_padded_to_width(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let editor = cx
            .new(|cx| Editor::from_markdown(cx, "A | B | C\n\n--- | --- | ---".to_string(), None));

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let delimiter = editor.document.root_blocks()[1].clone();
                delimiter.update(cx, |block, block_cx| {
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        // Two cells typed under a three-column table: absorbed as a row and
        // padded to the header width, matching how pasted ragged rows behave.
        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let row = editor.document.root_blocks()[1].clone();
                row.update(cx, |block, block_cx| {
                    block.replace_text_in_visible_range(0..0, "one | two", None, false, block_cx);
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let table = editor.document.root_blocks()[0]
                .read(cx)
                .record
                .table
                .clone()
                .expect("table");
            assert_eq!(table.rows.len(), 1);
            assert_eq!(table.rows[0].len(), 3);
            assert_eq!(table.rows[0][0].serialize_markdown(), "one");
            assert_eq!(table.rows[0][1].serialize_markdown(), "two");
            assert_eq!(table.rows[0][2].serialize_markdown(), "");
        });
    }

    #[gpui::test]
    async fn lone_pipe_row_without_table_context_stays_a_paragraph(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let block = editor.document.root_blocks()[0].clone();
                block.update(cx, |block, block_cx| {
                    block.replace_text_in_visible_range(0..0, "| a | b |", None, false, block_cx);
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let roots = editor.document.root_blocks();
            assert_eq!(roots[0].read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(roots[0].read(cx).display_text(), "| a | b |");
        });
    }

    #[gpui::test]
    async fn math_block_exit_shortcut_creates_plain_text_block(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let editor = cx.new(|cx| Editor::from_markdown(cx, "$$n^2$$".to_string(), None));

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let block = editor.document.blocks()[0].entity.clone();
                block.update(cx, |block, block_cx| {
                    block.on_exit_code_block(&ExitCodeBlock, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.document.blocks();
            assert_eq!(visible.len(), 2);
            assert_eq!(visible[0].entity.read(cx).kind(), BlockKind::MathBlock);
            assert_eq!(visible[0].entity.read(cx).display_text(), "$$n^2$$");
            assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[1].entity.read(cx).display_text(), "");
            assert_eq!(editor.document.to_markdown(cx), "$$n^2$$\n\n");
        });
    }

    #[gpui::test]
    async fn dollar_dollar_enter_creates_editable_math_block(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let block = editor.document.blocks()[0].entity.clone();
                block.update(cx, |block, block_cx| {
                    block.replace_text_in_visible_range(
                        0..block.visible_len(),
                        "$$",
                        None,
                        false,
                        block_cx,
                    );
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.document.blocks();
            assert_eq!(visible.len(), 1);
            let block = visible[0].entity.read(cx);
            assert_eq!(block.kind(), BlockKind::MathBlock);
            assert_eq!(block.display_text(), "$$\n\n$$");
            assert_eq!(block.selected_range, 3..3);
            assert!(block.uses_raw_text_editing());
            assert_eq!(editor.document.to_markdown(cx), "$$\n\n$$");
        });
    }

    #[gpui::test]
    async fn dollar_dollar_prefix_then_enter_wraps_existing_line(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let editor = cx.new(|cx| Editor::from_markdown(cx, "E = mc^2".to_string(), None));

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let block = editor.document.blocks()[0].entity.clone();
                block.update(cx, |block, block_cx| {
                    // Home, type the fence in front of the formula, then Enter.
                    block.move_to(0, block_cx);
                    block.replace_text_in_visible_range(0..0, "$$", None, false, block_cx);
                    block.move_to("$$".len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.document.blocks();
            assert_eq!(visible.len(), 1);
            let block = visible[0].entity.read(cx);
            assert_eq!(block.kind(), BlockKind::MathBlock);
            // The pre-existing text is kept as the formula body.
            assert_eq!(block.display_text(), "$$\nE = mc^2\n$$");
            assert_eq!(block.selected_range, "$$\n".len().."$$\n".len());
            assert_eq!(editor.document.to_markdown(cx), "$$\nE = mc^2\n$$");
        });
    }

    #[gpui::test]
    async fn enter_inside_math_block_keeps_local_formula_editing(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let editor = cx.new(|cx| Editor::from_markdown(cx, "$$n^2$$".to_string(), None));

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let block = editor.document.blocks()[0].entity.clone();
                block.update(cx, |block, block_cx| {
                    block.move_to(3, block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.document.blocks();
            assert_eq!(visible.len(), 1);
            assert_eq!(visible[0].entity.read(cx).kind(), BlockKind::MathBlock);
            assert_eq!(visible[0].entity.read(cx).display_text(), "$$n\n^2$$");
            assert_eq!(editor.document.to_markdown(cx), "$$n\n^2$$");
        });
    }

    #[gpui::test]
    async fn auto_created_math_block_exit_shortcut_creates_plain_text_block(
        cx: &mut TestAppContext,
    ) {
        let cx = cx.add_empty_window();
        let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let block = editor.document.blocks()[0].entity.clone();
                block.update(cx, |block, block_cx| {
                    block.replace_text_in_visible_range(
                        0..block.visible_len(),
                        "$$",
                        None,
                        false,
                        block_cx,
                    );
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                    block.on_exit_code_block(&ExitCodeBlock, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.document.blocks();
            assert_eq!(visible.len(), 2);
            assert_eq!(visible[0].entity.read(cx).kind(), BlockKind::MathBlock);
            assert_eq!(visible[0].entity.read(cx).display_text(), "$$\n\n$$");
            assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[1].entity.read(cx).display_text(), "");
            assert_eq!(editor.document.to_markdown(cx), "$$\n\n$$\n\n");
        });
    }

    #[gpui::test]
    async fn raw_like_block_exit_shortcut_creates_plain_text_block(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let cases = [
            (
                BlockData::new(
                    BlockKind::HtmlBlock,
                    RichText::plain("<div>\ncontent\n</div>".to_string()),
                ),
                BlockKind::HtmlBlock,
                "<div>\ncontent\n</div>",
            ),
            (
                BlockData::new(
                    BlockKind::MermaidBlock,
                    RichText::plain("```mermaid\nflowchart LR\nA-->B\n```".to_string()),
                ),
                BlockKind::MermaidBlock,
                "```mermaid\nflowchart LR\nA-->B\n```",
            ),
            (
                BlockData::new(
                    BlockKind::RawMarkdown,
                    RichText::plain("::: custom\ncontent\n:::".to_string()),
                ),
                BlockKind::RawMarkdown,
                "::: custom\ncontent\n:::",
            ),
            (
                BlockData::new(
                    BlockKind::HtmlComment,
                    RichText::plain("<!--\ncomment\n-->".to_string()),
                ),
                BlockKind::HtmlComment,
                "<!--\ncomment\n-->",
            ),
        ];

        for (record, kind, text) in cases {
            let editor = cx.new(|cx| {
                let mut editor = Editor::from_markdown(cx, String::new(), None);
                let block = Editor::new_block(cx, record.clone());
                editor.document.replace_blocks(vec![block], cx);
                editor
            });

            cx.update(|window, cx| {
                editor.update(cx, |editor, cx| {
                    let block = editor.document.blocks()[0].entity.clone();
                    block.update(cx, |block, block_cx| {
                        block.on_exit_code_block(&ExitCodeBlock, window, block_cx);
                    });
                });
            });

            editor.update(cx, |editor, cx| {
                let visible = editor.document.blocks();
                assert_eq!(visible.len(), 2);
                assert_eq!(visible[0].entity.read(cx).kind(), kind);
                assert_eq!(visible[0].entity.read(cx).display_text(), text);
                assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::Paragraph);
                assert_eq!(visible[1].entity.read(cx).display_text(), "");
            });
        }
    }

    #[gpui::test]
    async fn table_cell_enter_still_moves_to_next_row(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let markdown = ["| A | B |", "| --- | --- |", "| 1 | 2 |", "| 3 | 4 |"].join("\n");
        let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

        let mut next_cell_id = None;
        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let table = editor.document.first_root().expect("table root").clone();
                let (cell, expected_next_cell_id) = {
                    let table = table.read(cx);
                    let runtime = table.table_runtime.as_ref().expect("table runtime");
                    (runtime.rows[0][0].clone(), runtime.rows[1][0].entity_id())
                };
                next_cell_id = Some(expected_next_cell_id);
                cell.update(cx, |block, block_cx| {
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, _cx| {
            assert_eq!(editor.document.blocks().len(), 1);
            assert_eq!(editor.focus.pending, next_cell_id);
        });
    }

    #[gpui::test]
    async fn table_cell_exit_shortcut_inserts_sibling_after_table(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let markdown = ["> [!NOTE]", "> | A | B |", "> | --- | --- |", "> | 1 | 2 |"].join("\n");
        let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let callout = editor.document.first_root().expect("callout root").clone();
                let table = callout
                    .read(cx)
                    .children
                    .iter()
                    .find(|child| child.read(cx).kind() == BlockKind::Table)
                    .expect("nested table")
                    .clone();
                let cell = table
                    .read(cx)
                    .table_runtime
                    .as_ref()
                    .expect("table runtime")
                    .rows[0][0]
                    .clone();
                cell.update(cx, |block, block_cx| {
                    block.on_exit_code_block(&ExitCodeBlock, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let callout = editor.document.first_root().expect("callout root").clone();
            let children = callout.read(cx).children.clone();
            assert_eq!(children.len(), 2);
            assert_eq!(children[0].read(cx).kind(), BlockKind::Table);
            assert_eq!(children[1].read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(children[1].read(cx).display_text(), "");
            assert_eq!(editor.focus.pending, Some(children[1].entity_id()));
        });
    }
}
