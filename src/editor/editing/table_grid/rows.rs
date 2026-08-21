//! Table row operations: insert, append, duplicate, move, delete, expand.

use gpui::*;

use crate::editor::block_protocol::UndoCaptureKind;
use crate::editor::controller::{Editor, TableAxisSelection};
use crate::editor::tree::block::Block;
use crate::model::block::table::{TableAxis, TableCellPosition};
use crate::model::parse::BlockData;

impl Editor {
    pub(crate) fn append_table_row(&mut self, table_block: &Entity<Block>, cx: &mut Context<Self>) {
        self.sync_table_data_from_grid(table_block, cx);

        let Some(mut table) = table_block.read(cx).data.table.clone() else {
            return;
        };
        let started_local_capture = if self.tab().undo.pending_capture.is_none() {
            self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
            true
        } else {
            false
        };
        table.append_row();

        table_block.update(cx, move |block, _cx| {
            block.data.table = Some(table.clone());
        });
        self.rebuild_table_grids(cx);
        if let Some(cell) = table_block
            .read(cx)
            .table_grid
            .as_ref()
            .and_then(|grid| grid.rows.last())
            .and_then(|row| row.first())
        {
            self.focus_block(cell.entity_id());
        }
        self.mark_dirty(cx);
        self.request_active_block_scroll_into_view(self.active_pane_id(), cx);
        if started_local_capture {
            self.finalize_pending_undo_capture(cx);
        }
        cx.notify();
    }

    pub(crate) fn move_table_row(
        &mut self,
        table_block: &Entity<Block>,
        visual_row: usize,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        self.sync_table_data_from_grid(table_block, cx);
        let Some(mut table) = table_block.read(cx).data.table.clone() else {
            return;
        };
        let total_rows = table.rows.len() + 1;
        let next_row = if delta < 0 {
            visual_row.checked_sub(delta.unsigned_abs() as usize)
        } else {
            visual_row.checked_add(delta as usize)
        };
        let Some(next_row) = next_row else {
            return;
        };
        // Visual rows are the header (0) plus every body row, so the last valid
        // index is `rows.len()`.
        if next_row >= total_rows {
            return;
        }
        let started_local_capture = if self.tab().undo.pending_capture.is_none() {
            self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
            true
        } else {
            false
        };
        table.swap_visual_rows(visual_row, next_row);
        table_block.update(cx, move |block, _cx| {
            block.data.table = Some(table.clone());
        });
        self.rebuild_table_grids(cx);
        let selection = TableAxisSelection {
            table_block_id: table_block.entity_id(),
            kind: TableAxis::Row,
            index: next_row,
        };
        self.set_table_axis_selection(Some(selection), cx);
        self.focus_table_cell_position(
            table_block,
            TableCellPosition {
                row: next_row,
                column: 0,
            },
            cx,
        );
        self.mark_dirty(cx);
        self.request_active_block_scroll_into_view(self.active_pane_id(), cx);
        if started_local_capture {
            self.finalize_pending_undo_capture(cx);
        }
        cx.notify();
    }

    pub(crate) fn delete_table_row(
        &mut self,
        table_block: &Entity<Block>,
        row_index: usize,
        cx: &mut Context<Self>,
    ) {
        self.sync_table_data_from_grid(table_block, cx);
        let Some(mut table) = table_block.read(cx).data.table.clone() else {
            return;
        };
        if row_index >= table.rows.len() {
            return;
        }
        let started_local_capture = if self.tab().undo.pending_capture.is_none() {
            self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
            true
        } else {
            false
        };
        table.remove_body_row(row_index);
        let remaining_body_rows = table.rows.len();
        table_block.update(cx, move |block, _cx| {
            block.data.table = Some(table.clone());
        });
        self.rebuild_table_grids(cx);
        // Row selections are addressed by visual index, where the first body row
        // is `1` (the header is `0`). With no body rows left, fall back to the
        // header so focus lands on a cell that still exists.
        let focus_visual_row = if remaining_body_rows == 0 {
            0
        } else {
            row_index.min(remaining_body_rows - 1) + 1
        };
        if remaining_body_rows == 0 {
            self.clear_table_axis_selection(cx);
        } else {
            self.set_table_axis_selection(
                Some(TableAxisSelection {
                    table_block_id: table_block.entity_id(),
                    kind: TableAxis::Row,
                    index: focus_visual_row,
                }),
                cx,
            );
        }
        self.focus_table_cell_position(
            table_block,
            TableCellPosition {
                row: focus_visual_row,
                column: 0,
            },
            cx,
        );
        self.mark_dirty(cx);
        self.request_active_block_scroll_into_view(self.active_pane_id(), cx);
        if started_local_capture {
            self.finalize_pending_undo_capture(cx);
        }
        cx.notify();
    }

    pub(crate) fn delete_table_header_row(
        &mut self,
        table_block: &Entity<Block>,
        cx: &mut Context<Self>,
    ) {
        self.sync_table_data_from_grid(table_block, cx);
        let Some(mut table) = table_block.read(cx).data.table.clone() else {
            return;
        };
        // The first body row is promoted into the header, so there must be at
        // least one body row to delete the header.
        if table.rows.is_empty() {
            return;
        }
        let started_local_capture = if self.tab().undo.pending_capture.is_none() {
            self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
            true
        } else {
            false
        };
        table.remove_header_row();
        table_block.update(cx, move |block, _cx| {
            block.data.table = Some(table.clone());
        });
        self.rebuild_table_grids(cx);
        self.clear_table_axis_selection(cx);
        self.focus_table_cell_position(table_block, TableCellPosition { row: 0, column: 0 }, cx);
        self.mark_dirty(cx);
        self.request_active_block_scroll_into_view(self.active_pane_id(), cx);
        if started_local_capture {
            self.finalize_pending_undo_capture(cx);
        }
        cx.notify();
    }

    pub(crate) fn remove_table_block(
        &mut self,
        table_block: &Entity<Block>,
        cx: &mut Context<Self>,
    ) {
        let Some(location) = self.doc().find_block_location(table_block.entity_id()) else {
            return;
        };
        let started_local_capture = if self.tab().undo.pending_capture.is_none() {
            self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
            true
        } else {
            false
        };
        // Insert the replacement paragraph after the table first, then remove the
        // table, so the document is never momentarily empty.
        let paragraph = Self::new_block(cx, BlockData::paragraph(String::new()));
        self.doc_mut().insert_blocks_at(
            location.parent.clone(),
            location.index + 1,
            vec![paragraph.clone()],
            cx,
        );
        let table_id = table_block.entity_id();
        self.doc_mut().with_structure_mutation(cx, |document, cx| {
            let _ = document.remove_block_unindexed(table_id, cx);
        });
        self.rebuild_table_grids(cx);
        self.clear_table_axis_selection(cx);
        self.focus_block(paragraph.entity_id());
        self.mark_dirty(cx);
        self.request_active_block_scroll_into_view(self.active_pane_id(), cx);
        if started_local_capture {
            self.finalize_pending_undo_capture(cx);
        }
        cx.notify();
    }

    pub(crate) fn insert_table_row_at(
        &mut self,
        table_block: &Entity<Block>,
        visual_row: usize,
        cx: &mut Context<Self>,
    ) {
        self.sync_table_data_from_grid(table_block, cx);
        let Some(mut table) = table_block.read(cx).data.table.clone() else {
            return;
        };
        let started_local_capture = if self.tab().undo.pending_capture.is_none() {
            self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
            true
        } else {
            false
        };
        table.insert_row_at(visual_row);
        table_block.update(cx, move |block, _cx| {
            block.data.table = Some(table.clone());
        });
        self.rebuild_table_grids(cx);
        self.mark_dirty(cx);
        if started_local_capture {
            self.finalize_pending_undo_capture(cx);
        }
        cx.notify();
    }

    pub(crate) fn duplicate_table_row(
        &mut self,
        table_block: &Entity<Block>,
        visual_row: usize,
        cx: &mut Context<Self>,
    ) {
        self.sync_table_data_from_grid(table_block, cx);
        let Some(mut table) = table_block.read(cx).data.table.clone() else {
            return;
        };
        let started_local_capture = if self.tab().undo.pending_capture.is_none() {
            self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
            true
        } else {
            false
        };
        table.duplicate_row(visual_row);
        table_block.update(cx, move |block, _cx| {
            block.data.table = Some(table.clone());
        });
        self.rebuild_table_grids(cx);
        self.mark_dirty(cx);
        if started_local_capture {
            self.finalize_pending_undo_capture(cx);
        }
        cx.notify();
    }

    pub(crate) fn expand_table_block(
        &mut self,
        table_block: &Entity<Block>,
        cx: &mut Context<Self>,
    ) {
        self.sync_table_data_from_grid(table_block, cx);
        let Some(mut table) = table_block.read(cx).data.table.clone() else {
            return;
        };
        let started_local_capture = if self.tab().undo.pending_capture.is_none() {
            self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
            true
        } else {
            false
        };
        table.expand_table();
        table_block.update(cx, move |block, _cx| {
            block.data.table = Some(table.clone());
        });
        self.rebuild_table_grids(cx);
        self.mark_dirty(cx);
        if started_local_capture {
            self.finalize_pending_undo_capture(cx);
        }
        cx.notify();
    }
}
