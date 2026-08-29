//! Table row operations: insert, append, duplicate, move, delete, expand.

use gpui::*;

use crate::editor::document::protocol::UndoCaptureKind;
use crate::editor::engine::controller::{Editor, TableAxisSelection};
use crate::editor::document::block::Block;
use markdown::block::table::{TableAxis, TableCellPosition};
use markdown::parse::BlockData;

impl Editor {
    pub(crate) fn append_table_row(&mut self, table_block: &Entity<Block>, cx: &mut Context<Self>) {
        self.mutate_table(table_block, cx, |table| {
            table.append_row();
        });
        if let Some(cell) = table_block
            .read(cx)
            .table_grid
            .as_ref()
            .and_then(|grid| grid.rows.last())
            .and_then(|row| row.first())
        {
            self.focus_block(cell.entity_id());
        }
        self.request_autoscroll_active_pane(
            crate::editor::engine::controller::AutoscrollStrategy::Fit { margin: px(20.0) },
            cx,
        );
    }

    pub(crate) fn move_table_row(
        &mut self,
        table_block: &Entity<Block>,
        visual_row: usize,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let mut target_row = None;
        self.mutate_table(table_block, cx, |table| {
            let total_rows = table.rows.len() + 1;
            let next_row = if delta < 0 {
                visual_row.checked_sub(delta.unsigned_abs() as usize)
            } else {
                visual_row.checked_add(delta as usize)
            };
            if let Some(next_row) = next_row {
                if next_row < total_rows {
                    table.swap_visual_rows(visual_row, next_row);
                    target_row = Some(next_row);
                }
            }
        });
        if let Some(next_row) = target_row {
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
            self.request_autoscroll_active_pane(
                crate::editor::engine::controller::AutoscrollStrategy::Fit { margin: px(20.0) },
                cx,
            );
        }
    }

    pub(crate) fn delete_table_row(
        &mut self,
        table_block: &Entity<Block>,
        row_index: usize,
        cx: &mut Context<Self>,
    ) {
        let mut remaining_rows = 0;
        self.mutate_table(table_block, cx, |table| {
            if row_index < table.rows.len() {
                table.remove_body_row(row_index);
            }
            remaining_rows = table.rows.len();
        });
        let focus_visual_row = if remaining_rows == 0 {
            0
        } else {
            row_index.min(remaining_rows - 1) + 1
        };
        if remaining_rows == 0 {
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
        self.request_autoscroll_active_pane(
            crate::editor::engine::controller::AutoscrollStrategy::Fit { margin: px(20.0) },
            cx,
        );
    }

    pub(crate) fn delete_table_header_row(
        &mut self,
        table_block: &Entity<Block>,
        cx: &mut Context<Self>,
    ) {
        let mut deleted = false;
        self.mutate_table(table_block, cx, |table| {
            if !table.rows.is_empty() {
                deleted = table.remove_header_row();
            }
        });
        if deleted {
            self.clear_table_axis_selection(cx);
            self.focus_table_cell_position(table_block, TableCellPosition { row: 0, column: 0 }, cx);
            self.request_autoscroll_active_pane(
                crate::editor::engine::controller::AutoscrollStrategy::Fit { margin: px(20.0) },
                cx,
            );
        }
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
        self.request_autoscroll_active_pane(
            crate::editor::engine::controller::AutoscrollStrategy::Fit { margin: px(20.0) },
            cx,
        );
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
        self.mutate_table(table_block, cx, |table| {
            table.insert_row_at(visual_row);
        });
    }

    pub(crate) fn duplicate_table_row(
        &mut self,
        table_block: &Entity<Block>,
        visual_row: usize,
        cx: &mut Context<Self>,
    ) {
        self.mutate_table(table_block, cx, |table| {
            table.duplicate_row(visual_row);
        });
    }
}
