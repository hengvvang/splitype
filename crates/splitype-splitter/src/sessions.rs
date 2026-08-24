//! Drag-session records and the corner-drag fact vocabulary.
//!
//! These are pure state records; the gesture handling that drives them
//! and every policy decision (what a drag means, whether to render an
//! indicator) lives in the hosts' render layer (`src/app/window_layout`
//! drives the mouse gestures for both the outer panels and the inner
//! editor panes; `src/editor/view` renders the panes).

use gpui::{Pixels, Point};

use crate::tree::{Direction, LeafRect, SplitAxis};

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

/// Target edge or region within a hovered area during a move/dock/join/swap drag.
/// Direct 1:1 match with Blender's `AreaDockTarget`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaDockTarget {
    /// No target / dragging within same area.
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

/// Active drag session for resizing a split bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitterDragSession {
    pub split_id: usize,
    pub axis: SplitAxis,
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
    /// The computed dock target when hovering another leaf.
    pub dock_target: AreaDockTarget,
    /// The computed dynamic dock/split ratio.
    pub dock_ratio: f32,
}

/// Context menu state for right-clicking a border divider bar between leaves.
///
/// `split_id` doubles as the target area id: by the split tree's convention
/// a split node's id equals its second child leaf's id, so split/close (which
/// target that leaf) and swap (which needs the split id) all use one value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderMenuState {
    pub split_id: usize,
    pub axis: SplitAxis,
    pub position: Point<Pixels>,
}

/// Minimum drag distance before a host's modifier-based shortcut fires.
pub const MODIFIER_THRESHOLD_PX: f32 = 4.0;

/// Snaps a split ratio with 0.5 center magnetic snapping and optional 1/12 grid snapping.
/// Smoothly ranges from 0.0 (0% at outer edge) to 1.0 (100% at center).
pub fn calc_snapped_ratio(raw_ratio: f32, ctrl_held: bool) -> f32 {
    let clamped = raw_ratio.clamp(0.0, 1.0);
    // 0.5 center magnetic snapping (0.46 ~ 0.54 -> 0.50)
    if (0.46..=0.54).contains(&clamped) {
        return 0.5;
    }
    if ctrl_held {
        let frac = (clamped * 12.0).round() / 12.0;
        return frac.clamp(0.0, 1.0);
    }
    clamped
}

/// Check if two leaf rects share an edge (are direct adjacent neighbors).
pub fn is_adjacent_neighbor(a: &LeafRect, b: &LeafRect) -> bool {
    const EPS: f32 = 6.0;
    let overlap_x = (a.x + a.width).min(b.x + b.width) - a.x.max(b.x);
    let overlap_y = (a.y + a.height).min(b.y + b.height) - a.y.max(b.y);

    let shares_vertical_border =
        (a.x + a.width - b.x).abs() <= EPS || (b.x + b.width - a.x).abs() <= EPS;
    let shares_horizontal_border =
        (a.y + a.height - b.y).abs() <= EPS || (b.y + b.height - a.y).abs() <= EPS;

    (shares_vertical_border && overlap_y > EPS) || (shares_horizontal_border && overlap_x > EPS)
}

/// Calculates the dock target (None = Join, Center = Swap, Top/Bottom/Left/Right = Dock) and ratio.
/// Perfectly mirrors Blender's `area_docking_target` (source/blender/editors/screen/screen_ops.cc).
pub fn calculate_dock_target(
    source_rect: &LeafRect,
    target_rect: &LeafRect,
    _dir: Direction,
    pointer_pos: Point<Pixels>,
    ctrl_held: bool,
) -> (AreaDockTarget, f32) {
    if target_rect.width <= 0.0 || target_rect.height <= 0.0 {
        return (AreaDockTarget::None, 0.5);
    }
    if ctrl_held {
        return (AreaDockTarget::Center, 0.5);
    }

    let px = f32::from(pointer_pos.x);
    let py = f32::from(pointer_pos.y);
    let fac_x = ((px - target_rect.x) / target_rect.width).clamp(0.0, 1.0);
    let fac_y = ((py - target_rect.y) / target_rect.height).clamp(0.0, 1.0);

    const EPS: f32 = 6.0;

    // 1. Direct Neighbor Join Zone check (0.15 join zone)
    // Up or Down immediate neighbor:
    let is_source_above = (source_rect.y + source_rect.height - target_rect.y).abs() <= EPS;
    let is_source_below = (target_rect.y + target_rect.height - source_rect.y).abs() <= EPS;
    if is_source_above || is_source_below {
        let overlap_x = (source_rect.x + source_rect.width).min(target_rect.x + target_rect.width)
            - source_rect.x.max(target_rect.x);
        if overlap_x > EPS && px >= source_rect.x && px <= source_rect.x + source_rect.width {
            let in_join_y = if is_source_above {
                fac_y <= 0.15
            } else {
                fac_y >= 0.85
            };
            if in_join_y {
                return (AreaDockTarget::None, 0.5);
            }
        }
    }

    // Left or Right immediate neighbor:
    let is_source_left = (source_rect.x + source_rect.width - target_rect.x).abs() <= EPS;
    let is_source_right = (target_rect.x + target_rect.width - source_rect.x).abs() <= EPS;
    if is_source_left || is_source_right {
        let overlap_y = (source_rect.y + source_rect.height).min(target_rect.y + target_rect.height)
            - source_rect.y.max(target_rect.y);
        if overlap_y > EPS && py >= source_rect.y && py <= source_rect.y + source_rect.height {
            let in_join_x = if is_source_left {
                fac_x <= 0.15
            } else {
                fac_x >= 0.85
            };
            if in_join_x {
                return (AreaDockTarget::None, 0.5);
            }
        }
    }

    // 2. Center zone (0.40..=0.60 in both dimensions) -> triggers Swap Areas (Blender lines 4978-4980)
    if (0.40..=0.60).contains(&fac_x) && (0.40..=0.60).contains(&fac_y) {
        return (AreaDockTarget::Center, 0.5);
    }

    // 3. 4-quadrant trapezoid Move & Dock (Full 0.0 ~ 1.0 scaling)
    let is_top = fac_y <= fac_x && fac_y <= (1.0 - fac_x);
    let is_bottom = fac_y >= fac_x && fac_y >= (1.0 - fac_x);
    let is_left = fac_x <= fac_y && fac_x <= (1.0 - fac_y);

    if is_top {
        let raw_pos = if is_source_above {
            ((fac_y - 0.15) / 0.35).clamp(0.0, 1.0)
        } else {
            fac_y * 2.0
        };
        let ratio = calc_snapped_ratio(raw_pos, ctrl_held);
        (AreaDockTarget::Top, ratio)
    } else if is_bottom {
        let raw_pos = if is_source_below {
            ((0.85 - fac_y) / 0.35).clamp(0.0, 1.0)
        } else {
            (1.0 - fac_y) * 2.0
        };
        let ratio = calc_snapped_ratio(raw_pos, ctrl_held);
        (AreaDockTarget::Bottom, ratio)
    } else if is_left {
        let raw_pos = if is_source_left {
            ((fac_x - 0.15) / 0.35).clamp(0.0, 1.0)
        } else {
            fac_x * 2.0
        };
        let ratio = calc_snapped_ratio(raw_pos, ctrl_held);
        (AreaDockTarget::Left, ratio)
    } else {
        let raw_pos = if is_source_right {
            ((0.85 - fac_x) / 0.35).clamp(0.0, 1.0)
        } else {
            (1.0 - fac_x) * 2.0
        };
        let ratio = calc_snapped_ratio(raw_pos, ctrl_held);
        (AreaDockTarget::Right, ratio)
    }
}

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
