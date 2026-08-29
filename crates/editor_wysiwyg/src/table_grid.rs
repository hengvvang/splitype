//! Native-table grid data (matrix structure and cell lookup).
//!
//! The grid manipulation logic (axis ops, row/column edits) is
//! editor-side until the `Editor` entity converges; this module holds
//! only the pure matrix type every consumer shares.

use gpui::Entity;

use crate::document::block::Block;
use crate::markdown::block::table::TableCellPosition;

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
