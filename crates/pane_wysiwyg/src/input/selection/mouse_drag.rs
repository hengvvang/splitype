//! Mouse drag tracking for cross-block text selection.

use gpui::*;

use crate::state::CrossBlockSelectionEndpoint;

/// Computes the cross-block selection anchor for a given mouse pointer position.
pub fn cross_block_endpoint_for_block_offset(
    entity_id: EntityId,
    offset: usize,
) -> CrossBlockSelectionEndpoint {
    CrossBlockSelectionEndpoint { entity_id, offset }
}


