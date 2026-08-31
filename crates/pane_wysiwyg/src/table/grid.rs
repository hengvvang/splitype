//! TableGrid matrix structure and entity lifecycle binding.

use gpui::*;

use crate::model::block::Block;
use crate::markdown::block::table::{TableCellPosition, TableColumnAlignment, TableData};
use crate::state::TableCellBinding;
use crate::table::TableGrid;

/// Installs the table grid structure onto a Table block entity.
pub fn install_table_grid_for_block(
    table_block: &Entity<Block>,
    table: &TableData,
    mut create_cell: impl FnMut(crate::markdown::inline::text::BlockText, TableCellPosition, TableColumnAlignment) -> (Entity<Block>, TableCellBinding),
    cx: &mut App,
) -> Vec<TableCellBinding> {
    let mut bindings = Vec::new();
    let header = table
        .header
        .iter()
        .cloned()
        .enumerate()
        .map(|(column, text)| {
            let alignment = table
                .alignments
                .get(column)
                .copied()
                .unwrap_or(TableColumnAlignment::Default);
            let position = TableCellPosition { row: 0, column };
            let (cell, binding) = create_cell(text, position, alignment);
            bindings.push(binding);
            cell
        })
        .collect::<Vec<_>>();

    let rows = table
        .rows
        .iter()
        .cloned()
        .enumerate()
        .map(|(body_row_index, row)| {
            row.into_iter()
                .enumerate()
                .map(|(column, text)| {
                    let alignment = table
                        .alignments
                        .get(column)
                        .copied()
                        .unwrap_or(TableColumnAlignment::Default);
                    let position = TableCellPosition {
                        row: body_row_index + 1,
                        column,
                    };
                    let (cell, binding) = create_cell(text, position, alignment);
                    bindings.push(binding);
                    cell
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    table_block.update(cx, {
        let grid = TableGrid { header, rows };
        move |block, _cx| block.set_table_grid(grid.clone())
    });

    bindings
}

/// Syncs the TableData back from the cell block entities in the TableGrid.
pub fn sync_table_data_from_grid(table_block: &Entity<Block>, cx: &mut App) {
    let Some(grid) = table_block.read(cx).table_grid.clone() else {
        return;
    };
    let alignments = table_block
        .read(cx)
        .data
        .table
        .as_ref()
        .map(|table| table.alignments.clone())
        .unwrap_or_default();
    let header = grid
        .header
        .iter()
        .map(|cell| cell.read(cx).data.text.clone())
        .collect::<Vec<_>>();
    let rows = grid
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| cell.read(cx).data.text.clone())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    table_block.update(cx, move |block, _cx| {
        block.data.table = Some(TableData {
            header,
            rows,
            alignments,
        });
    });
}


