//! Strongly-typed unique identifier for document tree nodes.

use std::fmt;

use uuid::Uuid;

/// Unique identifier of a block in the document tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub Uuid);

impl BlockId {
    /// Generate a new random `BlockId`.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for BlockId {
    fn default() -> Self {
        Self::new()
    }
}
