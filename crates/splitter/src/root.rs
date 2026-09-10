//! [`SplitterRoot`] — one initialized split region: the tree of panel
//! containers plus the tree-level state and interaction manager.
//!
//! A root is created when a region is initialized as a split container
//! (the window body below the titlebar, or an editor's pane container).
//! All transient interaction state (drag sessions, dropdowns, maximize states)
//! is owned by [`LayoutInteraction`], while durable topology lives in [`SplitTree`].

use gpui::{Pixels, Point, Size};

use crate::container::SplitterContainer;
use crate::error::{Result, SplitterError};
use crate::geometry::{Direction, LeafRect, SplitAxis, id_at_point};
use crate::gesture::{
    AreaDockTarget, BorderMenuState, CornerDragModifier, CornerDragSession, SplitterDragSession,
};
use crate::id::{LeafId, NodeIdAllocator, SplitId};
use crate::interaction::LayoutInteraction;
use crate::policy::CornerDragResult;
use crate::tree::SplitTree;

/// One initialized split region: the panel tree plus tree-level state.
///
/// Only durable topology is serialized; drag sessions, menus, and interaction
/// state are transient and skipped.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(bound(
    serialize = "T: serde::Serialize",
    deserialize = "T: serde::Deserialize<'de>"
))]
pub struct SplitterRoot<T: Clone + PartialEq> {
    /// The tree of panel containers hanging on this root.
    pub tree: SplitTree<T>,
    /// Centralized ID allocator for this root layout.
    #[serde(rename = "next_node_id")]
    pub allocator: NodeIdAllocator,
    /// Transient layout interaction manager (drag sessions, dropdowns, maximize).
    #[serde(skip)]
    pub interaction: LayoutInteraction,
    /// The most recently activated leaf; hosts route global actions to this leaf.
    pub active_leaf: Option<LeafId>,
    /// Activation history, ordered oldest to newest. When the active leaf
    /// closes, focus moves to the previous entry in history instead of
    /// picking an arbitrary neighbor.
    pub activation_history: Vec<LeafId>,
}

impl<T: Clone + PartialEq> SplitterRoot<T> {
    /// A root with one leaf (one panel container), used to seed editor
    /// pane containers or new window panels.
    pub fn single_leaf<I: Into<LeafId>>(initial_id: I, kind: T) -> Self {
        let leaf_id = initial_id.into();
        Self {
            tree: SplitTree::Leaf(SplitterContainer::new(leaf_id, kind)),
            allocator: NodeIdAllocator::with_start(leaf_id.as_u32() + 1),
            interaction: LayoutInteraction::new(),
            active_leaf: None,
            activation_history: Vec::new(),
        }
    }

    // ------------------------------------------------------------------
    // Topology mutations: Split / Close / Kind / Join / Swap / Dock
    // ------------------------------------------------------------------

    /// Split `target` leaf at `ratio` with a sibling of the SAME kind.
    /// Returns the new leaf's id.
    pub fn split_leaf<I: Into<LeafId>>(
        &mut self,
        target: I,
        axis: SplitAxis,
        ratio: f32,
    ) -> Result<LeafId> {
        if let Some(max_leaf) = self.interaction.maximized_leaf() {
            return Err(SplitterError::LayoutMaximized(max_leaf));
        }
        if !(0.01..=0.99).contains(&ratio) {
            return Err(SplitterError::InvalidRatio(ratio));
        }
        let leaf_id = target.into();
        let kind = self
            .tree
            .find_leaf_kind(leaf_id)
            .ok_or(SplitterError::LeafNotFound(leaf_id))?;
        let split_id = self.allocator.next_split_id();
        let new_leaf_id = self.allocator.next_leaf_id();
        self.tree.split_leaf_with_ratio(
            leaf_id,
            split_id,
            new_leaf_id,
            axis,
            ratio,
            kind,
        )?;
        self.interaction.clear_border_menu();
        Ok(new_leaf_id)
    }

    /// Split at an internal divider: finds an actionable adjacent child leaf under `split_id`
    /// and splits it.
    pub fn split_divider<I: Into<SplitId>>(
        &mut self,
        split_id: I,
        axis: SplitAxis,
        ratio: f32,
    ) -> Result<LeafId> {
        let split_id = split_id.into();
        if let Some(max_leaf) = self.interaction.maximized_leaf() {
            return Err(SplitterError::LayoutMaximized(max_leaf));
        }
        let leaf_id = self
            .tree
            .find_split_second_leaf_id(split_id)
            .or_else(|| self.tree.find_split_first_leaf_id(split_id))
            .ok_or(SplitterError::SplitNotFound(split_id))?;
        self.split_leaf(leaf_id, axis, ratio)
    }

    /// Close a leaf; the last remaining leaf in the tree cannot be closed.
    pub fn close_leaf<I: Into<LeafId>>(&mut self, target: I) -> Result<()> {
        let leaf_id = target.into();
        self.tree.remove_leaf(leaf_id)?;
        self.retire_leaf(leaf_id);
        self.interaction.clear_border_menu();
        Ok(())
    }

    /// Close an actionable leaf adjacent to an internal divider. Returns the closed leaf's ID.
    pub fn close_divider<I: Into<SplitId>>(&mut self, split_id: I) -> Result<LeafId> {
        let split_id = split_id.into();
        let leaf_id = self
            .tree
            .find_split_second_leaf_id(split_id)
            .or_else(|| self.tree.find_split_first_leaf_id(split_id))
            .ok_or(SplitterError::SplitNotFound(split_id))?;
        self.close_leaf(leaf_id)?;
        Ok(leaf_id)
    }

    /// Mark `leaf_id` as the active leaf (the last leaf that received
    /// focus). Records the activation for fallback ordering.
    pub fn activate_leaf<I: Into<LeafId>>(&mut self, leaf_id: I) {
        let id = leaf_id.into();
        self.activation_history.retain(|node_id| *node_id != id);
        self.activation_history.push(id);
        self.active_leaf = Some(id);
    }

    /// Recompute the active leaf after the layout changed: the most
    /// recently activated leaf still present, or `None`.
    fn recompute_active_leaf(&mut self) {
        if self
            .active_leaf
            .is_some_and(|id| self.tree.find_leaf_kind(id).is_some())
        {
            return;
        }
        self.active_leaf = self
            .activation_history
            .iter()
            .rev()
            .copied()
            .find(|id| self.tree.find_leaf_kind(*id).is_some());
    }

    /// Drop a leaf from activation tracking and recompute the active leaf.
    fn retire_leaf<I: Into<LeafId>>(&mut self, removed: I) {
        let removed = removed.into();
        self.activation_history.retain(|id| *id != removed);
        if self.active_leaf == Some(removed) {
            self.active_leaf = None;
        }
        self.interaction.on_leaf_removed(removed);
        self.recompute_active_leaf();
    }

    /// Change a leaf's kind. A changed kind drops the leaf from the
    /// activation history — the host re-activates it when the new kind
    /// should be the active one.
    pub fn set_kind<I: Into<LeafId>>(&mut self, leaf_id: I, kind: T) -> Result<()> {
        let leaf_id = leaf_id.into();
        let previous = self
            .tree
            .find_leaf_kind(leaf_id)
            .ok_or(SplitterError::LeafNotFound(leaf_id))?;
        self.tree.set_leaf_kind(leaf_id, kind.clone())?;
        if previous != kind {
            self.retire_leaf(leaf_id);
        }
        self.interaction.clear_border_menu();
        Ok(())
    }

    /// Join `removed` into `into`. The removed leaf is closed and its
    /// space is absorbed by the `into` leaf. The two must be adjacent
    /// (share an edge) in the layout.
    pub fn join_leaves<I1: Into<LeafId>, I2: Into<LeafId>>(
        &mut self,
        into: I1,
        removed: I2,
    ) -> Result<()> {
        let into_id = into.into();
        let removed_id = removed.into();
        self.tree.join_leaf(into_id, removed_id)?;
        self.retire_leaf(removed_id);
        self.interaction.clear_border_menu();
        Ok(())
    }

    /// Swap the kind of leaf `a` and leaf `b`. Both leaves leave the
    /// activation history (same rule as [`Self::set_kind`]).
    pub fn swap_kinds<I1: Into<LeafId>, I2: Into<LeafId>>(
        &mut self,
        a: I1,
        b: I2,
    ) -> Result<()> {
        let a = a.into();
        let b = b.into();
        let kind_a = self
            .tree
            .find_leaf_kind(a)
            .ok_or(SplitterError::LeafNotFound(a))?;
        let kind_b = self
            .tree
            .find_leaf_kind(b)
            .ok_or(SplitterError::LeafNotFound(b))?;
        self.tree.set_leaf_kind(a, kind_b)?;
        self.tree.set_leaf_kind(b, kind_a)?;
        self.retire_leaf(a);
        self.retire_leaf(b);
        Ok(())
    }

    /// Swaps the children of a split divider.
    pub fn swap_split_sides<I: Into<SplitId>>(&mut self, split_id: I) -> Result<()> {
        let split_id = split_id.into();
        self.tree.swap_sibling_leaves(split_id)?;
        self.interaction.clear_border_menu();
        Ok(())
    }

    /// Move a source leaf and dock it onto a target leaf at the specified edge,
    /// closing the source leaf and rearranging its former neighbors, while splitting
    /// the target leaf to accommodate the moved leaf at the specified ratio.
    /// If `dock_target` is `Center`, it swaps kinds between source and target instead.
    pub fn move_and_dock_leaf<I1: Into<LeafId>, I2: Into<LeafId>>(
        &mut self,
        source_id: I1,
        target_id: I2,
        dock_target: AreaDockTarget,
        ratio: f32,
    ) -> Result<Option<LeafId>> {
        let source_id = source_id.into();
        let target_id = target_id.into();
        if let Some(max_leaf) = self.interaction.maximized_leaf() {
            return Err(SplitterError::LayoutMaximized(max_leaf));
        }
        if source_id == target_id {
            return Err(SplitterError::SameLeaf(source_id));
        }
        if dock_target == AreaDockTarget::Center {
            self.swap_kinds(source_id, target_id)?;
            return Ok(None);
        }
        let source_kind = self
            .tree
            .find_leaf_kind(source_id)
            .ok_or(SplitterError::LeafNotFound(source_id))?;
        let target_kind = self
            .tree
            .find_leaf_kind(target_id)
            .ok_or(SplitterError::LeafNotFound(target_id))?;

        // 1. Remove source leaf (collapsing source parent Split node and expanding neighbors)
        self.tree.remove_leaf(source_id)?;
        self.retire_leaf(source_id);

        // 2. Allocate new IDs
        let split_id = self.allocator.next_split_id();
        let new_leaf_id = self.allocator.next_leaf_id();

        // 3. Split target leaf based on dock_target
        let (axis, split_ratio, source_first) = match dock_target {
            AreaDockTarget::Left => (SplitAxis::Horizontal, ratio.clamp(0.01, 0.99), true),
            AreaDockTarget::Right => (
                SplitAxis::Horizontal,
                (1.0 - ratio).clamp(0.01, 0.99),
                false,
            ),
            AreaDockTarget::Top => (SplitAxis::Vertical, ratio.clamp(0.01, 0.99), true),
            AreaDockTarget::Bottom => (
                SplitAxis::Vertical,
                (1.0 - ratio).clamp(0.01, 0.99),
                false,
            ),
            _ => (SplitAxis::Horizontal, 0.5, true),
        };

        if source_first {
            self.tree.split_leaf_with_ratio(
                target_id,
                split_id,
                new_leaf_id,
                axis,
                split_ratio,
                target_kind,
            )?;
            self.tree.set_leaf_kind(target_id, source_kind)?;
        } else {
            self.tree.split_leaf_with_ratio(
                target_id,
                split_id,
                new_leaf_id,
                axis,
                split_ratio,
                source_kind,
            )?;
        }
        self.interaction.clear_border_menu();
        Ok(Some(new_leaf_id))
    }

    // ------------------------------------------------------------------
    // High-level session starters (Tree + Interaction coordination)
    // ------------------------------------------------------------------

    /// Finds the leaf container that is currently maximized, if any.
    pub fn find_maximized_leaf(&self) -> Option<&SplitterContainer<T>> {
        let leaf_id = self.interaction.maximized_leaf()?;
        self.tree.find_leaf(leaf_id)
    }

    /// Begin a splitter-bar drag on a split divider node.
    pub fn start_splitter_drag<I: Into<SplitId>>(
        &mut self,
        split_id: I,
        axis: SplitAxis,
        start_pointer_pos: f32,
        current_ratio: f32,
    ) {
        if self.interaction.is_maximized() {
            return;
        }
        let split_id = split_id.into();
        self.interaction
            .start_splitter_drag(SplitterDragSession {
                split_id,
                axis,
                start_pointer_pos,
                start_ratio: current_ratio,
                total_span: 1000.0,
            });
    }

    /// Open the border context menu on a split bar (right click).
    pub fn open_border_menu<I: Into<SplitId>>(&mut self, split_id: I, position: Point<Pixels>) {
        if self.interaction.is_maximized() {
            return;
        }
        let split_id = split_id.into();
        self.interaction
            .open_border_menu(BorderMenuState {
                split_id,
                position,
            });
    }

    /// Begin a corner-drag gesture from `target_id`'s panel at `pos`.
    pub fn start_corner_drag<I: Into<LeafId>>(
        &mut self,
        target_id: I,
        pos: Point<Pixels>,
        modifier: CornerDragModifier,
    ) {
        if self.interaction.is_maximized() {
            return;
        }
        let target_id = target_id.into();
        if self.tree.contains_leaf(target_id) {
            self.interaction
                .start_corner_drag(CornerDragSession {
                    target_id,
                    start_pos: pos,
                    gesture_dir: None,
                    modifier,
                    pointer_pos: Some(pos),
                    hover_leaf: None,
                    dock_target: AreaDockTarget::None,
                    dock_ratio: 0.5,
                });
        }
    }

    // ------------------------------------------------------------------
    // Gesture lifecycle operations
    // ------------------------------------------------------------------

    /// Progress the active drag gesture (splitter bar or corner drag).
    pub fn update_drag_gesture(&mut self, pos: Point<Pixels>, viewport: Size<Pixels>) -> bool {
        if let Some(drag) = self.interaction.active_splitter_drag().copied() {
            let current_pos = match drag.axis {
                SplitAxis::Horizontal => f32::from(pos.x),
                SplitAxis::Vertical => f32::from(pos.y),
            };
            let span = self
                .split_pixel_span(drag.split_id, viewport)
                .unwrap_or_else(|| match drag.axis {
                    SplitAxis::Horizontal => f32::from(viewport.width),
                    SplitAxis::Vertical => f32::from(viewport.height),
                });
            if span > 1.0 {
                let mut session = drag;
                session.total_span = span;
                self.interaction.start_splitter_drag(session);
            }
            if let Some(session) = self.interaction.active_splitter_drag() {
                if session.total_span > 1.0 {
                    let new_ratio = session.ratio_at(current_pos);
                    let _ = self.tree.set_split_ratio(session.split_id, new_ratio);
                }
            }
            true
        } else if self.interaction.active_corner_drag().is_some() {
            self.update_corner_drag(pos, viewport);
            true
        } else {
            false
        }
    }

    /// Finish and apply the active drag gesture (splitter bar or corner drag).
    pub fn finish_drag_gesture(
        &mut self,
        container_size: Size<Pixels>,
    ) -> Option<CornerDragResult<T>> {
        if self.interaction.is_splitter_dragging() {
            self.interaction.finish_splitter_drag();
            Some(CornerDragResult::None)
        } else if self.interaction.active_corner_drag().is_some() {
            Some(self.apply_corner_drag(container_size))
        } else {
            None
        }
    }

    /// Cancel the active drag gesture without applying it (e.g. Escape key).
    pub fn cancel_drag_gesture(&mut self) -> bool {
        if let Some(drag) = self.interaction.cancel_splitter_drag() {
            let _ = self.tree.set_split_ratio(drag.split_id, drag.start_ratio);
            return true;
        }
        if self.interaction.cancel_corner_drag() {
            return true;
        }
        false
    }

    /// Process a mouse-move event during a corner drag.
    fn update_corner_drag(
        &mut self,
        current_pos: Point<Pixels>,
        container_size: Size<Pixels>,
    ) -> bool {
        let Some(session) = self.interaction.active_corner_drag().copied() else {
            return false;
        };
        let target_id = session.target_id;

        let dx = f32::from(current_pos.x - session.start_pos.x);
        let dy = f32::from(current_pos.y - session.start_pos.y);
        let abs_dx = dx.abs();
        let abs_dy = dy.abs();

        let dir = if abs_dy > abs_dx {
            if dy > 0.0 {
                Direction::Down
            } else {
                Direction::Up
            }
        } else if dx > 0.0 {
            Direction::Right
        } else {
            Direction::Left
        };

        let leaf_rects = self.leaf_rects(container_size);
        let over_id = id_at_point(&leaf_rects, current_pos);

        let (dock_target, dock_ratio) = if let Some(over) = over_id {
            if over != target_id {
                let target_rect = leaf_rects.iter().find(|r| r.id == over);
                let source_rect = leaf_rects.iter().find(|r| r.id == target_id);
                if let (Some(target_rect), Some(source_rect)) = (target_rect, source_rect) {
                    crate::geometry::calculate_dock_target(
                        source_rect,
                        target_rect,
                        current_pos,
                        session.modifier == CornerDragModifier::Ctrl,
                    )
                } else {
                    (AreaDockTarget::None, 0.5)
                }
            } else {
                (AreaDockTarget::None, 0.5)
            }
        } else {
            (AreaDockTarget::None, 0.5)
        };

        self.interaction
            .start_corner_drag(CornerDragSession {
                target_id,
                start_pos: session.start_pos,
                gesture_dir: Some(dir),
                modifier: session.modifier,
                pointer_pos: Some(current_pos),
                hover_leaf: over_id,
                dock_target,
                dock_ratio,
            });
        true
    }

    /// Finish and apply the active corner-drag gesture on mouse release.
    fn apply_corner_drag(
        &mut self,
        container_size: Size<Pixels>,
    ) -> CornerDragResult<T> {
        let Some(facts) = self.interaction.finish_corner_drag() else {
            return CornerDragResult::None;
        };
        if self.interaction.is_maximized() {
            return CornerDragResult::None;
        }
        crate::policy::apply_corner_drag_session(self, &facts, container_size)
    }

    // ------------------------------------------------------------------
    // Geometry calculations
    // ------------------------------------------------------------------

    /// Collect all leaf rectangles in pixel coordinates.
    pub fn leaf_rects(&self, container_size: Size<Pixels>) -> Vec<LeafRect> {
        let w = f32::from(container_size.width);
        let h = f32::from(container_size.height);
        let mut rects = Vec::new();
        if w > 0.0 && h > 0.0 {
            let mut norm = Vec::new();
            self.tree.collect_leaf_rects(0.0, 0.0, 1.0, 1.0, &mut norm);
            for rect in norm {
                rects.push(LeafRect {
                    id: rect.id,
                    x: rect.x * w,
                    y: rect.y * h,
                    width: rect.width * w,
                    height: rect.height * h,
                });
            }
        }
        rects
    }

    /// Calculate the pixel span (width or height) of a split container.
    pub fn split_pixel_span<I: Into<SplitId>>(
        &self,
        split_id: I,
        container_size: Size<Pixels>,
    ) -> Option<f32> {
        let w = f32::from(container_size.width);
        let h = f32::from(container_size.height);
        let (_, span) = self.tree.find_split_span(split_id, w, h)?;
        Some(span)
    }

    /// Compute the split axis and ratio for a finished corner drag from the session facts.
    pub fn corner_split_facts(
        &self,
        facts: &CornerDragSession,
        container_size: Size<Pixels>,
    ) -> Option<(SplitAxis, f32)> {
        let dir = facts.gesture_dir?;
        let pos = facts.pointer_pos?;
        let mut rects = Vec::new();
        self.tree.collect_leaf_rects(0.0, 0.0, 1.0, 1.0, &mut rects);
        let target = rects.iter().find(|rect| rect.id == facts.target_id)?;
        let axis = if dir.is_vertical() {
            SplitAxis::Vertical
        } else {
            SplitAxis::Horizontal
        };
        let norm_x = f32::from(pos.x) / f32::from(container_size.width);
        let norm_y = f32::from(pos.y) / f32::from(container_size.height);
        let raw_ratio = match axis {
            SplitAxis::Horizontal => (norm_x - target.x) / target.width,
            SplitAxis::Vertical => (norm_y - target.y) / target.height,
        };
        let ratio = crate::geometry::calc_snapped_ratio(
            raw_ratio,
            facts.modifier == CornerDragModifier::Ctrl,
        );
        Some((axis, ratio))
    }
}
