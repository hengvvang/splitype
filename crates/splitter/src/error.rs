//! Domain error types and Result alias for layout operations.

use thiserror::Error;

use crate::id::{LeafId, SplitId};

/// Errors that can occur during layout tree queries, splits, joins, and mutations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum SplitterError {
    /// The specified leaf was not found in the layout tree.
    #[error("Target leaf with id {0} was not found in layout")]
    LeafNotFound(LeafId),

    /// The specified split divider was not found in the layout tree.
    #[error("Target split divider with id {0} was not found in layout")]
    SplitNotFound(SplitId),

    /// Attempted to close, join, or remove the only remaining leaf in the layout.
    #[error("Cannot remove or collapse the sole remaining leaf in the layout")]
    CannotRemoveLastLeaf,

    /// An operation was attempted while a leaf is currently maximized.
    #[error("Layout is currently maximized by leaf {0}; operation is prohibited")]
    LayoutMaximized(LeafId),

    /// Two leaves do not share an immediate split divider and cannot be joined.
    #[error("Leaves {0} and {1} do not share an immediate split border and cannot be joined")]
    NotAdjacent(LeafId, LeafId),

    /// The specified split ratio is out of bounds (must be within (0.0, 1.0)).
    #[error("Invalid ratio {0}; ratio must be between 0.01 and 0.99")]
    InvalidRatio(f32),

    /// Source and target leaves are identical for an operation requiring two distinct leaves.
    #[error("Target leaf {0} is identical to source leaf; cannot perform operation on self")]
    SameLeaf(LeafId),
}

/// Convenience Result alias for layout operations.
pub type Result<T> = std::result::Result<T, SplitterError>;
