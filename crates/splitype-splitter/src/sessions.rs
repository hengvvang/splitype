//! Drag-session records and the corner-drag action vocabulary.
//!
//! These are pure state records; the gesture handling that drives them
//! lives in the hosts' render layers (`src/windows/layout` for the outer
//! layout, `src/editor/windows/layout` for the inner one).

use gpui::{Pixels, Point};

use crate::tree::{AreaRect, Axis, Direction};
use crate::types::AreaSplitMode;

/// Modifier key held during a corner drag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CornerDragModifier {
    None,
    Swap, // Ctrl – swap area contents
    /// Shift – behavior depends on the dragged area's kind: Explorer
    /// behaves like a plain drag, Settings opens the floating settings
    /// window, Editor splits into a fresh blank editor.
    Duplicate,
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
        target_id: usize,
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

impl SplitterDragSession {
    /// The split ratio for the current pointer position: the drag delta
    /// over the split's pixel span, added to the start ratio and clamped.
    /// Pure computation shared by every layout level.
    pub fn ratio_at(&self, current_pointer_pos: f32) -> f32 {
        let delta = current_pointer_pos - self.start_pointer_pos;
        (self.start_ratio + delta / self.total_span).clamp(0.08, 0.92)
    }
}

/// Corner-drag gesture session.
///
/// Analogous to Blender's `sActionzoneData` – tracks which area corner was
/// grabbed, the gesture direction, and the modifier key held.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CornerDragSession {
    /// The area (outer level) or panel (inner level) whose corner was grabbed.
    pub target_id: usize,
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
///
/// `split_id` doubles as the target area id: by the split tree's convention
/// a split node's id equals its second child leaf's id, so split/close (which
/// target that leaf) and swap (which needs the split id) all use one value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderMenuState {
    pub split_id: usize,
    pub direction: Axis,
    pub position: Point<Pixels>,
}

/// The action produced by a corner-drag gesture (generic over the layout
/// level: window areas on the outer tree, editor panels on the inner one).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CornerDragAction {
    /// Split the dragged leaf with a same-kind sibling. The `mode` seeds
    /// the new leaf: [`AreaSplitMode::Copy`] clones the source, `Fresh`
    /// starts blank.
    Split {
        target_id: usize,
        direction: Axis,
        ratio: f32,
        mode: AreaSplitMode,
    },
    /// Join the dragged leaf into a neighbor (removes the dragged leaf).
    Join { from: usize, into: usize },
    /// Swap the kinds (and per-leaf state) of two leaves.
    Swap { from: usize, to: usize },
    /// Shift + drag on a kind whose behavior is `Duplicate`: the host
    /// decides what to do (inner editor panels currently no-op).
    Duplicate { target_id: usize },
    /// Gesture was cancelled. Hosts interpret kind-specific shortcuts:
    /// the outer Settings corner opens the floating settings window.
    Cancel,
}

/// Minimum drag distance before swap / duplicate gesture.
pub const MODIFIER_THRESHOLD_PX: f32 = 4.0;

/// Return the id of the element that contains `pos`, given pixel-space rects.
/// Generic over layout level: the id is an `AreaId` when called with outer
/// rects and a `PanelId` when called with inner rects.
pub fn id_at_point(rects: &[AreaRect], pos: Point<Pixels>) -> Option<usize> {
    let px = f32::from(pos.x);
    let py = f32::from(pos.y);
    for rect in rects {
        if px >= rect.x && px <= rect.x + rect.width && py >= rect.y && py <= rect.y + rect.height {
            return Some(rect.id);
        }
    }
    None
}
