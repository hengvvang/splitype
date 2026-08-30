//! WYSIWYG table grid binding and rebuild management for editor_core.

use gpui::*;

use crate::engine::controller::Editor;
use editor_wysiwyg::document::block::Block;
use editor_wysiwyg::document::protocol::BlockEvent;
use editor_wysiwyg::markdown::block::table::{TableCellPosition, TableColumnAlignment, TableData};
use editor_wysiwyg::markdown::inline::text::BlockText;
use editor_wysiwyg::markdown::parse::{BlockData, BlockKind};
use editor_wysiwyg::state::TableCellBinding;
use editor_wysiwyg::table_grid::TableGrid;

impl Editor {
    /// Creates a new table block entity.
    pub fn new_table_block(cx: &mut Context<Self>, table: TableData) -> Entity<Block> {
        Self::new_block(cx, BlockData::table(table))
    }

    /// Creates a new table cell block entity.
    pub fn new_table_cell_block(
        cx: &mut Context<Self>,
        text: BlockText,
        position: TableCellPosition,
        alignment: TableColumnAlignment,
    ) -> Entity<Block> {
        let block = Self::new_block(cx, BlockData::new(BlockKind::Paragraph, text));
        block.update(cx, |block, _cx| {
            block.set_table_cell_mode(position, alignment);
        });
        block
    }

    /// Installs table cell blocks and the TableGrid runtime onto a Table block entity.
    pub fn install_table_grid_for_block(
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

    /// Rebuilds table grids for all Table blocks in the active document.
    pub fn rebuild_table_grids(&mut self, cx: &mut Context<Self>) {
        if !self.session.has_tabs() {
            return;
        }
        self.tab_mut().tables.cells.clear();
        self.tab_mut().tables.axis_preview = None;
        let mut tables_to_install = Vec::new();
        if let Some(doc) = self.active_doc() {
            for entry in doc.blocks() {
                entry
                    .entity
                    .update(cx, |block, _cx| block.clear_table_grid());
                if entry.entity.read(cx).kind() == BlockKind::Table
                    && let Some(table) = entry.entity.read(cx).data.table.clone()
                {
                    tables_to_install.push((entry.entity.clone(), table));
                }
            }
        }
        for (entity, table) in tables_to_install {
            self.install_table_grid_for_block(&entity, &table, cx);
        }
    }

    /// Synchronizes TableData back to the table block entity from its cell blocks.
    pub fn sync_table_data_from_grid(
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

    pub fn table_cell_binding(&self, entity_id: EntityId) -> Option<TableCellBinding> {
        self.session.active_tab().and_then(|t| t.tables.cells.get(&entity_id).cloned())
    }

    pub fn focus_table_cell_position(
        &mut self,
        table_block: &Entity<Block>,
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
        self.focus_wysiwyg_block(cell.entity_id());
        cx.notify();
        true
    }

    pub fn on_table_cell_event(
        &mut self,
        binding: TableCellBinding,
        event: &BlockEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            BlockEvent::Changed => {
                self.sync_table_data_from_grid(&binding.table_block, cx);
                self.tab_mut().file.dirty = true;
                self.tab_mut().text_stale = true;
                self.request_autoscroll_active_pane(
                    crate::engine::controller::AutoscrollStrategy::Fit { margin: px(20.0) },
                    cx,
                );
                cx.notify();
            }
            BlockEvent::RequestFocus => {
                self.focus_wysiwyg_block(binding.cell.entity_id());
                cx.notify();
            }
            BlockEvent::RequestTableCellMoveHorizontal { delta } => {
                let current = binding.position;
                let table_block = binding.table_block.clone();
                let Some(grid) = table_block.read(cx).table_grid.clone() else {
                    return;
                };
                let col_count = grid.header.len();
                if col_count == 0 {
                    return;
                }
                let next_col = if *delta > 0 {
                    (current.column + 1).min(col_count - 1)
                } else {
                    current.column.saturating_sub(1)
                };
                let next_pos = TableCellPosition {
                    row: current.row,
                    column: next_col,
                };
                self.focus_table_cell_position(&table_block, next_pos, cx);
            }
            BlockEvent::RequestTableCellMoveVertical { delta } => {
                let current = binding.position;
                let table_block = binding.table_block.clone();
                let Some(grid) = table_block.read(cx).table_grid.clone() else {
                    return;
                };
                let total_rows = grid.rows.len() + 1;
                let next_row = if *delta > 0 {
                    (current.row + 1).min(total_rows - 1)
                } else {
                    current.row.saturating_sub(1)
                };
                let next_pos = TableCellPosition {
                    row: next_row,
                    column: current.column,
                };
                self.focus_table_cell_position(&table_block, next_pos, cx);
            }
            _ => {}
        }
    }
}
