//! Strong-typed node identification system for layout leaves and split dividers.
//!
//! Eliminates primitive obsession (`usize`) by separating [`LeafId`] (leaf panel containers)
//! from [`SplitId`] (internal split dividers), with a unified [`NodeId`] tagged enum.

use std::fmt;

use gpui::ElementId;

/// Strongly-typed identifier representing a leaf panel container.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(transparent)]
pub struct LeafId(pub u32);

impl LeafId {
    #[inline]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    #[inline]
    pub const fn as_u32(self) -> u32 {
        self.0
    }

    #[inline]
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

    #[inline]
    pub fn from_usize(val: usize) -> Self {
        Self(val as u32)
    }
}

impl fmt::Display for LeafId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Leaf({})", self.0)
    }
}

impl From<u32> for LeafId {
    #[inline]
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<LeafId> for u32 {
    #[inline]
    fn from(id: LeafId) -> Self {
        id.0
    }
}

impl From<i32> for LeafId {
    #[inline]
    fn from(id: i32) -> Self {
        Self(id as u32)
    }
}

impl From<usize> for LeafId {
    #[inline]
    fn from(id: usize) -> Self {
        Self(id as u32)
    }
}

impl From<LeafId> for usize {
    #[inline]
    fn from(id: LeafId) -> Self {
        id.0 as usize
    }
}

impl From<LeafId> for ElementId {
    #[inline]
    fn from(id: LeafId) -> Self {
        id.as_usize().into()
    }
}

/// Strongly-typed identifier representing an internal split divider node.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(transparent)]
pub struct SplitId(pub u32);

impl SplitId {
    #[inline]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    #[inline]
    pub const fn as_u32(self) -> u32 {
        self.0
    }

    #[inline]
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

    #[inline]
    pub fn from_usize(val: usize) -> Self {
        Self(val as u32)
    }
}

impl fmt::Display for SplitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Split({})", self.0)
    }
}

impl From<u32> for SplitId {
    #[inline]
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<SplitId> for u32 {
    #[inline]
    fn from(id: SplitId) -> Self {
        id.0
    }
}

impl From<i32> for SplitId {
    #[inline]
    fn from(id: i32) -> Self {
        Self(id as u32)
    }
}

impl From<usize> for SplitId {
    #[inline]
    fn from(id: usize) -> Self {
        Self(id as u32)
    }
}

impl From<SplitId> for usize {
    #[inline]
    fn from(id: SplitId) -> Self {
        id.0 as usize
    }
}

impl From<SplitId> for ElementId {
    #[inline]
    fn from(id: SplitId) -> Self {
        id.as_usize().into()
    }
}

/// Tagged identifier representing either a leaf panel or an internal split divider.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
pub enum NodeId {
    Leaf(LeafId),
    Split(SplitId),
}

impl NodeId {
    #[inline]
    pub fn is_leaf(&self) -> bool {
        matches!(self, Self::Leaf(_))
    }

    #[inline]
    pub fn is_split(&self) -> bool {
        matches!(self, Self::Split(_))
    }

    #[inline]
    pub fn as_leaf(&self) -> Option<LeafId> {
        match self {
            Self::Leaf(id) => Some(*id),
            Self::Split(_) => None,
        }
    }

    #[inline]
    pub fn as_split(&self) -> Option<SplitId> {
        match self {
            Self::Split(id) => Some(*id),
            Self::Leaf(_) => None,
        }
    }

    #[inline]
    pub fn raw_id(&self) -> u32 {
        match self {
            Self::Leaf(id) => id.0,
            Self::Split(id) => id.0,
        }
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Leaf(id) => write!(f, "{}", id),
            Self::Split(id) => write!(f, "{}", id),
        }
    }
}

impl From<LeafId> for NodeId {
    #[inline]
    fn from(id: LeafId) -> Self {
        Self::Leaf(id)
    }
}

impl From<SplitId> for NodeId {
    #[inline]
    fn from(id: SplitId) -> Self {
        Self::Split(id)
    }
}

impl From<NodeId> for ElementId {
    #[inline]
    fn from(id: NodeId) -> Self {
        (id.raw_id() as usize).into()
    }
}

/// Centralized, safe ID generator ensuring collision-free numbering across
/// leaves and split dividers within a split root.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(transparent)]
pub struct NodeIdAllocator {
    next_id: u32,
}

impl Default for NodeIdAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeIdAllocator {
    /// Creates a new allocator starting at 1.
    #[inline]
    pub const fn new() -> Self {
        Self { next_id: 1 }
    }

    /// Creates an allocator starting from a specific initial counter value.
    #[inline]
    pub const fn with_start(start: u32) -> Self {
        Self { next_id: start }
    }

    /// Allocates the next unique LeafId.
    pub fn next_leaf_id(&mut self) -> LeafId {
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).expect("NodeId overflow");
        LeafId(id)
    }

    /// Allocates the next unique SplitId.
    pub fn next_split_id(&mut self) -> SplitId {
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).expect("NodeId overflow");
        SplitId(id)
    }

    /// Peeks at the next ID without advancing the counter.
    #[inline]
    pub const fn peek_next(&self) -> u32 {
        self.next_id
    }

    /// Updates the allocator counter to be strictly greater than `observed_id`.
    /// Useful when deserializing or restoring existing layouts to prevent ID reuse.
    pub fn observe(&mut self, observed_id: u32) {
        if observed_id >= self.next_id {
            self.next_id = observed_id.saturating_add(1);
        }
    }
}
