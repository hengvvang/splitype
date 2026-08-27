//! Table column operations: insert, append, duplicate, move, align, delete.

use gpui::*;

use crate::editor::engine::controller::{Editor, TableAxisSelection};
use crate::editor::document::block::Block;
use crate::model::block::table::{TableAxis, TableCellPosition, TableColumnAlignment};

impl Editor {
    pub(crate) fn append_table_column(
        &mut self,
        table_block: &Entity<Block>,
        cx: &mut Context<Self>,
    ) {
        self.mutate_table(table_block, cx, |table| {
            let alignment = table
                .alignments
                .last()
                .copied()
                .unwrap_or(TableColumnAlignment::Default);
            table.append_column(alignment);
        });
        if let Some(cell) = table_block
            .read(cx)
            .table_grid
            .as_ref()
            .and_then(|grid| grid.header.last())
        {
            self.focus_block(cell.entity_id());
        }
        self.request_autoscroll_active_pane(
            crate::editor::engine::controller::AutoscrollStrategy::Fit { margin: px(20.0) },
            cx,
        );
    }

    pub(crate) fn set_table_column_alignment(
        &mut self,
        table_block: &Entity<Block>,
        column: usize,
        alignment: TableColumnAlignment,
        cx: &mut Context<Self>,
    ) {
        self.mutate_table(table_block, cx, |table| {
            table.set_column_alignment(column, alignment);
        });
        let selection = TableAxisSelection {
            table_block_id: table_block.entity_id(),
            kind: TableAxis::Column,
            index: column,
        };
        self.set_table_axis_selection(Some(selection), cx);
        self.focus_table_cell_position(table_block, TableCellPosition { row: 0, column }, cx);
        self.request_autoscroll_active_pane(
            crate::editor::engine::controller::AutoscrollStrategy::Fit { margin: px(20.0) },
            cx,
        );
    }

    pub(crate) fn move_table_column(
        &mut self,
        table_block: &Entity<Block>,
        column: usize,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let mut target_col = None;
        self.mutate_table(table_block, cx, |table| {
            let next_column = if delta < 0 {
                column.checked_sub(delta.unsigned_abs() as usize)
            } else {
                column.checked_add(delta as usize)
            };
            if let Some(next_column) = next_column {
                if next_column < table.column_count() {
                    table.swap_columns(column, next_column);
                    target_col = Some(next_column);
                }
            }
        });
        if let Some(next_column) = target_col {
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
            self.request_autoscroll_active_pane(
                crate::editor::engine::controller::AutoscrollStrategy::Fit { margin: px(20.0) },
                cx,
            );
        }
    }

    pub(crate) fn delete_table_column(
        &mut self,
        table_block: &Entity<Block>,
        column: usize,
        cx: &mut Context<Self>,
    ) {
        let mut focus_col = None;
        self.mutate_table(table_block, cx, |table| {
            if table.column_count() > 1 && column < table.column_count() {
                table.remove_column(column);
                focus_col = Some(column.min(table.column_count().saturating_sub(1)));
            }
        });
        if let Some(focus_column) = focus_col {
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
            self.request_autoscroll_active_pane(
                crate::editor::engine::controller::AutoscrollStrategy::Fit { margin: px(20.0) },
                cx,
            );
        }
    }

    pub(crate) fn insert_table_column_at(
        &mut self,
        table_block: &Entity<Block>,
        column: usize,
        cx: &mut Context<Self>,
    ) {
        self.mutate_table(table_block, cx, |table| {
            table.insert_column_at(column, TableColumnAlignment::Default);
        });
    }

    pub(crate) fn duplicate_table_column(
        &mut self,
        table_block: &Entity<Block>,
        column: usize,
        cx: &mut Context<Self>,
    ) {
        self.mutate_table(table_block, cx, |table| {
            table.duplicate_column(column);
        });
    }
}
