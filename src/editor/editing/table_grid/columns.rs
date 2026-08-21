//! Table column operations: insert, append, duplicate, move, align, delete.

use gpui::*;

use crate::editor::block_protocol::UndoCaptureKind;
use crate::editor::controller::{Editor, TableAxisSelection};
use crate::editor::tree::block::Block;
use crate::model::block::table::{TableAxis, TableCellPosition, TableColumnAlignment};

impl Editor {
    pub(crate) fn append_table_column(
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
        let alignment = table
            .alignments
            .last()
            .copied()
            .unwrap_or(TableColumnAlignment::Default);
        table.append_column(alignment);

        table_block.update(cx, move |block, _cx| {
            block.data.table = Some(table.clone());
        });
        self.rebuild_table_grids(cx);
        if let Some(cell) = table_block
            .read(cx)
            .table_grid
            .as_ref()
            .and_then(|grid| grid.header.last())
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

    pub(crate) fn set_table_column_alignment(
        &mut self,
        table_block: &Entity<Block>,
        column: usize,
        alignment: TableColumnAlignment,
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
        table.set_column_alignment(column, alignment);
        table_block.update(cx, move |block, _cx| {
            block.data.table = Some(table.clone());
        });
        self.rebuild_table_grids(cx);
        let selection = TableAxisSelection {
            table_block_id: table_block.entity_id(),
            kind: TableAxis::Column,
            index: column,
        };
        self.set_table_axis_selection(Some(selection), cx);
        self.focus_table_cell_position(table_block, TableCellPosition { row: 0, column }, cx);
        self.mark_dirty(cx);
        self.request_active_block_scroll_into_view(self.active_pane_id(), cx);
        if started_local_capture {
            self.finalize_pending_undo_capture(cx);
        }
        cx.notify();
    }

    pub(crate) fn move_table_column(
        &mut self,
        table_block: &Entity<Block>,
        column: usize,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        self.sync_table_data_from_grid(table_block, cx);
        let Some(mut table) = table_block.read(cx).data.table.clone() else {
            return;
        };
        let next_column = if delta < 0 {
            column.checked_sub(delta.unsigned_abs() as usize)
        } else {
            column.checked_add(delta as usize)
        };
        let Some(next_column) = next_column else {
            return;
        };
        if next_column >= table.column_count() {
            return;
        }
        let started_local_capture = if self.tab().undo.pending_capture.is_none() {
            self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
            true
        } else {
            false
        };
        table.swap_columns(column, next_column);
        table_block.update(cx, move |block, _cx| {
            block.data.table = Some(table.clone());
        });
        self.rebuild_table_grids(cx);
        let selection = TableAxisSelection {
            table_block_id: table_block.entity_id(),
            kind: TableAxis::Column,
            index: next_column,
        };
        self.set_table_axis_selection(Some(selection), cx);
        self.focus_table_cell_position(
            table_block,
            TableCellPosition {
                row: 0,
                column: next_column,
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

    pub(crate) fn delete_table_column(
        &mut self,
        table_block: &Entity<Block>,
        column: usize,
        cx: &mut Context<Self>,
    ) {
        self.sync_table_data_from_grid(table_block, cx);
        let Some(mut table) = table_block.read(cx).data.table.clone() else {
            return;
        };
        if table.column_count() <= 1 || column >= table.column_count() {
            return;
        }
        let started_local_capture = if self.tab().undo.pending_capture.is_none() {
            self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
            true
        } else {
            false
        };
        table.remove_column(column);
        let focus_column = column.min(table.column_count().saturating_sub(1));
        table_block.update(cx, move |block, _cx| {
            block.data.table = Some(table.clone());
        });
        self.rebuild_table_grids(cx);
        let selection = TableAxisSelection {
            table_block_id: table_block.entity_id(),
            kind: TableAxis::Column,
            index: focus_column,
        };
        self.set_table_axis_selection(Some(selection), cx);
        self.focus_table_cell_position(
            table_block,
            TableCellPosition {
                row: 0,
                column: focus_column,
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

    pub(crate) fn insert_table_column_at(
        &mut self,
        table_block: &Entity<Block>,
        column: usize,
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
        table.insert_column_at(column, TableColumnAlignment::Default);
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

    pub(crate) fn duplicate_table_column(
        &mut self,
        table_block: &Entity<Block>,
        column: usize,
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
        table.duplicate_column(column);
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
