//! Native-table cell state helpers on [`Block`].
//!
//! Pure block methods (cell position/alignment, text alignment, grid
//! binding); the table manipulation flows that orchestrate them stay
//! editor-side until the `Editor` entity converges.

use gpui::TextAlign;

use crate::model::block::{Block, BlockEditMode};
use crate::table::TableGrid;
use crate::markdown::block::table::{TableAxisMarker, TableCellPosition, TableColumnAlignment};

impl Block {
    pub fn is_table_cell(&self) -> bool {
        self.table_cell_position.is_some()
    }

    pub fn table_cell_position(&self) -> Option<TableCellPosition> {
        self.table_cell_position
    }

    pub fn table_cell_alignment(&self) -> Option<TableColumnAlignment> {
        self.table_cell_alignment
    }

    pub fn text_align(&self) -> TextAlign {
        match self
            .table_cell_alignment()
            .unwrap_or(TableColumnAlignment::Default)
        {
            TableColumnAlignment::Default | TableColumnAlignment::Left => TextAlign::Left,
            TableColumnAlignment::Center => TextAlign::Center,
            TableColumnAlignment::Right => TextAlign::Right,
        }
    }

    pub fn set_table_cell_mode(
        &mut self,
        position: TableCellPosition,
        alignment: TableColumnAlignment,
    ) {
        self.table_cell_position = Some(position);
        self.table_cell_alignment = Some(alignment);
        self.edit_mode = BlockEditMode::RenderedRich;
        self.clear_inline_projection();
        self.sync_render_cache();
    }

    pub fn set_table_grid(&mut self, runtime: TableGrid) {
        self.table_grid = Some(runtime);
    }

    pub fn clear_table_grid(&mut self) {
        self.table_grid = None;
        self.table_axis_preview = None;
        self.table_axis_selection = None;
        self.table_interaction.clear();
    }

    pub fn set_table_axis_visual_state(
        &mut self,
        preview: Option<TableAxisMarker>,
        selection: Option<TableAxisMarker>,
    ) {
        self.table_axis_preview = preview;
        self.table_axis_selection = selection;
    }
}

