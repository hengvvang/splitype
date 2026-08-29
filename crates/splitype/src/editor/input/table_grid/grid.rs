//! TableGrid matrix structure and entity lifecycle binding.

use gpui::*;

use crate::editor::engine::controller::{Editor, TableCellBinding};
use crate::editor::document::block::Block;
use markdown::block::table::{TableCellPosition, TableColumnAlignment, TableData};
use markdown::parse::{BlockData, BlockKind};

#[derive(Clone)]
pub struct TableGrid {
    pub header: Vec<Entity<Block>>,
    pub rows: Vec<Vec<Entity<Block>>>,
}

impl TableGrid {
    pub fn cell(&self, position: TableCellPosition) -> Option<Entity<Block>> {
        if position.is_header() {
            self.header.get(position.column).cloned()
        } else {
            self.rows
                .get(position.body_row_index()?)
                .and_then(|row| row.get(position.column))
                .cloned()
        }
    }
}

impl Editor {
    pub(crate) fn new_table_block(cx: &mut Context<Self>, table: TableData) -> Entity<Block> {
        Self::new_block(cx, BlockData::table(table))
    }

    pub(crate) fn install_table_grid_for_block(
        &mut self,
        table_block: &Entity<Block>,
        table: &TableData,
        cx: &mut Context<Self>,
    ) {
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
                let cell = Self::new_table_cell_block(cx, text, position, alignment);
                self.tab_mut().tables.cells.insert(
                    cell.entity_id(),
                    TableCellBinding {
                        table_block: table_block.clone(),
                        cell: cell.clone(),
                        position,
                    },
                );
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
                        let cell = Self::new_table_cell_block(cx, text, position, alignment);
                        self.tab_mut().tables.cells.insert(
                            cell.entity_id(),
                            TableCellBinding {
                                table_block: table_block.clone(),
                                cell: cell.clone(),
                                position,
                            },
                        );
                        cell
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        table_block.update(cx, {
            let grid = TableGrid { header, rows };
            move |block, _cx| block.set_table_grid(grid.clone())
        });
    }

    pub(crate) fn rebuild_table_grids(&mut self, cx: &mut Context<Self>) {
        self.tab_mut().tables.cells.clear();
        self.tab_mut().tables.axis_preview = None;
        let mut tables_to_install = Vec::new();
        for entry in self.doc().blocks() {
            entry
                .entity
                .update(cx, |block, _cx| block.clear_table_grid());
            if entry.entity.read(cx).kind() == BlockKind::Table
                && let Some(table) = entry.entity.read(cx).data.table.clone()
            {
                tables_to_install.push((entry.entity.clone(), table));
            }
        }
        for (entity, table) in tables_to_install {
            self.install_table_grid_for_block(&entity, &table, cx);
        }
        // Cells are runtime-only blocks outside the document tree; recreating
        // them invalidates the reference-context sync state so the next
        // rebuild_reference_registries refreshes the new cell entities.
        self.doc_mut().mark_structure_changed();
        self.rebuild_reference_registries(cx);
        self.sync_table_axis_visuals(cx);
    }

    pub(crate) fn sync_table_data_from_grid(
        &mut self,
        table_block: &Entity<Block>,
        cx: &mut Context<Self>,
    ) {
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

    /// Executes an atomic table mutation with automatic undo snapshotting, grid resync, and dirty tracking.
    pub(crate) fn mutate_table<R>(
        &mut self,
        table_block: &Entity<Block>,
        cx: &mut Context<Self>,
        f: impl FnOnce(&mut TableData) -> R,
    ) -> Option<R> {
        self.sync_table_data_from_grid(table_block, cx);
        let mut table = table_block.read(cx).data.table.clone()?;
        let started_local_capture = if self.tab().undo.pending_capture.is_none() {
            self.prepare_undo_capture(
                crate::editor::document::protocol::UndoCaptureKind::NonCoalescible,
                cx,
            );
            true
        } else {
            false
        };
        let result = f(&mut table);
        table_block.update(cx, move |block, _cx| {
            block.data.table = Some(table);
        });
        self.rebuild_table_grids(cx);
        self.mark_dirty(cx);
        if started_local_capture {
            self.finalize_pending_undo_capture(cx);
        }
        cx.notify();
        Some(result)
    }
}
