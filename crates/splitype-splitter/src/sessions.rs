//! Drag-session records and the corner-drag fact vocabulary.
//!
//! These are pure state records; the gesture handling that drives them
//! and every policy decision (what a drag means, whether to render an
//! indicator) lives in the hosts' render layer (`src/editor/window_layout`
//! drives the mouse gestures for both the outer panels and the inner
//! editor panes; `src/editor/pane_layout` renders the panes).

use gpui::{Pixels, Point};

use crate::tree::{LeafRect, Axis, Direction};

/// Modifier key held during a corner drag — a raw gesture fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CornerDragModifier {
    /// Plain drag.
    None,
    /// Ctrl + drag — the host decides (default: swap area contents).
    Ctrl,
    /// Shift + drag — the host decides (default: open the dragged panel
    /// in a new window).
    Shift,
    /// Alt + drag — the host decides (default: no-op).
    Alt,
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

/// Corner-drag gesture session — raw facts only.
///
/// Analogous to Blender's `sActionzoneData`: tracks which area corner was
/// grabbed, the gesture direction, the modifier key, and the pointer
/// facts. The engine never interprets them; hosts decide what the gesture
/// means and whether / how to render an indicator.
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
    /// The pointer's latest position (same coordinate space as the drag).
    pub pointer_pos: Option<Point<Pixels>>,
    /// The leaf the pointer is currently over, if any.
    pub hover_leaf: Option<usize>,
}

/// Context menu state for right-clicking a border divider bar between leaves.
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

/// Minimum drag distance before a host's modifier-based shortcut fires.
pub const MODIFIER_THRESHOLD_PX: f32 = 4.0;

/// Whether a corner drag has moved far enough from its start for a
/// modifier-based shortcut to fire (see [`MODIFIER_THRESHOLD_PX`]).
/// Pure over the session facts; hosts and the drag policy share it so
/// the threshold is checked in exactly one place.
pub fn past_shortcut_threshold(facts: &CornerDragSession) -> bool {
    let Some(pos) = facts.pointer_pos else {
        return false;
    };
    let dx = f32::from(pos.x - facts.start_pos.x);
    let dy = f32::from(pos.y - facts.start_pos.y);
    (dx * dx + dy * dy).sqrt() >= MODIFIER_THRESHOLD_PX
}

/// Return the id of the element that contains `pos`, given pixel-space rects.
/// Generic over layout level: the id is an `NodeId` when called with outer
/// rects and a `NodeId` when called with inner rects.
pub fn id_at_point(rects: &[LeafRect], pos: Point<Pixels>) -> Option<usize> {
    let px = f32::from(pos.x);
    let py = f32::from(pos.y);
    for rect in rects {
        if px >= rect.x && px <= rect.x + rect.width && py >= rect.y && py <= rect.y + rect.height {
            return Some(rect.id);
        }
    }
    None
}
