use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

/// Unique identifier for a Markdown block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockId(pub u64);

static NEXT_BLOCK_ID: AtomicU64 = AtomicU64::new(1);

impl BlockId {
    /// Generates a globally unique sequential block ID.
    #[inline]
    #[must_use]
    pub fn next() -> Self {
        Self(NEXT_BLOCK_ID.fetch_add(1, Ordering::Relaxed))
    }

    /// Creates a BlockId from a raw u64.
    #[inline]
    #[must_use]
    pub const fn from_raw(id: u64) -> Self {
        Self(id)
    }

    /// Returns the raw numerical value.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

impl Default for BlockId {
    fn default() -> Self {
        Self::next()
    }
}

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "block-{}", self.0)
    }
}

/// Unique identifier for an open document buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentId(pub Uuid);

impl DocumentId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for DocumentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for DocumentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Generic element ID wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ElementId(pub u64);

/// Node ID for split tree containers and panes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u64);

static NEXT_NODE_ID: AtomicU64 = AtomicU64::new(1);

impl NodeId {
    #[inline]
    #[must_use]
    pub fn next() -> Self {
        Self(NEXT_NODE_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_id_generation_is_monotonic() {
        let a = BlockId::next();
        let b = BlockId::next();
        assert!(b.raw() > a.raw());
    }

    #[test]
    fn test_document_id_uniqueness() {
        let a = DocumentId::new();
        let b = DocumentId::new();
        assert_ne!(a, b);
    }
}
