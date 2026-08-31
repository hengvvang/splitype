//! Cross-block selection keyboard and clipboard command helpers.

use crate::model::Document;
use crate::input::selection::state::NormalizedCrossBlockSelection;

/// Normalizes two endpoints into ordered start and end positions according to document tree index.
pub fn normalize_cross_block_endpoints(
    doc: &Document,
    start: crate::state::CrossBlockSelectionEndpoint,
    end: crate::state::CrossBlockSelectionEndpoint,
) -> Option<NormalizedCrossBlockSelection> {
    let start_index = doc.index_for_entity_id(start.entity_id)?;
    let end_index = doc.index_for_entity_id(end.entity_id)?;

    if start_index < end_index || (start_index == end_index && start.offset <= end.offset) {
        Some(NormalizedCrossBlockSelection {
            start,
            end,
            start_index,
            end_index,
            reversed: false,
        })
    } else {
        Some(NormalizedCrossBlockSelection {
            start: end,
            end: start,
            start_index: end_index,
            end_index: start_index,
            reversed: true,
        })
    }
}


