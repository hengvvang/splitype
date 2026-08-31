//! Table event category helpers.

use crate::model::protocol::BlockEvent;

/// Checks if an event is a table-specific event.
pub fn is_table_event(event: &BlockEvent) -> bool {
    matches!(
        event,
        BlockEvent::RequestAppendTableColumn
            | BlockEvent::RequestAppendTableRow
            | BlockEvent::RequestTableAxisPreview { .. }
            | BlockEvent::RequestSelectTableAxis { .. }
            | BlockEvent::RequestOpenTableAxisMenu { .. }
            | BlockEvent::RequestOpenTableSizePicker { .. }
            | BlockEvent::RequestReorderTableAxis { .. }
            | BlockEvent::RequestInsertTableAxisAt { .. }
    )
}
