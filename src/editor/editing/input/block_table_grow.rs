//! Table row/column append controls: hover state tracking, delayed close
//! scheduling, and the append actions themselves.

use std::time::Duration;

use gpui::*;

use crate::editor::block_protocol::BlockAction;
use crate::editor::tree::block::Block;
use crate::model::block::BlockKind;
impl Block {
    fn table_append_column_should_stay_visible(&self) -> bool {
        self.table_append_column_edge_hovered
            || self.table_append_column_zone_hovered
            || self.table_append_column_button_hovered
    }

    fn table_append_row_should_stay_visible(&self) -> bool {
        self.table_append_row_edge_hovered
            || self.table_append_row_zone_hovered
            || self.table_append_row_button_hovered
    }

    fn schedule_table_append_column_close(&mut self, cx: &mut Context<Self>) {
        if !self.table_append_column_hovered {
            return;
        }

        self.table_append_column_close_task = Some(cx.spawn(
            async |this: WeakEntity<Block>, cx: &mut AsyncApp| {
                cx.background_executor()
                    .timer(Duration::from_millis(120))
                    .await;
                let _ = this.update(cx, |block, cx| {
                    block.table_append_column_close_task = None;
                    if !block.table_append_column_should_stay_visible() {
                        block.table_append_column_hovered = false;
                        cx.notify();
                    }
                });
            },
        ));
    }

    fn schedule_table_append_row_close(&mut self, cx: &mut Context<Self>) {
        if !self.table_append_row_hovered {
            return;
        }

        self.table_append_row_close_task = Some(cx.spawn(
            async |this: WeakEntity<Block>, cx: &mut AsyncApp| {
                cx.background_executor()
                    .timer(Duration::from_millis(120))
                    .await;
                let _ = this.update(cx, |block, cx| {
                    block.table_append_row_close_task = None;
                    if !block.table_append_row_should_stay_visible() {
                        block.table_append_row_hovered = false;
                        cx.notify();
                    }
                });
            },
        ));
    }

    fn set_table_append_column_hover_part(
        &mut self,
        edge_hovered: Option<bool>,
        zone_hovered: Option<bool>,
        button_hovered: Option<bool>,
        cx: &mut Context<Self>,
    ) {
        let mut changed = false;
        if let Some(edge_hovered) = edge_hovered
            && self.table_append_column_edge_hovered != edge_hovered
        {
            self.table_append_column_edge_hovered = edge_hovered;
            changed = true;
        }
        if let Some(zone_hovered) = zone_hovered
            && self.table_append_column_zone_hovered != zone_hovered
        {
            self.table_append_column_zone_hovered = zone_hovered;
            changed = true;
        }
        if let Some(button_hovered) = button_hovered
            && self.table_append_column_button_hovered != button_hovered
        {
            self.table_append_column_button_hovered = button_hovered;
            changed = true;
        }

        if self.table_append_column_should_stay_visible() {
            self.table_append_column_close_task = None;
            if !self.table_append_column_hovered {
                self.table_append_column_hovered = true;
                changed = true;
            }
        } else if self.table_append_column_hovered && self.table_append_column_close_task.is_none()
        {
            self.schedule_table_append_column_close(cx);
        }

        if changed {
            cx.notify();
        }
    }

    fn set_table_append_row_hover_part(
        &mut self,
        edge_hovered: Option<bool>,
        zone_hovered: Option<bool>,
        button_hovered: Option<bool>,
        cx: &mut Context<Self>,
    ) {
        let mut changed = false;
        if let Some(edge_hovered) = edge_hovered
            && self.table_append_row_edge_hovered != edge_hovered
        {
            self.table_append_row_edge_hovered = edge_hovered;
            changed = true;
        }
        if let Some(zone_hovered) = zone_hovered
            && self.table_append_row_zone_hovered != zone_hovered
        {
            self.table_append_row_zone_hovered = zone_hovered;
            changed = true;
        }
        if let Some(button_hovered) = button_hovered
            && self.table_append_row_button_hovered != button_hovered
        {
            self.table_append_row_button_hovered = button_hovered;
            changed = true;
        }

        if self.table_append_row_should_stay_visible() {
            self.table_append_row_close_task = None;
            if !self.table_append_row_hovered {
                self.table_append_row_hovered = true;
                changed = true;
            }
        } else if self.table_append_row_hovered && self.table_append_row_close_task.is_none() {
            self.schedule_table_append_row_close(cx);
        }

        if changed {
            cx.notify();
        }
    }

    pub(crate) fn on_table_append_column_zone_hover(
        &mut self,
        hovered: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_table_append_column_hover_part(None, Some(*hovered), None, cx);
    }

    pub(crate) fn on_table_append_column_button_hover(
        &mut self,
        hovered: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_table_append_column_hover_part(None, None, Some(*hovered), cx);
    }

    pub(crate) fn on_table_append_row_zone_hover(
        &mut self,
        hovered: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_table_append_row_hover_part(None, Some(*hovered), None, cx);
    }

    pub(crate) fn on_table_append_row_button_hover(
        &mut self,
        hovered: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_table_append_row_hover_part(None, None, Some(*hovered), cx);
    }

    pub(crate) fn on_table_append_column_edge_hover(
        &mut self,
        hovered: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_table_append_column_hover_part(Some(*hovered), None, None, cx);
    }

    pub(crate) fn on_table_append_row_edge_hover(
        &mut self,
        hovered: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_table_append_row_hover_part(Some(*hovered), None, None, cx);
    }

    pub(crate) fn on_append_table_column(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.kind() == BlockKind::Table {
            cx.emit(BlockAction::RequestAppendTableColumn);
        }
    }

    pub(crate) fn on_append_table_row(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.kind() == BlockKind::Table {
            cx.emit(BlockAction::RequestAppendTableRow);
        }
    }

    pub(crate) fn on_expand_table(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.kind() == BlockKind::Table {
            cx.emit(BlockAction::RequestExpandTable);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Block;
    use crate::model::block::BlockData;
    use gpui::{AppContext, TestAppContext};

    #[gpui::test]
    async fn append_column_button_stays_visible_while_crossing_hover_gap(cx: &mut TestAppContext) {
        let block = cx.new(|cx| Block::with_data(cx, BlockData::paragraph(String::new())));

        block.update(cx, |block, cx| {
            block.set_table_append_column_hover_part(Some(true), None, None, cx);
            assert!(block.table_append_column_hovered);

            block.set_table_append_column_hover_part(Some(false), None, Some(true), cx);
            assert!(block.table_append_column_hovered);
            assert!(!block.table_append_column_edge_hovered);
            assert!(!block.table_append_column_zone_hovered);
            assert!(block.table_append_column_button_hovered);
            assert!(block.table_append_column_close_task.is_none());
        });
    }
    #[gpui::test]
    async fn append_row_button_stays_visible_while_crossing_hover_gap(cx: &mut TestAppContext) {
        let block = cx.new(|cx| Block::with_data(cx, BlockData::paragraph(String::new())));

        block.update(cx, |block, cx| {
            block.set_table_append_row_hover_part(Some(true), None, None, cx);
            assert!(block.table_append_row_hovered);

            block.set_table_append_row_hover_part(Some(false), None, Some(true), cx);
            assert!(block.table_append_row_hovered);
            assert!(!block.table_append_row_edge_hovered);
            assert!(!block.table_append_row_zone_hovered);
            assert!(block.table_append_row_button_hovered);
            assert!(block.table_append_row_close_task.is_none());
        });
    }

    #[gpui::test]
    async fn column_edge_hover_reveals_only_column_append_control(cx: &mut TestAppContext) {
        let block = cx.new(|cx| Block::with_data(cx, BlockData::paragraph(String::new())));

        block.update(cx, |block, cx| {
            block.set_table_append_column_hover_part(Some(true), None, None, cx);
            assert!(block.table_append_column_edge_hovered);
            assert!(block.table_append_column_hovered);
            assert!(!block.table_append_row_hovered);
            assert!(block.table_append_column_close_task.is_none());
            assert!(block.table_append_row_close_task.is_none());
        });
    }

    #[gpui::test]
    async fn row_edge_hover_reveals_only_row_append_control(cx: &mut TestAppContext) {
        let block = cx.new(|cx| Block::with_data(cx, BlockData::paragraph(String::new())));

        block.update(cx, |block, cx| {
            block.set_table_append_row_hover_part(Some(true), None, None, cx);
            assert!(block.table_append_row_edge_hovered);
            assert!(block.table_append_row_hovered);
            assert!(!block.table_append_column_hovered);
            assert!(block.table_append_column_close_task.is_none());
            assert!(block.table_append_row_close_task.is_none());
        });
    }
}
