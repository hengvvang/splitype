//! The panel container — one leaf of the split tree.
//!
//! `SplitterContainer<T>` represents a single panel leaf in the split tree.
//! `T` is the panel payload type (such as `PanelKind` or `PaneKind`).
//! All transient interaction state (drag sessions, dropdowns, maximize states)
//! is managed independently by [`crate::interaction::LayoutInteraction`].

use crate::id::LeafId;

/// A panel container: one leaf of the split tree, holding the panel type
/// `T` as its payload and a strongly-typed [`LeafId`].
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(bound(
    serialize = "T: serde::Serialize",
    deserialize = "T: serde::Deserialize<'de>"
))]
pub struct SplitterContainer<T> {
    /// This panel's strongly-typed leaf identifier.
    pub id: LeafId,
    /// The panel kind — the identity/payload of the panel.
    pub kind: T,
}

impl<T: Clone + PartialEq> SplitterContainer<T> {
    #[inline]
    pub fn new<I: Into<LeafId>>(id: I, kind: T) -> Self {
        Self {
            id: id.into(),
            kind,
        }
    }
}
