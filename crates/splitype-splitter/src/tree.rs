//! The recursive binary split tree and its operations.
//!
//! Generic over the kind type `T` so that the outer layout uses `WindowPanelKind`
//! while inner (Edit pane) layouts use `EditorPaneKind`.

use crate::container::SplitterContainer;

/// The one id concept of the engine: every node of every container
/// (leaves and split nodes alike) is numbered from this single space.
pub type NodeId = usize;

/// Split orientation between adjacent leaves in the layout tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SplitAxis {
    Horizontal, // Splits left and right (vertical divider)
    Vertical,   // Splits top and bottom (horizontal divider)
}

/// Cardinal direction used for corner-drag gesture routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LeafRect {
    pub id: NodeId,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Recursive binary layout tree representing tiled leaves and splitters.
///
/// Design is inspired by Blender's screen area action-zone system: each
/// area exposes four corner hot-zones that, when dragged, produce either a
/// split (same area), a join (neighbour area), a swap (Ctrl), or a
/// window-clone (Shift) – with differentiated gesture thresholds and
/// directional cursors.
///
/// Every leaf is a [`SplitterContainer`] (a panel). Splitting a leaf
/// replaces it with a `Split` node holding two containers — the original
/// and the freshly created one — both hanging on this tree.
#[derive(Clone, Debug)]
pub enum SplitTree<T: Copy> {
    Leaf(SplitterContainer<T>),
    Split {
        id: NodeId,
        axis: SplitAxis,
        ratio: f32,
        first: Box<SplitTree<T>>,
        second: Box<SplitTree<T>>,
    },
}

impl<T: Copy + PartialEq> PartialEq for SplitTree<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Leaf(c1), Self::Leaf(c2)) => c1 == c2,
            (
                Self::Split {
                    id: id1,
                    axis: d1,
                    ratio: r1,
                    first: f1,
                    second: s1,
                },
                Self::Split {
                    id: id2,
                    axis: d2,
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
            Self::Leaf(_) => 1,
            Self::Split { first, second, .. } => first.count_leaves() + second.count_leaves(),
        }
    }

    pub fn find_leaf_kind(&self, leaf_id: NodeId) -> Option<T> {
        match self {
            Self::Leaf(container) => (container.id == leaf_id).then_some(container.kind),
            Self::Split { first, second, .. } => first
                .find_leaf_kind(leaf_id)
                .or_else(|| second.find_leaf_kind(leaf_id)),
        }
    }

    /// The container (panel) of the leaf with `leaf_id`, if any.
    pub fn find_leaf(&self, leaf_id: NodeId) -> Option<&SplitterContainer<T>> {
        match self {
            Self::Leaf(container) => (container.id == leaf_id).then_some(container),
            Self::Split { first, second, .. } => first
                .find_leaf(leaf_id)
                .or_else(|| second.find_leaf(leaf_id)),
        }
    }

    /// The container (panel) of the leaf with `leaf_id`, mutably.
    pub fn find_leaf_mut(&mut self, leaf_id: NodeId) -> Option<&mut SplitterContainer<T>> {
        match self {
            Self::Leaf(container) => (container.id == leaf_id).then_some(container),
            Self::Split { first, second, .. } => first
                .find_leaf_mut(leaf_id)
                .or_else(|| second.find_leaf_mut(leaf_id)),
        }
    }

    /// Finds the first leaf container that is currently maximized, if any.
    pub fn find_maximized_leaf(&self) -> Option<&SplitterContainer<T>> {
        match self {
            Self::Leaf(container) => container.maximized.then_some(container),
            Self::Split { first, second, .. } => first
                .find_maximized_leaf()
                .or_else(|| second.find_maximized_leaf()),
        }
    }

    /// Finds the first leaf container matching the given `kind`, if any.
    pub fn find_first_leaf_by_kind(&self, kind: T) -> Option<&SplitterContainer<T>> {
        match self {
            Self::Leaf(container) => (container.kind == kind).then_some(container),
            Self::Split { first, second, .. } => first
                .find_first_leaf_by_kind(kind)
                .or_else(|| second.find_first_leaf_by_kind(kind)),
        }
    }

    pub fn set_leaf_kind(&mut self, leaf_id: NodeId, new_kind: T) -> bool {
        match self {
            Self::Leaf(container) => {
                if container.id == leaf_id {
                    container.kind = new_kind;
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
    pub fn contains_leaf(&self, leaf_id: NodeId) -> bool {
        match self {
            Self::Leaf(container) => container.id == leaf_id,
            Self::Split { first, second, .. } => {
                first.contains_leaf(leaf_id) || second.contains_leaf(leaf_id)
            }
        }
    }

    /// Collect all leaf ids in tree order.
    pub fn leaf_ids(&self, out: &mut Vec<NodeId>) {
        match self {
            Self::Leaf(container) => out.push(container.id),
            Self::Split { first, second, .. } => {
                first.leaf_ids(out);
                second.leaf_ids(out);
            }
        }
    }

    /// Collect all leaf rectangles as [`LeafRect`]s. Coordinates are in
    /// layout-space (normalized 0..1).
    pub fn collect_leaf_rects(&self, x: f32, y: f32, w: f32, h: f32, out: &mut Vec<LeafRect>) {
        match self {
            Self::Leaf(container) => {
                out.push(LeafRect {
                    id: container.id,
                    x,
                    y,
                    width: w,
                    height: h,
                });
            }
            Self::Split {
                axis,
                ratio,
                first,
                second,
                ..
            } => {
                let r = ratio.clamp(0.0, 1.0);
                match axis {
                    SplitAxis::Horizontal => {
                        first.collect_leaf_rects(x, y, w * r, h, out);
                        second.collect_leaf_rects(x + w * r, y, w * (1.0 - r), h, out);
                    }
                    SplitAxis::Vertical => {
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
        target_split_id: NodeId,
        w: f32,
        h: f32,
    ) -> Option<(SplitAxis, f32)> {
        match self {
            Self::Leaf { .. } => None,
            Self::Split {
                id,
                axis,
                ratio,
                first,
                second,
            } => {
                if *id == target_split_id {
                    let span = match axis {
                        SplitAxis::Horizontal => w,
                        SplitAxis::Vertical => h,
                    };
                    return Some((*axis, span));
                }
                let r = ratio.clamp(0.0, 1.0);
                match axis {
                    SplitAxis::Horizontal => first
                        .find_split_span(target_split_id, w * r, h)
                        .or_else(|| second.find_split_span(target_split_id, w * (1.0 - r), h)),
                    SplitAxis::Vertical => first
                        .find_split_span(target_split_id, w, h * r)
                        .or_else(|| second.find_split_span(target_split_id, w, h * (1.0 - r))),
                }
            }
        }
    }

    /// Finds the first (primary) leaf ID in the subtree rooted at `split_id`.
    pub fn find_split_first_leaf_id(&self, target_split_id: NodeId) -> Option<NodeId> {
        match self {
            Self::Leaf(_) => None,
            Self::Split {
                id, first, second, ..
            } => {
                if *id == target_split_id {
                    let mut leaves = Vec::new();
                    first.leaf_ids(&mut leaves);
                    leaves.first().copied()
                } else {
                    first
                        .find_split_first_leaf_id(target_split_id)
                        .or_else(|| second.find_split_first_leaf_id(target_split_id))
                }
            }
        }
    }

    /// Finds the second (secondary) leaf ID in the subtree rooted at `split_id`.
    pub fn find_split_second_leaf_id(&self, target_split_id: NodeId) -> Option<NodeId> {
        match self {
            Self::Leaf(_) => None,
            Self::Split {
                id, first, second, ..
            } => {
                if *id == target_split_id {
                    let mut leaves = Vec::new();
                    second.leaf_ids(&mut leaves);
                    leaves.first().copied()
                } else {
                    first
                        .find_split_second_leaf_id(target_split_id)
                        .or_else(|| second.find_split_second_leaf_id(target_split_id))
                }
            }
        }
    }

    /// Split a leaf at a specific ratio (clamped to [0.15, 0.85]).
    ///
    /// `split_id` is the ID assigned to the new `Split` parent node, and
    /// `new_leaf_id` is the ID assigned to the newly created sibling leaf.
    /// `next_kind` is the area kind assigned to the newly created sibling leaf.
    pub fn split_leaf_with_ratio(
        &mut self,
        target_id: NodeId,
        split_id: NodeId,
        new_leaf_id: NodeId,
        axis: SplitAxis,
        ratio: f32,
        next_kind: T,
    ) -> bool {
        let ratio = ratio.clamp(0.01, 0.99);
        match self {
            Self::Leaf(container) => {
                if container.id == target_id {
                    let original = container.clone();
                    *self = Self::Split {
                        id: split_id,
                        axis,
                        ratio,
                        first: Box::new(Self::Leaf(original)),
                        second: Box::new(Self::Leaf(SplitterContainer::new(
                            new_leaf_id,
                            next_kind,
                        ))),
                    };
                    true
                } else {
                    false
                }
            }
            Self::Split { first, second, .. } => {
                first.split_leaf_with_ratio(
                    target_id,
                    split_id,
                    new_leaf_id,
                    axis,
                    ratio,
                    next_kind,
                ) || second.split_leaf_with_ratio(
                    target_id,
                    split_id,
                    new_leaf_id,
                    axis,
                    ratio,
                    next_kind,
                )
            }
        }
    }

    pub fn remove_leaf(&mut self, target_id: NodeId) -> bool {
        match self {
            Self::Leaf(_) => false,
            Self::Split { first, second, .. } => {
                if let Self::Leaf(container) = &**first {
                    if container.id == target_id {
                        *self = (**second).clone();
                        return true;
                    }
                }
                if let Self::Leaf(container) = &**second {
                    if container.id == target_id {
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
    pub fn join_leaf(&mut self, into_id: NodeId, target_id: NodeId) -> bool {
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
                    if target_in_first {
                        if let Self::Leaf(container) = &**first
                            && container.id == target_id
                        {
                            *self = (**second).clone();
                            return true;
                        }
                        if !first.remove_leaf(target_id) {
                            return false;
                        }
                    } else {
                        if let Self::Leaf(container) = &**second
                            && container.id == target_id
                        {
                            *self = (**first).clone();
                            return true;
                        }
                        if !second.remove_leaf(target_id) {
                            return false;
                        }
                    }
                    true
                }
            }
        }
    }

    pub fn set_split_ratio(&mut self, split_id: NodeId, new_ratio: f32) -> bool {
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
    /// panel's pane tree is an independent copy of the source panel's.
    pub fn clone_with_new_ids(&self, next_id: &mut NodeId) -> SplitTree<T> {
        match self {
            Self::Leaf(container) => {
                let id = *next_id;
                *next_id += 1;
                // The clone is a fresh panel: same kind, no interaction
                // state (no drag session, dropdown, or maximized).
                Self::Leaf(SplitterContainer::new(id, container.kind))
            }
            Self::Split {
                axis,
                ratio,
                first,
                second,
                ..
            } => {
                let id = *next_id;
                *next_id += 1;
                Self::Split {
                    id,
                    axis: *axis,
                    ratio: *ratio,
                    first: Box::new(first.clone_with_new_ids(next_id)),
                    second: Box::new(second.clone_with_new_ids(next_id)),
                }
            }
        }
    }

    pub fn swap_sibling_leaves(&mut self, split_id: NodeId) -> bool {
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
