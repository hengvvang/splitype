//! Table axis selection, hover preview, and visual markers.

use gpui::*;

use crate::model::block::Block;
use markdown_parser::block::table::TableAxis;

/// Selected row or column in a rendered native table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableAxisSelection {
    pub table_block_id: EntityId,
    pub kind: TableAxis,
    pub index: usize,
}

/// Moves an axis (row or column) in the table block data.
pub fn reorder_table_axis(
    table_block: &Entity<Block>,
    kind: TableAxis,
    from: usize,
    to: usize,
    cx: &mut App,
) {
    if from == to {
        return;
    }
    let Some(mut table) = table_block.read(cx).data.table.clone() else {
        return;
    };
    match kind {
        TableAxis::Row => {
            let total_rows = table.rows.len() + 1;
            if from < total_rows && to < total_rows {
                table.move_visual_row(from, to);
            }
        }
        TableAxis::Column => {
            let total_cols = table.column_count();
            if from < total_cols && to < total_cols {
                table.move_column(from, to);
            }
        }
    }
    table_block.update(cx, move |block, _cx| {
        block.data.table = Some(table);
    });
}
