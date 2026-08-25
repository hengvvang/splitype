//! Table event routing: cell navigation, row/column addition/deletion, and alignment.

use gpui::*;

use crate::editor::document::protocol::BlockEvent;
use crate::editor::engine::controller::*;
use crate::model::parse::BlockKind;

impl Editor {
    pub(crate) fn on_table_event(
        &mut self,
        block: &Entity<crate::editor::document::block::Block>,
        event: &BlockEvent,
        cx: &mut Context<Self>,
    ) {
        if block.read(cx).kind() != BlockKind::Table {
            return;
        }

        match event {
            BlockEvent::RequestAppendTableColumn => {
                self.prepare_undo_capture(
                    crate::editor::document::protocol::UndoCaptureKind::NonCoalescible,
                    cx,
                );
                self.append_table_column(block, cx);
                self.finalize_pending_undo_capture(cx);
            }
            BlockEvent::RequestAppendTableRow => {
                self.prepare_undo_capture(
                    crate::editor::document::protocol::UndoCaptureKind::NonCoalescible,
                    cx,
                );
                self.append_table_row(block, cx);
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
            BlockEvent::RequestOpenTableSizePicker { position } => {
                self.open_table_size_picker(block.entity_id(), *position, cx);
            }
            BlockEvent::RequestReorderTableAxis { kind, from, to } => {
                self.reorder_table_axis(block, *kind, *from, *to, cx);
            }
            BlockEvent::RequestInsertTableAxisAt { kind, index } => match kind {
                crate::model::block::table::TableAxis::Column => {
                    self.insert_table_column_at(block, *index, cx);
                }
                crate::model::block::table::TableAxis::Row => {
                    self.insert_table_row_at(block, *index, cx);
                }
            },
            _ => {}
        }
    }
}
