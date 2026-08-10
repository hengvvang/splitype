//! The panel container — one leaf of the split tree.
//!
//! `SplitterContainer<T>` is a single panel: `T` is the panel type, so
//! `SplitterContainer<Editor>` reads as "an editor panel" — the type is
//! the identity. Every leaf of a [`SplitTree`] is one of these
//! containers; splitting a leaf creates a second container, and both
//! hang on the same tree. Each container records its own interaction
//! state (corner-drag session, dropdown, maximized), so containers stay
//! fully self-contained — never shared across containers.

use gpui::{Pixels, Point};

use crate::sessions::{CornerDragModifier, CornerDragSession};
use crate::types::NodeId;

/// A panel container: one leaf of the split tree, holding the panel type
/// `T` and its own interaction state.
#[derive(Clone, Debug, PartialEq)]
pub struct SplitterContainer<T> {
    /// This panel's node id (unique within its root's id space).
    pub id: NodeId,
    /// The panel type — the identity of the panel.
    pub kind: T,
    /// This panel's own corner-drag session, while one of its four
    /// corners is being dragged.
    pub active_corner_drag: Option<CornerDragSession>,
    /// Whether this panel's dropdown menu is open.
    pub open_dropdown: bool,
    /// Whether this panel is maximized (fills the whole root area).
    pub maximized: bool,
}

impl<T: Copy + PartialEq> SplitterContainer<T> {
    pub fn new(id: NodeId, kind: T) -> Self {
        Self {
            id,
            kind,
            active_corner_drag: None,
            open_dropdown: false,
            maximized: false,
        }
    }

    /// Begin a corner-drag gesture from this panel's corner.
    pub fn start_corner_drag(&mut self, pos: Point<Pixels>, modifier: CornerDragModifier) {
        self.active_corner_drag = Some(CornerDragSession {
            target_id: self.id,
            start_pos: pos,
            gesture_dir: None,
            modifier,
            pointer_pos: Some(pos),
            hover_leaf: None,
        });
    }

    /// End this panel's corner-drag session, returning the raw facts.
    pub fn finish_corner_drag(&mut self) -> Option<CornerDragSession> {
        let session = self.active_corner_drag?;
        self.active_corner_drag = None;
        Some(session)
    }

    /// End this panel's corner-drag session without returning facts.
    pub fn end_corner_drag(&mut self) {
        self.active_corner_drag = None;
    }
}
