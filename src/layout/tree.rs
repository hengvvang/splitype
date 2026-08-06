//! The recursive binary split tree and its operations.
//!
//! Generic over the area type `T` so that the outer layout uses `WindowAreaKind`
//! while inner (Edit sub-panel) layouts use `EditorInnerPanelKind`.

/// Split orientation between adjacent areas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Horizontal, // Splits left and right
    Vertical,   // Splits top and bottom
}

/// Cardinal direction used for corner-drag gesture routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Right,
    Left,
}

impl Direction {
    pub fn is_vertical(self) -> bool {
        matches!(self, Self::Up | Self::Down)
    }
}

/// A leaf's rectangle in layout space, normalized to 0..1 (or scaled to
/// pixels by the host when collected from `WindowLayout`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AreaRect {
    pub id: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Recursive binary layout tree representing tiled areas and splitters.
///
/// Design is inspired by Blender's screen area action-zone system: each
/// area exposes four corner hot-zones that, when dragged, produce either a
/// split (same area), a join (neighbour area), a swap (Ctrl), or a duplicate
/// (Shift) – with differentiated gesture thresholds and directional cursors.
#[derive(Clone, Debug)]
pub enum SplitTree<T: Copy> {
    Leaf {
        id: usize,
        kind: T,
    },
    Split {
        id: usize,
        direction: Axis,
        ratio: f32,
        first: Box<SplitTree<T>>,
        second: Box<SplitTree<T>>,
    },
}

impl<T: Copy + PartialEq> PartialEq for SplitTree<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Leaf { id: id1, kind: t1 }, Self::Leaf { id: id2, kind: t2 }) => {
                id1 == id2 && t1 == t2
            }
            (
                Self::Split {
                    id: id1,
                    direction: d1,
                    ratio: r1,
                    first: f1,
                    second: s1,
                },
                Self::Split {
                    id: id2,
                    direction: d2,
                    ratio: r2,
                    first: f2,
                    second: s2,
                },
            ) => id1 == id2 && d1 == d2 && (r1 - r2).abs() < 1e-4 && f1 == f2 && s1 == s2,
            _ => false,
        }
    }
}

impl<T: Copy + PartialEq> SplitTree<T> {
    pub fn count_leaves(&self) -> usize {
        match self {
            Self::Leaf { .. } => 1,
            Self::Split { first, second, .. } => first.count_leaves() + second.count_leaves(),
        }
    }

    pub fn find_leaf_kind(&self, leaf_id: usize) -> Option<T> {
        match self {
            Self::Leaf { id, kind } => {
                if *id == leaf_id {
                    Some(*kind)
                } else {
                    None
                }
            }
            Self::Split { first, second, .. } => first
                .find_leaf_kind(leaf_id)
                .or_else(|| second.find_leaf_kind(leaf_id)),
        }
    }

    pub fn set_leaf_kind(&mut self, leaf_id: usize, new_kind: T) -> bool {
        match self {
            Self::Leaf { id, kind } => {
                if *id == leaf_id {
                    *kind = new_kind;
                    true
                } else {
                    false
                }
            }
            Self::Split { first, second, .. } => {
                first.set_leaf_kind(leaf_id, new_kind) || second.set_leaf_kind(leaf_id, new_kind)
            }
        }
    }

    /// Does the subtree contain the leaf with `leaf_id`?
    pub fn contains_leaf(&self, leaf_id: usize) -> bool {
        match self {
            Self::Leaf { id, .. } => *id == leaf_id,
            Self::Split { first, second, .. } => {
                first.contains_leaf(leaf_id) || second.contains_leaf(leaf_id)
            }
        }
    }

    /// Collect all leaf rectangles as [`AreaRect`]s. Coordinates are in
    /// layout-space (normalized 0..1).
    pub fn collect_leaf_rects(&self, x: f32, y: f32, w: f32, h: f32, out: &mut Vec<AreaRect>) {
        match self {
            Self::Leaf { id, .. } => {
                out.push(AreaRect {
                    id: *id,
                    x,
                    y,
                    width: w,
                    height: h,
                });
            }
            Self::Split {
                direction,
                ratio,
                first,
                second,
                ..
            } => {
                let r = ratio.clamp(0.0, 1.0);
                match direction {
                    Axis::Horizontal => {
                        first.collect_leaf_rects(x, y, w * r, h, out);
                        second.collect_leaf_rects(x + w * r, y, w * (1.0 - r), h, out);
                    }
                    Axis::Vertical => {
                        first.collect_leaf_rects(x, y, w, h * r, out);
                        second.collect_leaf_rects(x, y + h * r, w, h * (1.0 - r), out);
                    }
                }
            }
        }
    }

    /// Find layout-space span (0..1 width or height) for a target split node.
    pub fn find_split_span(
        &self,
        target_split_id: usize,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) -> Option<(Axis, f32)> {
        match self {
            Self::Leaf { .. } => None,
            Self::Split {
                id,
                direction,
                ratio,
                first,
                second,
            } => {
                if *id == target_split_id {
                    let span = match direction {
                        Axis::Horizontal => w,
                        Axis::Vertical => h,
                    };
                    return Some((*direction, span));
                }
                let r = ratio.clamp(0.0, 1.0);
                match direction {
                    Axis::Horizontal => first
                        .find_split_span(target_split_id, x, y, w * r, h)
                        .or_else(|| {
                            second.find_split_span(target_split_id, x + w * r, y, w * (1.0 - r), h)
                        }),
                    Axis::Vertical => first
                        .find_split_span(target_split_id, x, y, w, h * r)
                        .or_else(|| {
                            second.find_split_span(target_split_id, x, y + h * r, w, h * (1.0 - r))
                        }),
                }
            }
        }
    }

    /// Split a leaf at 50% ratio with the given `next_type` for the new side.
    #[allow(dead_code)] // used by tests
    pub fn split_leaf(
        &mut self,
        target_id: usize,
        new_id: usize,
        direction: Axis,
        next_type: T,
    ) -> bool {
        self.split_leaf_with_ratio(target_id, new_id, direction, 0.5, next_type)
    }

    /// Split a leaf at a specific ratio (clamped to [0.15, 0.85]).
    ///
    /// `next_type` is the area type assigned to the newly created sibling leaf.
    pub fn split_leaf_with_ratio(
        &mut self,
        target_id: usize,
        new_id: usize,
        direction: Axis,
        ratio: f32,
        next_type: T,
    ) -> bool {
        let ratio = ratio.clamp(0.15, 0.85);
        match self {
            Self::Leaf { id, kind } => {
                if *id == target_id {
                    let old_type = *kind;
                    *self = Self::Split {
                        id: new_id,
                        direction,
                        ratio,
                        first: Box::new(Self::Leaf {
                            id: *id,
                            kind: old_type,
                        }),
                        second: Box::new(Self::Leaf {
                            id: new_id,
                            kind: next_type,
                        }),
                    };
                    true
                } else {
                    false
                }
            }
            Self::Split { first, second, .. } => {
                first.split_leaf_with_ratio(target_id, new_id, direction, ratio, next_type)
                    || second.split_leaf_with_ratio(target_id, new_id, direction, ratio, next_type)
            }
        }
    }

    pub fn remove_leaf(&mut self, target_id: usize) -> bool {
        match self {
            Self::Leaf { .. } => false,
            Self::Split { first, second, .. } => {
                if let Self::Leaf { id, .. } = **first {
                    if id == target_id {
                        *self = (**second).clone();
                        return true;
                    }
                }
                if let Self::Leaf { id, .. } = **second {
                    if id == target_id {
                        *self = (**first).clone();
                        return true;
                    }
                }
                first.remove_leaf(target_id) || second.remove_leaf(target_id)
            }
        }
    }

    /// Join `target_id` into `into_id`. The `target_id` leaf is removed and
    /// `into_id` expands to fill the space. Both leaves must share an immediate
    /// split parent (be adjacent siblings).  Returns true on success.
    pub fn join_leaf(&mut self, into_id: usize, target_id: usize) -> bool {
        if into_id == target_id {
            return false;
        }
        match self {
            Self::Leaf { .. } => false,
            Self::Split { first, second, .. } => {
                let into_in_first = first.contains_leaf(into_id);
                let target_in_first = first.contains_leaf(target_id);

                if into_in_first && target_in_first {
                    first.join_leaf(into_id, target_id)
                } else if !into_in_first && !target_in_first {
                    second.join_leaf(into_id, target_id)
                } else {
                    // The two leaves are in different children → this split is their
                    // lowest common ancestor.  Remove the target leaf from its side;
                    // the remaining child (with target removed) stays in place so the
                    // split direction and ratio are preserved.
                    if target_in_first {
                        if !first.remove_leaf(target_id) {
                            return false;
                        }
                    } else {
                        if !second.remove_leaf(target_id) {
                            return false;
                        }
                    }
                    true
                }
            }
        }
    }

    pub fn set_split_ratio(&mut self, split_id: usize, new_ratio: f32) -> bool {
        match self {
            Self::Leaf { .. } => false,
            Self::Split {
                id,
                ratio,
                first,
                second,
                ..
            } => {
                if *id == split_id {
                    *ratio = new_ratio.clamp(0.08, 0.92);
                    true
                } else {
                    first.set_split_ratio(split_id, new_ratio)
                        || second.set_split_ratio(split_id, new_ratio)
                }
            }
        }
    }

    /// Deep-clone this subtree, assigning fresh node ids from a shared id
    /// pool (`next_node_id`). Used when an Editor area is split: the new
    /// area's inner panel tree is an independent copy of the source area's.
    pub fn clone_with_new_ids(&self, next_id: &mut usize) -> SplitTree<T> {
        match self {
            Self::Leaf { kind, .. } => {
                let id = *next_id;
                *next_id += 1;
                Self::Leaf { id, kind: *kind }
            }
            Self::Split {
                direction,
                ratio,
                first,
                second,
                ..
            } => {
                let id = *next_id;
                *next_id += 1;
                Self::Split {
                    id,
                    direction: *direction,
                    ratio: *ratio,
                    first: Box::new(first.clone_with_new_ids(next_id)),
                    second: Box::new(second.clone_with_new_ids(next_id)),
                }
            }
        }
    }

    pub fn swap_sibling_leaves(&mut self, split_id: usize) -> bool {
        match self {
            Self::Leaf { .. } => false,
            Self::Split {
                id, first, second, ..
            } => {
                if *id == split_id {
                    std::mem::swap(first, second);
                    true
                } else {
                    first.swap_sibling_leaves(split_id) || second.swap_sibling_leaves(split_id)
                }
            }
        }
    }
}
