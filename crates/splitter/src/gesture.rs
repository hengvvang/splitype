//! Gesture session records and user interaction facts.
//!
//! Tracks active drag states, pointer deltas, modifier keys, and dock intents.
//! Pure state facts decoupled from layout topology and graphics rendering.

use gpui::{Pixels, Point};

use crate::geometry::{Direction, SplitAxis};
use crate::id::{LeafId, SplitId};

/// Modifier key held during a corner drag gesture.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CornerDragModifier {
    /// Plain drag without modifier keys (split or join).
    #[default]
    None,
    /// Ctrl + drag — swaps area contents.
    Ctrl,
    /// Shift + drag — opens the dragged panel in a new cloned window.
    Shift,
}

/// Target edge or region within a hovered area during a move/dock/join/swap drag.
/// Direct 1:1 match with Blender's `AreaDockTarget`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AreaDockTarget {
    /// No target / dragging within same area.
    #[default]
    None,
    /// Dock to the top edge of target area (horizontal split).
    Top,
    /// Dock to the bottom edge of target area (horizontal split).
    Bottom,
    /// Dock to the left edge of target area (vertical split).
    Left,
    /// Dock to the right edge of target area (vertical split).
    Right,
    /// Hovering the center region of target area (triggers Swap Areas).
    Center,
}

impl AreaDockTarget {
    /// Returns `true` if this target represents docking to an edge (Top, Bottom, Left, or Right).
    #[inline]
    pub const fn is_edge_dock(self) -> bool {
        matches!(self, Self::Top | Self::Bottom | Self::Left | Self::Right)
    }

    /// Returns `true` if this target represents hovering the center for an area swap.
    #[inline]
    pub const fn is_center_swap(self) -> bool {
        matches!(self, Self::Center)
    }
}

/// Active drag session for resizing an internal split divider bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitterDragSession {
    pub split_id: SplitId,
    pub axis: SplitAxis,
    pub start_pointer_pos: f32,
    pub start_ratio: f32,
    pub total_span: f32,
}

impl SplitterDragSession {
    /// The split ratio for the current pointer position: the drag delta
    /// over the split's pixel span, added to the start ratio and clamped.
    #[inline]
    pub fn ratio_at(&self, current_pointer_pos: f32) -> f32 {
        let delta = current_pointer_pos - self.start_pointer_pos;
        (self.start_ratio + delta / self.total_span).clamp(0.08, 0.92)
    }
}

/// Corner-drag gesture session — raw gesture facts.
///
/// Analogous to Blender's `sActionzoneData`: tracks which area corner was
/// grabbed, the gesture direction, modifier keys, pointer position, and computed dock targets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CornerDragSession {
    /// The leaf panel container whose corner was grabbed.
    pub target_id: LeafId,
    /// Where the drag started (in window coordinates).
    pub start_pos: Point<Pixels>,
    /// Cardinal direction deduced from pointer delta so far.
    pub gesture_dir: Option<Direction>,
    /// Modifier key held during the drag.
    pub modifier: CornerDragModifier,
    /// The pointer's latest position.
    pub pointer_pos: Option<Point<Pixels>>,
    /// The leaf the pointer is currently hovering over, if any.
    pub hover_leaf: Option<LeafId>,
    /// The computed dock target when hovering another leaf.
    pub dock_target: AreaDockTarget,
    /// The computed dynamic dock/split ratio.
    pub dock_ratio: f32,
}

/// Context menu state for right-clicking a border divider bar between leaves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderMenuState {
    pub split_id: SplitId,
    pub position: Point<Pixels>,
}

/// Minimum drag distance before a modifier-based shortcut fires.
pub const MODIFIER_THRESHOLD_PX: f32 = 4.0;

/// Whether a corner drag has moved far enough from its start for a modifier-based shortcut to fire.
pub fn past_shortcut_threshold(facts: &CornerDragSession) -> bool {
    let Some(pos) = facts.pointer_pos else {
        return false;
    };
    let dx = f32::from(pos.x - facts.start_pos.x);
    let dy = f32::from(pos.y - facts.start_pos.y);
    (dx * dx + dy * dy).sqrt() >= MODIFIER_THRESHOLD_PX
}
