//! Navigation: page keys, viewport scrolling, and table-cell focus
//! movement. Mouse/scrollbar interactions live in `mouse`; menu input
//! lives in `crate::editor::panes::document_pane::menu`.

use gpui::*;

use super::actions::{JumpToBottom, JumpToTop, PageDown, PageUp};
use crate::editor::document::protocol::BlockEvent;
use crate::editor::engine::controller::*;
use crate::editor::document::block::CollapsedCaretAffinity;
use splitype_model::block::table::TableCellPosition;
use splitype_model::parse::BlockKind;

impl Editor {
    pub(crate) fn on_page_up(&mut self, _: &PageUp, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_active_tab() {
            return;
        }
        let page = self.active_pane_scroll().handle.bounds().size.height;
        self.scroll_viewport_by(self.active_pane_id(), page, cx);
    }

    pub(crate) fn on_page_down(
        &mut self,
        _: &PageDown,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_active_tab() {
            return;
        }
        let page = self.active_pane_scroll().handle.bounds().size.height;
        self.scroll_viewport_by(self.active_pane_id(), -page, cx);
    }

    pub(crate) fn on_jump_to_top(
        &mut self,
        _: &JumpToTop,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_active_tab() {
            return;
        }
        self.set_vertical_scroll_offset(self.active_pane_id(), px(0.0), cx);
    }

    pub(crate) fn on_jump_to_bottom(
        &mut self,
        _: &JumpToBottom,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_active_tab() {
            return;
        }
        let max_offset_y = self
            .active_pane_scroll()
            .handle
            .max_offset()
            .y
            .max(px(0.0));
        self.set_vertical_scroll_offset(self.active_pane_id(), -max_offset_y, cx);
    }

    /// Scrolls the viewport vertically by `delta`. A positive `delta` moves
    /// toward the start of the document; a negative one moves toward the end.
    /// One page is the current viewport height, so the step tracks window size.
    pub(crate) fn scroll_viewport_by(
        &mut self,
        pane_id: PaneId,
        delta: Pixels,
        cx: &mut Context<Self>,
    ) {
        let target = self
            .pane_state_ref(pane_id)
            .map(|state| state.scroll.handle.offset().y + delta)
            .unwrap_or_default();
        self.set_vertical_scroll_offset(pane_id, target, cx);
    }

    /// Applies an absolute vertical scroll offset, clamped to the scrollable
    /// range. Offsets run from 0 at the top to `-max_offset` at the bottom.
    pub(crate) fn set_vertical_scroll_offset(
        &mut self,
        pane_id: PaneId,
        target_y: Pixels,
        cx: &mut Context<Self>,
    ) {
        let max_offset_y = self
            .pane_state_ref(pane_id)
            .map(|state| state.scroll.handle.max_offset().y.max(px(0.0)))
            .unwrap_or_default();
        let mut offset = self
            .pane_state_ref(pane_id)
            .map(|state| state.scroll.handle.offset())
            .unwrap_or_default();
        offset.y = target_y.min(px(0.0)).max(-max_offset_y);
        {
            let state = self.pane_state(pane_id);
            state.scroll.handle.set_offset(offset);
            state.scroll.pending_autoscroll = None;
        }
        self.bump_scrollbar_visibility(pane_id, cx);
        cx.notify();
    }

    pub(crate) fn focus_table_cell_position(
        &mut self,
        table_block: &Entity<crate::editor::document::block::Block>,
        position: TableCellPosition,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(cell) = table_block
            .read(cx)
            .table_grid
            .as_ref()
            .and_then(|grid| grid.cell(position))
        else {
            return false;
        };
        self.focus_block(cell.entity_id());
        cx.notify();
        true
    }

    /// Focus a cell when keyboard navigation enters a table from an adjacent
    /// block. Entering from above lands on the first header cell; entering from
    /// below lands on the first cell of the last body row, falling back to the
    /// header when the table has no body rows.
    pub(crate) fn focus_table_entry_cell(
        &mut self,
        table_block: &Entity<crate::editor::document::block::Block>,
        from_top: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(grid) = table_block.read(cx).table_grid.clone() else {
            return false;
        };
        let cell = if from_top {
            grid.header.first().cloned()
        } else {
            grid.rows
                .last()
                .and_then(|row| row.first())
                .cloned()
                .or_else(|| grid.header.first().cloned())
        };
        let Some(cell) = cell else {
            return false;
        };
        self.focus_block(cell.entity_id());
        cx.notify();
        true
    }

    /// Move focus from a table edge to the block immediately above (delta < 0)
    /// or below (delta > 0) it, mirroring how plain blocks transfer focus when
    /// the caret leaves their first or last line. When the neighbor is itself a
    /// table, drop into one of its cells so the caret stays editable instead of
    /// landing on the table container. `to_block_start` lands the caret at the
    /// neighbor's start (Block Up/Down semantics) rather than the nearest edge
    /// (Move Up/Down semantics).
    pub(crate) fn focus_block_adjacent_to_table(
        &mut self,
        table_block: &Entity<crate::editor::document::block::Block>,
        delta: i32,
        to_block_start: bool,
        cx: &mut Context<Self>,
    ) {
        let entry = self.doc().cloned_entries();
        let Some(index) = entry
            .iter()
            .position(|entry| entry.entity.entity_id() == table_block.entity_id())
        else {
            return;
        };
        let target_index = if delta < 0 {
            index.checked_sub(1)
        } else {
            Some(index + 1)
        };
        let Some(target) = target_index
            .and_then(|target_index| entry.get(target_index))
            .map(|entry| entry.entity.clone())
        else {
            return;
        };
        if target.read(cx).kind() == BlockKind::Table
            && self.focus_table_entry_cell(&target, delta > 0, cx)
        {
            return;
        }
        self.focus_block(target.entity_id());
        if to_block_start {
            target.update(cx, |target, cx| target.move_to(0, cx));
        } else {
            let prefer_last_line = delta < 0;
            let offset = target
                .read(cx)
                .entry_offset_for_vertical_focus(prefer_last_line, None);
            target.update(cx, move |target, cx| {
                target.move_to_with_preferred_x(offset, None, cx);
            });
        }
        cx.notify();
    }

    pub(crate) fn focus_table_cell_horizontal_neighbor(
        &mut self,
        table_block: &Entity<crate::editor::document::block::Block>,
        position: TableCellPosition,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let Some(grid) = table_block.read(cx).table_grid.clone() else {
            return;
        };
        let columns = grid.header.len();
        let total_rows = 1 + grid.rows.len();
        if columns == 0 || total_rows == 0 {
            return;
        }

        let linear = position.row * columns + position.column;
        let next = if delta < 0 {
            linear.checked_sub(delta.unsigned_abs() as usize)
        } else {
            linear.checked_add(delta as usize)
        };
        let Some(next) = next else {
            if delta < 0 {
                self.focus_block_adjacent_to_table(table_block, -1, false, cx);
            }
            return;
        };
        if next >= total_rows * columns {
            if delta > 0 {
                self.append_table_row(table_block, cx);
                let _ = self.focus_table_cell_position(
                    table_block,
                    TableCellPosition {
                        row: total_rows,
                        column: 0,
                    },
                    cx,
                );
            }
            return;
        }

        let next_position = TableCellPosition {
            row: next / columns,
            column: next % columns,
        };
        let _ = self.focus_table_cell_position(table_block, next_position, cx);
    }

    pub(crate) fn focus_table_cell_vertical_neighbor(
        &mut self,
        table_block: &Entity<crate::editor::document::block::Block>,
        position: TableCellPosition,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let Some(grid) = table_block.read(cx).table_grid.clone() else {
            return;
        };
        let max_row = grid.rows.len();
        let next_row = if delta < 0 {
            position.row.checked_sub(delta.unsigned_abs() as usize)
        } else {
            position.row.checked_add(delta as usize)
        };
        // Moving past the first/last row leaves the table for the adjacent
        // block rather than stopping at the edge.
        let Some(next_row) = next_row.filter(|row| *row <= max_row) else {
            self.focus_block_adjacent_to_table(table_block, delta, false, cx);
            return;
        };

        let next_position = TableCellPosition {
            row: next_row,
            column: position.column.min(grid.header.len().saturating_sub(1)),
        };
        let _ = self.focus_table_cell_position(table_block, next_position, cx);
    }

    pub(crate) fn on_table_cell_event(
        &mut self,
        binding: crate::editor::engine::controller::TableCellBinding,
        event: &BlockEvent,
        cx: &mut Context<Self>,
    ) {
        if event.clears_cross_block_selection() {
            let state = self.active_pane_state();
            if let Some(selection) = state.selection_mut() {
                selection.select_all_cycle = None;
            }
            self.clear_cross_block_selection(cx);
        }

        match event {
            BlockEvent::Changed => {
                self.sync_table_data_from_grid(&binding.table_block, cx);
                self.sync_references_after_block_change(&binding.cell, cx);
                self.mark_dirty(cx);
                self.request_autoscroll_active_pane(
                    crate::editor::engine::controller::AutoscrollStrategy::Fit { margin: px(20.0) },
                    cx,
                );
                self.finalize_pending_undo_capture(cx);
            }
            BlockEvent::RequestOpenLink {
                prompt_target,
                open_target,
            } => {
                self.request_open_link_prompt(prompt_target.clone(), open_target.clone(), cx);
            }
            BlockEvent::RequestJumpToFootnoteDefinition { id, .. } => {
                let _ = self.jump_to_footnote_definition(id, cx);
            }
            BlockEvent::RequestJumpToFootnoteBackref { id } => {
                let _ = self.jump_to_footnote_backref(id, cx);
            }
            BlockEvent::RequestTableCellMoveHorizontal { delta } => {
                self.focus_table_cell_horizontal_neighbor(
                    &binding.table_block,
                    binding.position,
                    *delta,
                    cx,
                );
            }
            BlockEvent::RequestTableCellMoveVertical { delta } => {
                self.focus_table_cell_vertical_neighbor(
                    &binding.table_block,
                    binding.position,
                    *delta,
                    cx,
                );
            }
            BlockEvent::RequestNewline { .. } => {
                let Some(location) = self
                    .doc()
                    .find_block_location(binding.table_block.entity_id())
                else {
                    return;
                };
                self.clear_table_axis_preview(cx);
                self.clear_table_axis_selection(cx);
                self.sync_table_data_from_grid(&binding.table_block, cx);
                self.prepare_undo_capture(
                    crate::editor::document::protocol::UndoCaptureKind::NonCoalescible,
                    cx,
                );
                let new_block = Self::new_block(cx, BlockData::paragraph(String::new()));
                self.doc_mut().insert_blocks_at(
                    location.parent,
                    location.index + 1,
                    vec![new_block.clone()],
                    cx,
                );
                self.rebuild_reference_registries(cx);
                self.focus_block(new_block.entity_id());
                self.mark_dirty(cx);
                self.request_autoscroll_active_pane(
                    crate::editor::engine::controller::AutoscrollStrategy::Fit { margin: px(20.0) },
                    cx,
                );
                self.finalize_pending_undo_capture(cx);
                cx.notify();
            }
            BlockEvent::RequestFocus => {
                self.clear_table_axis_preview(cx);
                self.clear_table_axis_selection(cx);
                self.focus_block(binding.cell.entity_id());
                cx.notify();
            }
            BlockEvent::RequestFocusPrevious { .. } => {
                self.focus_table_cell_vertical_neighbor(
                    &binding.table_block,
                    binding.position,
                    -1,
                    cx,
                );
            }
            BlockEvent::RequestFocusNext { .. } => {
                self.focus_table_cell_vertical_neighbor(
                    &binding.table_block,
                    binding.position,
                    1,
                    cx,
                );
            }
            // Block Up/Down treat the table as a single block: leave it
            // entirely for the block above/below rather than stepping by cell.
            BlockEvent::RequestBlockUp => {
                self.focus_block_adjacent_to_table(&binding.table_block, -1, true, cx);
            }
            BlockEvent::RequestBlockDown => {
                self.focus_block_adjacent_to_table(&binding.table_block, 1, true, cx);
            }
            _ => {}
        }
    }

    pub(crate) fn nearest_quote_ancestor(
        &self,
        entity_id: EntityId,
        cx: &App,
    ) -> Option<Entity<crate::editor::document::block::Block>> {
        let mut current = self.focusable_entity_by_id(entity_id)?;
        loop {
            if current.read(cx).kind().is_quote_container() {
                return Some(current);
            }
            let location = self.doc().find_block_location(current.entity_id())?;
            current = location.parent?;
        }
    }

    pub(crate) fn topmost_quote_ancestor(
        &self,
        entity_id: EntityId,
        cx: &App,
    ) -> Option<Entity<crate::editor::document::block::Block>> {
        let mut current = self.nearest_quote_ancestor(entity_id, cx)?;
        loop {
            let Some(location) = self.doc().find_block_location(current.entity_id()) else {
                break;
            };
            let Some(parent) = location.parent.clone() else {
                break;
            };
            if !parent.read(cx).kind().is_quote_container() {
                break;
            }
            current = parent;
        }
        Some(current)
    }

    pub(crate) fn quote_break_insertion_target(
        &self,
        entity_id: EntityId,
        cx: &App,
    ) -> Option<(Option<Entity<crate::editor::document::block::Block>>, usize)> {
        let quote_block = self.nearest_quote_ancestor(entity_id, cx)?;
        let location = self.doc().find_block_location(quote_block.entity_id())?;
        Some((location.parent.clone(), location.index + 1))
    }

    pub(crate) fn callout_break_insertion_target(
        &self,
        entity_id: EntityId,
        cx: &App,
    ) -> Option<(Option<Entity<crate::editor::document::block::Block>>, usize)> {
        let callout_root = self.topmost_quote_ancestor(entity_id, cx)?;
        let location = self.doc().find_block_location(callout_root.entity_id())?;
        Some((location.parent.clone(), location.index + 1))
    }

    pub(crate) fn ensure_callout_body_entry(
        &mut self,
        callout: &Entity<crate::editor::document::block::Block>,
        cx: &mut Context<Self>,
    ) -> Option<Entity<crate::editor::document::block::Block>> {
        if !matches!(callout.read(cx).kind(), BlockKind::Callout(_)) {
            return None;
        }

        if let Some(first_child) = callout.read(cx).children.first().cloned() {
            return Some(first_child);
        }

        let body = Self::new_block(cx, BlockData::paragraph(String::new()));
        self.doc_mut()
            .insert_blocks_at(Some(callout.clone()), 0, vec![body.clone()], cx);
        Some(body)
    }

    pub(crate) fn materialize_empty_callout_shortcut(
        &mut self,
        block: &Entity<crate::editor::document::block::Block>,
        cx: &mut Context<Self>,
    ) -> Option<EntityId> {
        if !self.is_wysiwyg() {
            return None;
        }

        let (kind, text_markdown, has_children) = block.read_with(cx, |block, _cx| {
            (
                block.kind(),
                block.data.text.serialize_markdown(),
                !block.children.is_empty(),
            )
        });
        if kind != BlockKind::Blockquote || has_children {
            return None;
        }

        let Some((variant, text)) =
            splitype_model::block::CalloutKind::parse_header_line(&text_markdown)
        else {
            return None;
        };

        block.update(cx, |block, cx| {
            block.data.kind = BlockKind::Callout(variant);
            block.data.set_text(BlockText::from_markdown(&text));
            block.sync_edit_mode_from_kind();
            block.sync_render_cache();
            block.cursor_blink_epoch = Instant::now();
            cx.notify();
        });
        let body = self.ensure_callout_body_entry(block, cx)?;
        Some(body.entity_id())
    }

    pub(crate) fn downgrade_empty_callout_body_to_quote(
        &mut self,
        block: &Entity<crate::editor::document::block::Block>,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(location) = self.doc().find_block_location(block.entity_id()) else {
            return false;
        };
        let Some(parent) = location.parent.clone() else {
            return false;
        };

        let (header_markdown, only_child, block_is_empty_leaf) = {
            let parent_ref = parent.read(cx);
            let Some(variant) = parent_ref.kind().callout_kind() else {
                return false;
            };
            let block_ref = block.read(cx);
            (
                variant.header_markdown(&parent_ref.data.text.serialize_markdown()),
                parent_ref.children.len() == 1,
                block_ref.kind() == BlockKind::Paragraph
                    && block_ref.display_text().is_empty()
                    && block_ref.children.is_empty(),
            )
        };
        if !only_child || !block_is_empty_leaf {
            return false;
        }

        self.prepare_undo_capture(
            crate::editor::document::protocol::UndoCaptureKind::NonCoalescible,
            cx,
        );
        self.doc_mut().with_structure_mutation(cx, |document, cx| {
            let _ = document.remove_block_unindexed(block.entity_id(), cx);
            parent.update(cx, |parent, cx| {
                parent.data.kind = BlockKind::Blockquote;
                parent
                    .data
                    .set_text(BlockText::from_markdown(&header_markdown));
                parent.sync_edit_mode_from_kind();
                parent.sync_render_cache();
                parent.assign_collapsed_selection_offset(0, CollapsedCaretAffinity::Default, None);
                parent.marked_range = None;
                parent.cursor_blink_epoch = Instant::now();
                cx.notify();
            });
        });
        self.focus_block(parent.entity_id());
        self.rebuild_reference_registries(cx);
        self.mark_dirty(cx);
        self.finalize_pending_undo_capture(cx);
        cx.notify();
        true
    }
}
