//! Drag-session records and the corner-drag action vocabulary.
//!
//! These are pure state records; the gesture handling that drives them
//! lives in the hosts' render layers (`src/windows/layout` for the outer
//! layout, `src/editor/windows/layout` for the inner one).

use gpui::{Pixels, Point};

use crate::layout::tree::{AreaRect, Axis, Direction};

/// Modifier key held during a corner drag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CornerDragModifier {
    None,
    Swap,      // Ctrl  – swap area contents
    Duplicate, // Shift – duplicate area into a new window
}

/// Live preview state during a corner drag gesture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CornerDragPreview {
    /// Still near the corner, not enough movement yet.
    Dragging,
    /// Showing a split preview line at the given ratio.
    SplitPreview { direction: Axis, ratio: f32 },
    /// Showing a join target highlight.
    JoinPreview {
        target_leaf_id: usize,
        direction: Direction,
    },
}

/// Active drag session for resizing a split bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitterDragSession {
    pub split_id: usize,
    pub direction: Axis,
    pub start_pointer_pos: f32,
    pub start_ratio: f32,
    pub total_span: f32,
}

/// Corner-drag gesture session.
///
/// Analogous to Blender's `sActionzoneData` – tracks which area corner was
/// grabbed, the gesture direction, and the modifier key held.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CornerDragSession {
    /// The leaf whose corner was grabbed.
    pub leaf_id: usize,
    /// Where the drag started (in window coords).
    pub start_pos: Point<Pixels>,
    /// Cardinal direction deduced from the mouse delta so far.
    pub gesture_dir: Option<Direction>,
    /// Modifier key held during the drag.
    pub modifier: CornerDragModifier,
    /// Live preview state for the corner drag overlay.
    pub preview: CornerDragPreview,
}

/// Context menu state for right-clicking a border divider bar between areas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderMenuState {
    pub split_id: usize,
    pub direction: Axis,
    pub position: Point<Pixels>,
}

/// The action that should be performed once a corner-drag gesture crosses its
/// threshold.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CornerDragAction {
    /// Split the dragged leaf into two.
    Split {
        leaf_id: usize,
        direction: Axis,
        ratio: f32,
    },
    /// Join the dragged leaf into a neighbor (remove the dragged leaf).
    Join { from: usize, into: usize },
    /// Swap area types between two leaves.
    Swap { from: usize, to: usize },
    /// Duplicate the dragged leaf (open in a new window).
    Duplicate { leaf_id: usize },
    /// Gesture was cancelled (e.g. dragged to invalid target).
    Cancel,
}

/// Minimum drag distance before swap / duplicate gesture.
pub const MODIFIER_THRESHOLD_PX: f32 = 4.0;

/// Return the leaf id that contains `pos`, given pixel-space rects.
pub fn area_id_at_point(rects: &[AreaRect], pos: Point<Pixels>) -> Option<usize> {
    let px = f32::from(pos.x);
    let py = f32::from(pos.y);
    for rect in rects {
        if px >= rect.x && px <= rect.x + rect.width && py >= rect.y && py <= rect.y + rect.height {
            return Some(rect.id);
        }
    }
    None
}
