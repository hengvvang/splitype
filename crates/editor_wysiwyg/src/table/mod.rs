//! Native-table grid data, matrix structure, axis ops, column measurement and row/column edits.

pub mod axis;
pub mod columns;
pub mod grid;
pub mod measure;
pub mod rows;

pub use axis::*;
pub use columns::*;
pub use grid::*;
pub use measure::*;
pub use rows::*;

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
