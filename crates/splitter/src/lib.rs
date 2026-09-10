//! Tiled layout engine — the pure layout core shared by the window-level
//! panel layout (`SplitterRoot<PanelKind>`) and the editor's inner pane
//! layout (`SplitterRoot<PaneKind>`).
//!
//! This crate owns the split tree topology, pure geometry, drag-gesture
//! state machines, and the gesture policy. It renders nothing: every visual element
//! lives in the `ui` crate. Hosts drive gestures with mouse/keyboard events and
//! apply the returned [`CornerDragResult`] to their own panel/pane state.
//!
//! - [`SplitterContainer`] — one leaf container holding a typed payload `T` and [`LeafId`].
//! - [`SplitTree`] — the recursive binary split tree; every leaf is a [`SplitterContainer`].
//! - [`SplitterRoot`] — one split region: the tree plus tree-level state and allocator.
//! - [`geometry`] — spatial coordinates, rect math, and Blender-style snapping/dock algorithms.
//! - [`gesture`] — drag session records, modifier keys, and interaction states.
//! - [`interaction`] — transient UI interaction state manager.
//! - [`policy`] — translates gesture facts into tree mutations and structured results.
//! - [`id`] — strongly-typed identifiers: [`LeafId`], [`SplitId`], and [`NodeId`].
//! - [`error`] — domain error types and Result alias.

pub mod container;
pub mod error;
pub mod geometry;
pub mod gesture;
pub mod id;
pub mod interaction;
pub mod policy;
pub mod root;
pub mod tree;

pub use container::SplitterContainer;
pub use error::{Result as SplitterResult, SplitterError};
pub use geometry::{
    Direction, LeafRect, SplitAxis, calc_snapped_ratio, calculate_dock_target,
    calculate_join_slice_rect, id_at_point,
};
pub use gesture::{
    AreaDockTarget, BorderMenuState, CornerDragModifier, CornerDragSession,
    MODIFIER_THRESHOLD_PX, SplitterDragSession, past_shortcut_threshold,
};
pub use id::{LeafId, NodeId, NodeIdAllocator, SplitId};
pub use interaction::LayoutInteraction;
pub use policy::{ClonedContainer, CornerDragResult, apply_corner_drag_session};
pub use root::SplitterRoot;
pub use tree::SplitTree;

#[cfg(test)]
mod new_types_tests;
#[cfg(test)]
mod splitter_tests;
