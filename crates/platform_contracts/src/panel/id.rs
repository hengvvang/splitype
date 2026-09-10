//! Panel identifier.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use splitter::LeafId;

/// Strongly-typed identifier representing a top-level window panel tile.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(transparent)]
pub struct PanelId(pub LeafId);

impl PanelId {
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

impl From<LeafId> for PanelId {
    #[inline]
    fn from(id: LeafId) -> Self {
        Self(id)
    }
}

impl From<PanelId> for LeafId {
    #[inline]
    fn from(id: PanelId) -> Self {
        id.0
    }
}

impl From<u32> for PanelId {
    #[inline]
    fn from(id: u32) -> Self {
        Self(LeafId::new(id))
    }
}

impl From<i32> for PanelId {
    #[inline]
    fn from(id: i32) -> Self {
        Self(LeafId::new(id as u32))
    }
}

impl From<usize> for PanelId {
    #[inline]
    fn from(id: usize) -> Self {
        Self(LeafId::from_usize(id))
    }
}

impl From<PanelId> for usize {
    #[inline]
    fn from(id: PanelId) -> Self {
        id.0.as_usize()
    }
}

impl From<PanelId> for gpui::ElementId {
    #[inline]
    fn from(id: PanelId) -> Self {
        id.0.as_usize().into()
    }
}

impl std::fmt::Display for PanelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
