//! The recursive binary split tree and its operations.
//!
//! Generic over the kind type `T` so that the outer layout uses
//! `platform_contracts::PanelKind` while inner (edit pane) layouts use
//! `editor_contracts::PaneKind`.

use crate::container::SplitterContainer;
use crate::error::{Result, SplitterError};
use crate::id::{LeafId, NodeIdAllocator, SplitId};

pub use crate::geometry::{Direction, LeafRect, SplitAxis};

/// Recursive binary layout tree representing tiled leaves and splitters.
///
/// Design is inspired by Blender's screen area action-zone system: each
/// area exposes four corner hot-zones that, when dragged, produce either a
/// split (same area), a join (neighbour area), a swap (Ctrl), or a
/// window-clone (Shift) – with differentiated gesture thresholds and
/// directional cursors.
///
/// Every leaf is a [`SplitterContainer`]. Splitting a leaf replaces it with a
/// `Split` node holding two containers — the original and the freshly created one —
/// both hanging on this tree.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(bound(
    serialize = "T: serde::Serialize",
    deserialize = "T: serde::Deserialize<'de>"
))]
pub enum SplitTree<T: Clone> {
    Leaf(SplitterContainer<T>),
    Split {
        id: SplitId,
        axis: SplitAxis,
        ratio: f32,
        first: Box<SplitTree<T>>,
        second: Box<SplitTree<T>>,
    },
}

impl<T: Clone + PartialEq> PartialEq for SplitTree<T> {
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

impl<T: Clone + PartialEq> SplitTree<T> {
    pub fn count_leaves(&self) -> usize {
        match self {
            Self::Leaf(_) => 1,
            Self::Split { first, second, .. } => first.count_leaves() + second.count_leaves(),
        }
    }

    /// Returns the ID of the very first leaf in depth-first order.
    pub fn first_leaf_id(&self) -> Option<LeafId> {
        match self {
            Self::Leaf(container) => Some(container.id),
            Self::Split { first, .. } => first.first_leaf_id(),
        }
    }

    pub fn find_leaf_kind<I: Into<LeafId>>(&self, leaf_id: I) -> Option<T> {
        let leaf_id = leaf_id.into();
        match self {
            Self::Leaf(container) => (container.id == leaf_id).then_some(container.kind.clone()),
            Self::Split { first, second, .. } => first
                .find_leaf_kind(leaf_id)
                .or_else(|| second.find_leaf_kind(leaf_id)),
        }
    }

    /// The container (panel) of the leaf with `leaf_id`, if any.
    pub fn find_leaf<I: Into<LeafId>>(&self, leaf_id: I) -> Option<&SplitterContainer<T>> {
        let leaf_id = leaf_id.into();
        match self {
            Self::Leaf(container) => (container.id == leaf_id).then_some(container),
            Self::Split { first, second, .. } => first
                .find_leaf(leaf_id)
                .or_else(|| second.find_leaf(leaf_id)),
        }
    }

    /// The container (panel) of the leaf with `leaf_id`, mutably.
    pub fn find_leaf_mut<I: Into<LeafId>>(&mut self, leaf_id: I) -> Option<&mut SplitterContainer<T>> {
        let leaf_id = leaf_id.into();
        match self {
            Self::Leaf(container) => (container.id == leaf_id).then_some(container),
            Self::Split { first, second, .. } => first
                .find_leaf_mut(leaf_id)
                .or_else(|| second.find_leaf_mut(leaf_id)),
        }
    }

    /// Sets a leaf's kind by id, returning an error if the leaf was not found.
    pub fn set_leaf_kind<I: Into<LeafId>>(&mut self, leaf_id: I, new_kind: T) -> Result<()> {
        let leaf_id = leaf_id.into();
        if self.set_leaf_kind_internal(leaf_id, new_kind) {
            Ok(())
        } else {
            Err(SplitterError::LeafNotFound(leaf_id))
        }
    }

    fn set_leaf_kind_internal(&mut self, leaf_id: LeafId, new_kind: T) -> bool {
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
                first.set_leaf_kind_internal(leaf_id, new_kind.clone())
                    || second.set_leaf_kind_internal(leaf_id, new_kind)
            }
        }
    }

    /// Does the subtree contain the leaf with `leaf_id`?
    pub fn contains_leaf<I: Into<LeafId>>(&self, leaf_id: I) -> bool {
        let leaf_id = leaf_id.into();
        match self {
            Self::Leaf(container) => container.id == leaf_id,
            Self::Split { first, second, .. } => {
                first.contains_leaf(leaf_id) || second.contains_leaf(leaf_id)
            }
        }
    }

    /// Does the subtree contain the split divider with `split_id`?
    pub fn contains_split<I: Into<SplitId>>(&self, split_id: I) -> bool {
        let split_id = split_id.into();
        match self {
            Self::Leaf(_) => false,
            Self::Split { id, first, second, .. } => {
                *id == split_id || first.contains_split(split_id) || second.contains_split(split_id)
            }
        }
    }

    /// Collect all leaf ids in tree order.
    pub fn leaf_ids(&self, out: &mut Vec<LeafId>) {
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
    pub fn find_split_span<I: Into<SplitId>>(
        &self,
        target_split_id: I,
        w: f32,
        h: f32,
    ) -> Option<(SplitAxis, f32)> {
        let target_split_id = target_split_id.into();
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
    pub fn find_split_first_leaf_id<I: Into<SplitId>>(&self, target_split_id: I) -> Option<LeafId> {
        let target_split_id = target_split_id.into();
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
    pub fn find_split_second_leaf_id<I: Into<SplitId>>(&self, target_split_id: I) -> Option<LeafId> {
        let target_split_id = target_split_id.into();
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

    /// Split a leaf at a specific ratio.
    pub fn split_leaf_with_ratio<L1: Into<LeafId>, S: Into<SplitId>, L2: Into<LeafId>>(
        &mut self,
        target_id: L1,
        split_id: S,
        new_leaf_id: L2,
        axis: SplitAxis,
        ratio: f32,
        next_kind: T,
    ) -> Result<()> {
        let target_id = target_id.into();
        let split_id = split_id.into();
        let new_leaf_id = new_leaf_id.into();
        if !(0.01..=0.99).contains(&ratio) {
            return Err(SplitterError::InvalidRatio(ratio));
        }
        if self.split_leaf_internal(target_id, split_id, new_leaf_id, axis, ratio, next_kind) {
            Ok(())
        } else {
            Err(SplitterError::LeafNotFound(target_id))
        }
    }

    fn split_leaf_internal(
        &mut self,
        target_id: LeafId,
        split_id: SplitId,
        new_leaf_id: LeafId,
        axis: SplitAxis,
        ratio: f32,
        next_kind: T,
    ) -> bool {
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
                first.split_leaf_internal(
                    target_id,
                    split_id,
                    new_leaf_id,
                    axis,
                    ratio,
                    next_kind.clone(),
                ) || second.split_leaf_internal(
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

    /// Removes a leaf from the tree, collapsing its parent split node and returning the removed leaf's kind.
    pub fn remove_leaf<I: Into<LeafId>>(&mut self, target_id: I) -> Result<T> {
        let target_id = target_id.into();
        match self {
            Self::Leaf(_) => Err(SplitterError::CannotRemoveLastLeaf),
            Self::Split { .. } => {
                self.remove_leaf_internal(target_id)
                    .ok_or(SplitterError::LeafNotFound(target_id))
            }
        }
    }

    fn remove_leaf_internal(&mut self, target_id: LeafId) -> Option<T> {
        let (first_kind, second_kind) = match self {
            Self::Leaf(_) => return None,
            Self::Split { first, second, .. } => {
                let first_match = match &**first {
                    Self::Leaf(c) if c.id == target_id => Some(c.kind.clone()),
                    _ => None,
                };
                let second_match = match &**second {
                    Self::Leaf(c) if c.id == target_id => Some(c.kind.clone()),
                    _ => None,
                };
                (first_match, second_match)
            }
        };

        if let Some(kind) = first_kind {
            let dummy = Self::Leaf(SplitterContainer::new(target_id, kind.clone()));
            let old = std::mem::replace(self, dummy);
            if let Self::Split { second, .. } = old {
                *self = *second;
                return Some(kind);
            }
        }

        if let Some(kind) = second_kind {
            let dummy = Self::Leaf(SplitterContainer::new(target_id, kind.clone()));
            let old = std::mem::replace(self, dummy);
            if let Self::Split { first, .. } = old {
                *self = *first;
                return Some(kind);
            }
        }

        match self {
            Self::Split { first, second, .. } => first
                .remove_leaf_internal(target_id)
                .or_else(|| second.remove_leaf_internal(target_id)),
            _ => None,
        }
    }

    /// Join `target_id` into `into_id`. The `target_id` leaf is removed and
    /// `into_id` expands to fill the space. Both leaves must share an immediate
    /// split parent (be adjacent siblings).
    pub fn join_leaf<I1: Into<LeafId>, I2: Into<LeafId>>(
        &mut self,
        into_id: I1,
        target_id: I2,
    ) -> Result<()> {
        let into_id = into_id.into();
        let target_id = target_id.into();
        if into_id == target_id {
            return Err(SplitterError::SameLeaf(into_id));
        }
        if !self.contains_leaf(into_id) {
            return Err(SplitterError::LeafNotFound(into_id));
        }
        if !self.contains_leaf(target_id) {
            return Err(SplitterError::LeafNotFound(target_id));
        }
        if self.join_leaf_internal(into_id, target_id) {
            Ok(())
        } else {
            Err(SplitterError::NotAdjacent(into_id, target_id))
        }
    }

    fn join_leaf_internal(&mut self, into_id: LeafId, target_id: LeafId) -> bool {
        let (in_first, target_is_first, target_is_second) = match self {
            Self::Leaf { .. } => return false,
            Self::Split { first, second, .. } => {
                let into_in_first = first.contains_leaf(into_id);
                let target_in_first = first.contains_leaf(target_id);

                if into_in_first && target_in_first {
                    return first.join_leaf_internal(into_id, target_id);
                } else if !into_in_first && !target_in_first {
                    return second.join_leaf_internal(into_id, target_id);
                }

                let target_is_first = match &**first {
                    Self::Leaf(c) if c.id == target_id => Some(c.kind.clone()),
                    _ => None,
                };
                let target_is_second = match &**second {
                    Self::Leaf(c) if c.id == target_id => Some(c.kind.clone()),
                    _ => None,
                };
                (into_in_first, target_is_first, target_is_second)
            }
        };

        if let Some(kind) = target_is_first {
            let dummy = Self::Leaf(SplitterContainer::new(target_id, kind));
            let old = std::mem::replace(self, dummy);
            if let Self::Split { second, .. } = old {
                *self = *second;
                return true;
            }
        }

        if let Some(kind) = target_is_second {
            let dummy = Self::Leaf(SplitterContainer::new(target_id, kind));
            let old = std::mem::replace(self, dummy);
            if let Self::Split { first, .. } = old {
                *self = *first;
                return true;
            }
        }

        match self {
            Self::Split { first, second, .. } => {
                if in_first {
                    second.remove_leaf_internal(target_id).is_some()
                } else {
                    first.remove_leaf_internal(target_id).is_some()
                }
            }
            _ => false,
        }
    }

    pub fn set_split_ratio<I: Into<SplitId>>(&mut self, split_id: I, new_ratio: f32) -> Result<()> {
        let split_id = split_id.into();
        if self.set_split_ratio_internal(split_id, new_ratio) {
            Ok(())
        } else {
            Err(SplitterError::SplitNotFound(split_id))
        }
    }

    fn set_split_ratio_internal(&mut self, split_id: SplitId, new_ratio: f32) -> bool {
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
                    first.set_split_ratio_internal(split_id, new_ratio)
                        || second.set_split_ratio_internal(split_id, new_ratio)
                }
            }
        }
    }

    /// Deep-clone this subtree, assigning fresh node ids from an id allocator.
    pub fn clone_with_allocator(&self, allocator: &mut NodeIdAllocator) -> SplitTree<T> {
        match self {
            Self::Leaf(container) => {
                let id = allocator.next_leaf_id();
                Self::Leaf(SplitterContainer::new(id, container.kind.clone()))
            }
            Self::Split {
                axis,
                ratio,
                first,
                second,
                ..
            } => {
                let id = allocator.next_split_id();
                Self::Split {
                    id,
                    axis: *axis,
                    ratio: *ratio,
                    first: Box::new(first.clone_with_allocator(allocator)),
                    second: Box::new(second.clone_with_allocator(allocator)),
                }
            }
        }
    }

    pub fn swap_sibling_leaves<I: Into<SplitId>>(&mut self, split_id: I) -> Result<()> {
        let split_id = split_id.into();
        if self.swap_sibling_leaves_internal(split_id) {
            Ok(())
        } else {
            Err(SplitterError::SplitNotFound(split_id))
        }
    }

    fn swap_sibling_leaves_internal(&mut self, split_id: SplitId) -> bool {
        match self {
            Self::Leaf { .. } => false,
            Self::Split {
                id, first, second, ..
            } => {
                if *id == split_id {
                    std::mem::swap(first, second);
                    true
                } else {
                    first.swap_sibling_leaves_internal(split_id)
                        || second.swap_sibling_leaves_internal(split_id)
                }
            }
        }
    }
}
