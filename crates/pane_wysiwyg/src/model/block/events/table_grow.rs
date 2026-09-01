//! Table row/column append controls: hover state tracking, delayed close
//! scheduling, and the append actions themselves.

use std::time::Duration;

use gpui::*;

use crate::model::block::{Block, TableHoverRegion};
use crate::model::protocol::BlockEvent;
use markdown_parser::parse::BlockKind;

impl Block {
    pub fn schedule_table_append_column_close(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.table_interaction.column_append.is_active {
            return;
        }

        let window_handle = window.window_handle();
        self.table_interaction.column_append.dismiss_task = Some(cx.spawn(
            async move |this: WeakEntity<Block>, cx: &mut AsyncApp| {
                cx.background_executor()
                    .timer(Duration::from_millis(120))
                    .await;
                // try-borrow path: a tick landing mid-render is skipped.
                let _ = window_handle.update(cx, |_view, _window, cx| {
                    let _ = this.update(cx, |block, cx| {
                        block.table_interaction.column_append.dismiss_task = None;
                        if !block.table_interaction.column_append.is_cursor_inside() {
                            block.table_interaction.column_append.is_active = false;
                            cx.notify();
                        }
                    });
                });
            },
        ));
    }

    pub fn schedule_table_append_row_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.table_interaction.row_append.is_active {
            return;
        }

        let window_handle = window.window_handle();
        self.table_interaction.row_append.dismiss_task = Some(cx.spawn(
            async move |this: WeakEntity<Block>, cx: &mut AsyncApp| {
                cx.background_executor()
                    .timer(Duration::from_millis(120))
                    .await;
                // try-borrow path: a tick landing mid-render is skipped.
                let _ = window_handle.update(cx, |_view, _window, cx| {
                    let _ = this.update(cx, |block, cx| {
                        block.table_interaction.row_append.dismiss_task = None;
                        if !block.table_interaction.row_append.is_cursor_inside() {
                            block.table_interaction.row_append.is_active = false;
                            cx.notify();
                        }
                    });
                });
            },
        ));
    }

    pub fn set_table_column_hover_region(
        &mut self,
        region: TableHoverRegion,
        hovered: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let changed = self
            .table_interaction
            .column_append
            .set_region_hovered(region, hovered);

        if !self.table_interaction.column_append.is_cursor_inside()
            && self.table_interaction.column_append.is_active
            && self.table_interaction.column_append.dismiss_task.is_none()
        {
            self.schedule_table_append_column_close(window, cx);
        }

        if changed {
            cx.notify();
        }
    }

    pub fn set_table_row_hover_region(
        &mut self,
        region: TableHoverRegion,
        hovered: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let changed = self
            .table_interaction
            .row_append
            .set_region_hovered(region, hovered);

        if !self.table_interaction.row_append.is_cursor_inside()
            && self.table_interaction.row_append.is_active
            && self.table_interaction.row_append.dismiss_task.is_none()
        {
            self.schedule_table_append_row_close(window, cx);
        }

        if changed {
            cx.notify();
        }
    }

    pub fn on_table_append_column_zone_hover(
        &mut self,
        hovered: &bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_table_column_hover_region(TableHoverRegion::BufferZone, *hovered, window, cx);
    }

    pub fn on_table_append_column_button_hover(
        &mut self,
        hovered: &bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_table_column_hover_region(TableHoverRegion::AppendButton, *hovered, window, cx);
    }

    pub fn on_table_append_row_zone_hover(
        &mut self,
        hovered: &bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_table_row_hover_region(TableHoverRegion::BufferZone, *hovered, window, cx);
    }

    pub fn on_table_append_row_button_hover(
        &mut self,
        hovered: &bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_table_row_hover_region(TableHoverRegion::AppendButton, *hovered, window, cx);
    }

    pub fn on_table_append_column_edge_hover(
        &mut self,
        hovered: &bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_table_column_hover_region(TableHoverRegion::Edge, *hovered, window, cx);
    }

    pub fn on_table_append_row_edge_hover(
        &mut self,
        hovered: &bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_table_row_hover_region(TableHoverRegion::Edge, *hovered, window, cx);
    }

    pub fn on_append_table_column(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.kind() == BlockKind::Table {
            cx.emit(BlockEvent::RequestAppendTableColumn);
        }
    }

    pub fn on_append_table_row(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.kind() == BlockKind::Table {
            cx.emit(BlockEvent::RequestAppendTableRow);
        }
    }
}
