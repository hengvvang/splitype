//! Window layout tree and panel identifiers.

use crate::panel::PanelKind;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use splitter::root::SplitterRoot;
use splitter::tree::NodeId;

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
pub struct PanelId(pub NodeId);

impl From<NodeId> for PanelId {
    #[inline]
    fn from(id: NodeId) -> Self {
        Self(id)
    }
}

impl From<PanelId> for NodeId {
    #[inline]
    fn from(id: PanelId) -> Self {
        id.0
    }
}

impl From<PanelId> for gpui::ElementId {
    #[inline]
    fn from(id: PanelId) -> Self {
        id.0.into()
    }
}

impl std::fmt::Display for PanelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The window-level split layout root containing PanelKind tiles.
pub type WindowLayout = SplitterRoot<PanelKind>;
