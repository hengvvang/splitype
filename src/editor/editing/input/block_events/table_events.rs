//! Table event routing: cell navigation, row/column addition/deletion, and alignment.

use gpui::*;

use crate::editor::block_protocol::BlockEvent;
use crate::editor::controller::*;
use crate::model::parse::BlockKind;

impl Editor {
    pub(crate) fn on_table_event(
        &mut self,
        block: &Entity<crate::editor::tree::block::Block>,
        event: &BlockEvent,
        cx: &mut Context<Self>,
    ) {
        if block.read(cx).kind() != BlockKind::Table {
            return;
        }

        match event {
            BlockEvent::RequestAppendTableColumn => {
                self.prepare_undo_capture(
                    crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
                    cx,
                );
                self.append_table_column(block, cx);
                self.finalize_pending_undo_capture(cx);
            }
            BlockEvent::RequestAppendTableRow => {
                self.prepare_undo_capture(
                    crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
                    cx,
                );
                self.append_table_row(block, cx);
                self.finalize_pending_undo_capture(cx);
            }
            BlockEvent::RequestExpandTable => {
                self.prepare_undo_capture(
                    crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
                    cx,
                );
                self.expand_table_block(block, cx);
                self.finalize_pending_undo_capture(cx);
            }
            BlockEvent::RequestTableAxisPreview {
                kind,
                index,
                hovered,
            } => {
                self.preview_table_axis(block.entity_id(), *kind, *index, *hovered, cx);
            }
            BlockEvent::RequestSelectTableAxis { kind, index } => {
                self.select_table_axis(block.entity_id(), *kind, *index, cx);
            }
            BlockEvent::RequestOpenTableAxisMenu {
                kind,
                index,
                position,
            } => {
                self.open_table_axis_menu(block.entity_id(), *kind, *index, *position, cx);
            }
            _ => {}
        }
    }
}
