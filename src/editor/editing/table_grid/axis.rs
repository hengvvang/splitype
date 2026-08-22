//! Table axis selection, hover preview, context menus, and visual synchronization.

use gpui::*;

use crate::editor::controller::{Editor, TableAxisSelection};
use crate::editor::tree::block::Block;
use crate::model::block::table::{TableAxis, TableAxisHighlight, TableAxisMarker};
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
            // Only clear on a leave that still owns the preview. Adjacent
            // handles share one preview slot, and a leave can arrive after
            // the next handle's enter; clearing unconditionally would erase
            // the highlight the pointer just moved onto.
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
        self.set_table_axis_preview(Some(selection), cx);
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
                crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
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
                    table.swap_visual_rows(from, to);
                }
            }
            TableAxis::Column => {
                let total_cols = table.column_count();
                if from < total_cols && to < total_cols {
                    table.swap_columns(from, to);
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
        self.set_table_axis_selection(Some(selection), cx);
        self.mark_dirty(cx);
        self.request_active_block_scroll_into_view(self.active_pane_id(), cx);
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

    pub(crate) fn normalize_table_axis_state(&mut self, cx: &mut Context<Self>) {
        if let Some(selection) = self.tab().tables.axis_selection
            && !self.table_axis_selection_valid(selection, cx)
        {
            self.tab_mut().tables.axis_selection = None;
        }
        if let Some(preview) = self.tab().tables.axis_preview
            && !self.table_axis_selection_valid(preview, cx)
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

            let Some(grid) = table_block.read(cx).table_grid.clone() else {
                continue;
            };

            let selected = self
                .tab()
                .tables
                .axis_selection
                .filter(|selection| selection.table_block_id == block_id);
            let preview = self
                .tab()
                .tables
                .axis_preview
                .filter(|selection| selection.table_block_id == block_id);

            // `row` is the visual row index: `0` is the header and body rows
            // follow at `1..`, matching how row selections are addressed.
            let mut apply_highlight = |cell: &Entity<Block>, row: usize, column: usize| {
                let highlight = if selected.is_some_and(|selection| match selection.kind {
                    TableAxis::Column => selection.index == column,
                    TableAxis::Row => selection.index == row,
                }) {
                    TableAxisHighlight::Selected
                } else if preview.is_some_and(|selection| match selection.kind {
                    TableAxis::Column => selection.index == column,
                    TableAxis::Row => selection.index == row,
                }) {
                    TableAxisHighlight::Preview
                } else {
                    TableAxisHighlight::None
                };

                cell.update(cx, move |block, cx| {
                    block.set_table_axis_highlight(highlight);
                    cx.notify();
                });
            };

            for (column, cell) in grid.header.iter().enumerate() {
                apply_highlight(cell, 0, column);
            }
            for (body_row_index, row) in grid.rows.iter().enumerate() {
                for (column, cell) in row.iter().enumerate() {
                    apply_highlight(cell, body_row_index + 1, column);
                }
            }
        }
    }
}
