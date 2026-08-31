//! Table axis selection, hover preview, and visual markers.

use gpui::*;

use crate::model::block::Block;
use crate::markdown::block::table::{TableAxis, TableAxisMarker};
use crate::state::TableAxisSelection;

/// Converts a selection into an axis marker.
pub fn table_axis_marker(selection: TableAxisSelection) -> TableAxisMarker {
    TableAxisMarker {
        kind: selection.kind,
        index: selection.index,
    }
}

/// Checks whether an axis selection is valid for the given table block.
pub fn table_axis_selection_valid(
    table_block: &Entity<Block>,
    selection: TableAxisSelection,
    cx: &App,
) -> bool {
    let Some(grid) = table_block.read(cx).table_grid.as_ref() else {
        return false;
    };
    match selection.kind {
        TableAxis::Column => selection.index < grid.header.len(),
        TableAxis::Row => selection.index <= grid.rows.len(),
    }
}

/// Checks whether an axis preview insertion boundary is valid for the given table block.
pub fn table_axis_preview_valid(
    table_block: &Entity<Block>,
    preview: TableAxisSelection,
    cx: &App,
) -> bool {
    let Some(grid) = table_block.read(cx).table_grid.as_ref() else {
        return false;
    };
    match preview.kind {
        TableAxis::Column => preview.index <= grid.header.len(),
        TableAxis::Row => preview.index <= grid.rows.len() + 1,
    }
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


