//! Text editing block events and formatting transformations.

use crate::markdown::parse::BlockKind;

/// Prepares trailing text for sibling blocks on Enter key press.
pub fn sibling_kind_on_newline(kind: BlockKind) -> BlockKind {
    kind.newline_sibling_kind()
}


