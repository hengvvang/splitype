//! Interaction block event helpers.

use crate::model::protocol::BlockEvent;

/// Checks if an event is an interaction request (links, footnotes, focus transfers).
pub fn is_interaction_event(event: &BlockEvent) -> bool {
    matches!(
        event,
        BlockEvent::RequestOpenLink { .. }
            | BlockEvent::RequestJumpToFootnoteDefinition { .. }
            | BlockEvent::RequestJumpToFootnoteBackref { .. }
            | BlockEvent::RequestFootnoteTooltip { .. }
            | BlockEvent::RequestFocusPrevious { .. }
            | BlockEvent::RequestFocusNext { .. }
            | BlockEvent::RequestBlockUp
            | BlockEvent::RequestBlockDown
            | BlockEvent::RequestFocus
    )
}

