//! Pane identifier.

use splitter::LeafId;

/// Strongly-typed identifier representing an editor pane leaf.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PaneId(pub LeafId);

impl PaneId {
    #[inline]
    pub const fn new(id: LeafId) -> Self {
        Self(id)
    }

    #[inline]
    pub const fn leaf_id(self) -> LeafId {
        self.0
    }

    #[inline]
    pub const fn as_u32(self) -> u32 {
        self.0.as_u32()
    }

    #[inline]
    pub const fn as_usize(self) -> usize {
        self.0.as_usize()
    }
}

impl From<LeafId> for PaneId {
    #[inline]
    fn from(id: LeafId) -> Self {
        Self(id)
    }
}

impl From<PaneId> for LeafId {
    #[inline]
    fn from(id: PaneId) -> Self {
        id.0
    }
}

impl From<u32> for PaneId {
    #[inline]
    fn from(id: u32) -> Self {
        Self(LeafId::new(id))
    }
}

impl From<i32> for PaneId {
    #[inline]
    fn from(id: i32) -> Self {
        Self(LeafId::new(id as u32))
    }
}

impl From<usize> for PaneId {
    #[inline]
    fn from(id: usize) -> Self {
        Self(LeafId::from_usize(id))
    }
}

impl From<PaneId> for usize {
    #[inline]
    fn from(id: PaneId) -> Self {
        id.0.as_usize()
    }
}

impl std::fmt::Display for PaneId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
