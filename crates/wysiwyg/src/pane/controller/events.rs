//! WysiwygDocumentController — events handlers.

use gpui::{Context, Entity, EntityId, Pixels, Point, SharedString, Window, px};

use crate::model::block::{Block, CollapsedCaretAffinity};
use crate::model::protocol::BlockEvent;
use crate::pane::state::TableAxisSelection;
use markdown_parser::block::table::{TableAxis, TableAxisMarker, TableCellPosition};
use markdown_parser::inline::text::BlockText;
use markdown_parser::parse::{BlockData, BlockKind};

use super::WysiwygDocumentController;
use super::{ContextSubmenu, FootnoteTooltipState, WysiwygContextMenuState};
impl WysiwygDocumentController {
    pub fn on_block_event(
        &mut self,
        block: Entity<Block>,
        event: &BlockEvent,
        cx: &mut Context<Self>,
    ) {
        if let Some(binding) = self.tables.cells.get(&block.entity_id()).cloned() {
            match event {
                BlockEvent::Changed => {
                    crate::table::grid::sync_table_data_from_grid(&binding.table_block, cx);
                    self.pending_edit = true;
                    self.commit_document_edit(false, cx);
                    cx.notify();
                    return;
                }
                BlockEvent::RequestTableCellMoveHorizontal { delta } => {
                    let current = binding.position;
                    let table_block = binding.table_block.clone();
                    if let Some(grid) = table_block.read(cx).table_grid.clone() {
                        let col_count = grid.header.len();
                        let total_rows = grid.rows.len() + 1;
                        if col_count > 0 {
                            let (next_row, next_col) = if *delta > 0 {
                                if current.column + 1 < col_count {
                                    (current.row, current.column + 1)
                                } else if current.row + 1 < total_rows {
                                    (current.row + 1, 0)
                                } else {
                                    table_block.update(cx, |b, _cx| {
                                        if let Some(table) = b.data.table.as_mut() {
                                            crate::table::rows::append_table_row(table);
                                        }
                                    });
                                    self.rebuild_table_grids(cx);
                                    self.pending_edit = true;
                                    self.commit_document_edit(false, cx);
                                    (current.row + 1, 0)
                                }
                            } else if current.column > 0 {
                                (current.row, current.column - 1)
                            } else if current.row > 0 {
                                (current.row - 1, col_count.saturating_sub(1))
                            } else {
                                (0, 0)
                            };
                            let next_pos = TableCellPosition {
                                row: next_row,
                                column: next_col,
                            };
                            if let Some(cell) = table_block
                                .read(cx)
                                .table_grid
                                .as_ref()
                                .and_then(|g| g.cell(next_pos))
                            {
                                self.active_entity = Some(cell.clone());
                                cell.update(cx, |c, cx| {
                                    c.start_cursor_blink(cx);
                                    cx.notify();
                                });
                                cx.notify();
                            }
                        }
                    }
                    return;
                }
                BlockEvent::RequestTableCellMoveVertical { delta } => {
                    let current = binding.position;
                    let table_block = binding.table_block.clone();
                    if let Some(grid) = table_block.read(cx).table_grid.clone() {
                        let total_rows = grid.rows.len() + 1;
                        let next_row = if *delta > 0 {
                            (current.row + 1).min(total_rows.saturating_sub(1))
                        } else {
                            current.row.saturating_sub(1)
                        };
                        let next_pos = TableCellPosition {
                            row: next_row,
                            column: current.column,
                        };
                        if let Some(cell) = grid.cell(next_pos) {
                            self.active_entity = Some(cell.clone());
                            cell.update(cx, |c, cx| {
                                c.start_cursor_blink(cx);
                                cx.notify();
                            });
                            cx.notify();
                        }
                    }
                    return;
                }
                _ => {}
            }
        }

        match event {
            BlockEvent::Changed => {
                self.pending_edit = true;
                self.commit_typing_edit(cx);
                cx.notify();
            }
            BlockEvent::RequestFocus => {
                self.active_entity = Some(block.clone());
                self.footnote_tooltip = None;
                self.clear_all_table_axis_selections(cx);
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
            BlockEvent::RequestAppendTableColumn => {
                let target = block.clone();
                let mut modified = false;
                target.update(cx, |b, _cx| {
                    if let Some(table) = b.data.table.as_mut() {
                        crate::table::columns::append_table_column(table);
                        modified = true;
                    }
                });
                if modified {
                    self.rebuild_table_grids(cx);
                    self.pending_edit = true;
                    self.commit_document_edit(false, cx);
                    cx.notify();
                }
            }
            BlockEvent::RequestAppendTableRow => {
                let target = block.clone();
                let mut modified = false;
                target.update(cx, |b, _cx| {
                    if let Some(table) = b.data.table.as_mut() {
                        crate::table::rows::append_table_row(table);
                        modified = true;
                    }
                });
                if modified {
                    self.rebuild_table_grids(cx);
                    self.pending_edit = true;
                    self.commit_document_edit(false, cx);
                    cx.notify();
                }
            }
            BlockEvent::RequestTableAxisPreview {
                kind,
                index,
                hovered,
            } => {
                let target_entity = block.entity_id();
                if let Some(doc) = &self.document {
                    for entry in doc.blocks() {
                        if entry.entity.entity_id() != target_entity {
                            entry.entity.update(cx, |blk, cx| {
                                if blk.table_axis_preview.is_some() {
                                    blk.table_axis_preview = None;
                                    cx.notify();
                                }
                            });
                        }
                    }
                }
                block.update(cx, |b, cx| {
                    b.table_axis_preview = if *hovered {
                        Some(TableAxisMarker {
                            kind: *kind,
                            index: *index,
                        })
                    } else {
                        None
                    };
                    cx.notify();
                });
                cx.notify();
            }
            BlockEvent::RequestSelectTableAxis { kind, index } => {
                let target_entity = block.entity_id();
                if let Some(doc) = &self.document {
                    for entry in doc.blocks() {
                        if entry.entity.entity_id() != target_entity {
                            entry.entity.update(cx, |blk, cx| {
                                if blk.table_axis_selection.is_some() {
                                    blk.table_axis_selection = None;
                                    cx.notify();
                                }
                            });
                        }
                    }
                }
                block.update(cx, |b, cx| {
                    b.table_axis_selection = Some(TableAxisMarker {
                        kind: *kind,
                        index: *index,
                    });
                    cx.notify();
                });
                cx.notify();
            }
            BlockEvent::RequestReorderTableAxis { kind, from, to } => {
                crate::table::axis::reorder_table_axis(&block, *kind, *from, *to, cx);
                self.rebuild_table_grids(cx);
                self.pending_edit = true;
                self.commit_document_edit(false, cx);
                cx.notify();
            }
            BlockEvent::RequestInsertTableAxisAt { kind, index } => {
                block.update(cx, |b, _cx| {
                    if let Some(table) = b.data.table.as_mut() {
                        match kind {
                            TableAxis::Column => {
                                crate::table::columns::insert_table_column_at(table, *index);
                            }
                            TableAxis::Row => {
                                crate::table::rows::insert_table_row_at(table, *index);
                            }
                        }
                    }
                });
                self.rebuild_table_grids(cx);
                self.pending_edit = true;
                self.commit_document_edit(false, cx);
                cx.notify();
            }
            BlockEvent::RequestFootnoteTooltip {
                id,
                content,
                position,
                show,
            } => {
                if *show {
                    let resolved_content = content.clone().or_else(|| {
                        let binding = self.references.footnotes.binding(id)?;
                        let doc = self.document.as_ref()?;
                        let def_entry = doc
                            .blocks()
                            .iter()
                            .find(|e| e.entity.read(cx).data.id == binding.definition_block_id)?;
                        let plain = def_entry.entity.read(cx).data.text.plain_text();
                        let text =
                            markdown_parser::block::footnote::split_footnote_definition_text(
                                &plain,
                            )
                            .1;
                        Some(SharedString::from(text.trim().to_string()))
                    });
                    if let Some(content) = resolved_content {
                        self.footnote_tooltip = Some(FootnoteTooltipState {
                            id: id.clone(),
                            content,
                            position: *position,
                        });
                        cx.notify();
                    }
                } else if self.footnote_tooltip.as_ref().map(|t| &t.id) == Some(id) {
                    self.footnote_tooltip = None;
                    cx.notify();
                }
            }
            BlockEvent::RequestJumpToFootnoteDefinition { id } => {
                if let Some(binding) = self.references.footnotes.binding(id)
                    && let Some(doc) = &self.document
                    && let Some(entry) = doc
                        .blocks()
                        .iter()
                        .find(|e| e.entity.read(cx).data.id == binding.definition_block_id)
                {
                    let target = entry.entity.clone();
                    self.active_entity = Some(target.clone());
                    target.update(cx, |b, cx| {
                        b.selected_range = 0..0;
                        b.selection_reversed = false;
                        b.marked_range = None;
                        b.start_cursor_blink(cx);
                        cx.notify();
                    });
                    cx.notify();
                }
            }
            BlockEvent::RequestJumpToFootnoteBackref { id } => {
                if let Some(binding) = self.references.footnotes.binding(id)
                    && let Some(first_ref) = binding.first_reference.as_ref()
                    && let Some(doc) = &self.document
                    && let Some(entry) = doc
                        .blocks()
                        .iter()
                        .find(|e| e.entity.read(cx).data.id == first_ref.block_id)
                {
                    let target = entry.entity.clone();
                    self.active_entity = Some(target.clone());
                    target.update(cx, |b, cx| {
                        b.sync_inline_projection_for_focus(true);
                        if let Some(range) =
                            b.display_range_for_footnote_occurrence(first_ref.occurrence_index)
                        {
                            b.selected_range = range;
                            b.selection_reversed = false;
                            b.marked_range = None;
                        }
                        b.start_cursor_blink(cx);
                        cx.notify();
                    });
                    cx.notify();
                }
            }
            BlockEvent::RequestOpenTableAxisMenu {
                kind,
                index,
                position,
            } => {
                let table_block_id = block.entity_id();
                self.context_menu = Some(WysiwygContextMenuState::TableAxis {
                    position: *position,
                    selection: TableAxisSelection {
                        table_block_id,
                        kind: *kind,
                        index: *index,
                    },
                });
                cx.notify();
            }
            _ => {}
        }
    }

    pub fn open_context_menu(
        &mut self,
        entity_id: EntityId,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if let Some(doc) = &self.document
            && let Some(target) = doc.block_entity_by_id(entity_id)
        {
            self.active_entity = Some(target);
        }
        self.context_menu = Some(WysiwygContextMenuState::Edit {
            position,
            target_entity_id: Some(entity_id),
            active_submenu: None,
        });
        cx.notify();
    }

    pub fn set_context_menu_submenu(
        &mut self,
        submenu: Option<ContextSubmenu>,
        cx: &mut Context<Self>,
    ) {
        if let Some(WysiwygContextMenuState::Edit { active_submenu, .. }) = &mut self.context_menu {
            if *active_submenu != submenu {
                *active_submenu = submenu;
                cx.notify();
            }
        }
    }

    pub fn close_context_menu(&mut self, cx: &mut Context<Self>) {
        self.context_menu = None;
        cx.notify();
    }

    pub fn paste_plain_into_active(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(active) = &self.active_entity else {
            return;
        };
        if let Some(item) = cx.read_from_clipboard() {
            if let Some(text) = item.text() {
                active.update(cx, |b, cx| {
                    let range = b.selected_range.clone();
                    b.apply_source_space_text_edit(range, &text, None, false, cx);
                });
                self.pending_edit = true;
                self.commit_document_edit(false, cx);
                cx.notify();
            }
        }
    }
}
