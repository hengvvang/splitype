//! Table axis selection, hover preview, context menus, and visual synchronization.

use gpui::*;

use crate::editor::engine::controller::{Editor, TableAxisSelection};
use crate::editor::document::block::Block;
use crate::model::block::table::{TableAxis, TableAxisMarker};
use crate::model::parse::BlockKind;

impl Editor {
    pub(crate) fn preview_table_axis(
        &mut self,
        table_block_id: EntityId,
        kind: TableAxis,
        index: usize,
        hovered: bool,
        cx: &mut Context<Self>,
    ) {
        let marker = TableAxisSelection {
            table_block_id,
            kind,
            index,
        };
        if hovered {
            self.set_table_axis_preview(Some(marker), cx);
        } else if self.tab().tables.axis_preview == Some(marker) {
            self.set_table_axis_preview(None, cx);
        }
    }

    pub(crate) fn select_table_axis(
        &mut self,
        table_block_id: EntityId,
        kind: TableAxis,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        let selection = TableAxisSelection {
            table_block_id,
            kind,
            index,
        };
        self.set_table_axis_preview(None, cx);
        self.set_table_axis_selection(Some(selection), cx);
    }

    pub(crate) fn open_table_axis_menu(
        &mut self,
        table_block_id: EntityId,
        kind: TableAxis,
        index: usize,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        self.select_table_axis(table_block_id, kind, index, cx);
        if let Some(selection) = self.tab().tables.axis_selection {
            self.open_table_axis_context_menu(position, selection, cx);
        }
    }

    pub(crate) fn reorder_table_axis(
        &mut self,
        table_block: &Entity<Block>,
        kind: TableAxis,
        from: usize,
        to: usize,
        cx: &mut Context<Self>,
    ) {
        if from == to {
            return;
        }
        self.sync_table_data_from_grid(table_block, cx);
        let Some(mut table) = table_block.read(cx).data.table.clone() else {
            return;
        };
        let started_local_capture = if self.tab().undo.pending_capture.is_none() {
            self.prepare_undo_capture(
                crate::editor::document::protocol::UndoCaptureKind::NonCoalescible,
                cx,
            );
            true
        } else {
            false
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
            block.data.table = Some(table.clone());
        });
        self.rebuild_table_grids(cx);
        let selection = TableAxisSelection {
            table_block_id: table_block.entity_id(),
            kind,
            index: to,
        };
        self.tab_mut().tables.axis_preview = None;
        self.set_table_axis_selection(Some(selection), cx);
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

    pub(crate) fn table_axis_marker(selection: TableAxisSelection) -> TableAxisMarker {
        TableAxisMarker {
            kind: selection.kind,
            index: selection.index,
        }
    }

    pub(crate) fn clear_table_axis_preview(&mut self, cx: &mut Context<Self>) {
        if self.tab_mut().tables.axis_preview.take().is_some() {
            self.sync_table_axis_visuals(cx);
        }
    }

    pub(crate) fn clear_table_axis_selection(&mut self, cx: &mut Context<Self>) {
        if self.tab_mut().tables.axis_selection.take().is_some() {
            self.sync_table_axis_visuals(cx);
        }
    }

    pub(crate) fn set_table_axis_preview(
        &mut self,
        preview: Option<TableAxisSelection>,
        cx: &mut Context<Self>,
    ) {
        if self.tab().tables.axis_preview != preview {
            self.tab_mut().tables.axis_preview = preview;
            self.sync_table_axis_visuals(cx);
        }
    }

    pub(crate) fn set_table_axis_selection(
        &mut self,
        selection: Option<TableAxisSelection>,
        cx: &mut Context<Self>,
    ) {
        if self.tab().tables.axis_selection != selection {
            self.tab_mut().tables.axis_selection = selection;
            self.sync_table_axis_visuals(cx);
        }
    }

    pub(crate) fn table_axis_selection_valid(
        &self,
        selection: TableAxisSelection,
        cx: &App,
    ) -> bool {
        let Some(table_block) = self.table_block_by_id(selection.table_block_id, cx) else {
            return false;
        };
        let Some(grid) = table_block.read(cx).table_grid.as_ref() else {
            return false;
        };
        match selection.kind {
            TableAxis::Column => selection.index < grid.header.len(),
            // Visual row index: `0` is the header, `1..=rows.len()` the body.
            TableAxis::Row => selection.index <= grid.rows.len(),
        }
    }

    pub(crate) fn table_axis_preview_valid(
        &self,
        preview: TableAxisSelection,
        cx: &App,
    ) -> bool {
        let Some(table_block) = self.table_block_by_id(preview.table_block_id, cx) else {
            return false;
        };
        let Some(grid) = table_block.read(cx).table_grid.as_ref() else {
            return false;
        };
        match preview.kind {
            // Insertion boundary can be from 0 up to grid.header.len() (after the last column)
            TableAxis::Column => preview.index <= grid.header.len(),
            // Insertion boundary can be from 0 up to grid.rows.len() + 1 (after the last row)
            TableAxis::Row => preview.index <= grid.rows.len() + 1,
        }
    }

    pub(crate) fn normalize_table_axis_state(&mut self, cx: &mut Context<Self>) {
        if let Some(selection) = self.tab().tables.axis_selection
            && !self.table_axis_selection_valid(selection, cx)
        {
            self.tab_mut().tables.axis_selection = None;
        }
        if let Some(preview) = self.tab().tables.axis_preview
            && !self.table_axis_preview_valid(preview, cx)
        {
            self.tab_mut().tables.axis_preview = None;
        }
    }

    pub(crate) fn sync_table_axis_visuals(&mut self, cx: &mut Context<Self>) {
        self.normalize_table_axis_state(cx);

        let visible_tables = self
            .doc()
            .blocks()
            .iter()
            .filter(|entry| entry.entity.read(cx).kind() == BlockKind::Table)
            .map(|entry| entry.entity.clone())
            .collect::<Vec<_>>();

        for table_block in &visible_tables {
            let block_id = table_block.entity_id();
            let preview_marker = self
                .tab()
                .tables
                .axis_preview
                .filter(|selection| selection.table_block_id == block_id)
                .map(Self::table_axis_marker);
            let selected_marker = self
                .tab()
                .tables
                .axis_selection
                .filter(|selection| selection.table_block_id == block_id)
                .map(Self::table_axis_marker);

            table_block.update(cx, move |block, cx| {
                block.set_table_axis_visual_state(preview_marker, selected_marker);
                cx.notify();
            });
        }
    }

    pub(crate) fn open_table_size_picker(
        &mut self,
        table_block_id: EntityId,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let Some(table_block) = self.table_block_by_id(table_block_id, cx) else {
            return;
        };
        let (current_rows, current_cols) = if let Some(table) = table_block.read(cx).data.table.as_ref() {
            (table.rows.len() + 1, table.column_count())
        } else if let Some(grid) = table_block.read(cx).table_grid.as_ref() {
            (grid.rows.len() + 1, grid.header.len())
        } else {
            (2, 2)
        };

        self.table_size_picker = Some(crate::editor::engine::controller::TableSizePickerState {
            table_block_id,
            position,
            current_rows,
            current_cols,
            hovered_rows: None,
            hovered_cols: None,
        });
        cx.notify();
    }

    pub(crate) fn close_table_size_picker(&mut self, cx: &mut Context<Self>) {
        if self.table_size_picker.take().is_some() {
            cx.notify();
        }
    }

    pub(crate) fn set_table_size_picker_hover(
        &mut self,
        rows: Option<usize>,
        cols: Option<usize>,
        cx: &mut Context<Self>,
    ) {
        if let Some(picker) = self.table_size_picker.as_mut() {
            picker.hovered_rows = rows;
            picker.hovered_cols = cols;
            cx.notify();
        }
    }

    pub(crate) fn resize_table(
        &mut self,
        table_block_id: EntityId,
        target_rows: usize,
        target_cols: usize,
        cx: &mut Context<Self>,
    ) {
        self.close_table_size_picker(cx);
        let Some(table_block) = self.table_block_by_id(table_block_id, cx) else {
            return;
        };
        self.mutate_table(&table_block, cx, |table| {
            table.resize_shape(target_rows, target_cols);
        });
    }
}
