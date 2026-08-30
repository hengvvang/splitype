//! Structural block event declarations and helper predicates.

use crate::markdown::parse::BlockKind;

/// Checks if a block kind can nest under a target parent block.
pub fn can_nest_under(child_kind: BlockKind, parent_kind: BlockKind) -> bool {
    child_kind.can_nest_under(&parent_kind)
}
