//! Tiled layout engine — the pure layout core shared by the window-level
//! panel layout (`SplitterRoot<PanelKind>`) and the editor's inner pane
//! layout (`SplitterRoot<PaneKind>`).
//!
//! This crate owns the split tree topology, its geometry, the drag-gesture
//! state machines and the gesture policy. It renders nothing: every split
//! visual (divider bars, corner handles, border context menus, drag
//! previews) lives in the `ui` crate. Hosts (the window shell, the editor)
//! drive the gestures with mouse/keyboard events and apply the returned
//! [`policy::CornerDragResult`] to their own panel/pane state.
//!
//! The engine depends only on gpui's geometry types (`Point` / `Size` /
//! `Pixels`) plus serde/schemars for persistence; panel and pane semantics
//! stay in the hosts.
//!
//! - [`SplitterContainer`] — one leaf: the panel type `T` is the identity,
//!   and each container records its own interaction state.
//! - [`SplitTree`] — the recursive binary split tree; every leaf is a
//!   [`SplitterContainer`].
//! - [`SplitterRoot`] — one split region: the tree plus tree-level state
//!   (id pool, activation, drag sessions, border menu).
//! - [`sessions`] — raw gesture-fact records and pure geometry math.
//! - [`policy`] — translates finished gesture facts into tree mutations
//!   and a structured [`policy::CornerDragResult`].

pub mod container;
pub mod policy;
pub mod root;
pub mod sessions;
pub mod tree;

pub use container::SplitterContainer;
pub use policy::{ClonedContainer, CornerDragResult, apply_corner_drag_session};
pub use root::SplitterRoot;
pub use sessions::{
    AreaDockTarget, BorderMenuState, CornerDragModifier, CornerDragSession, calc_snapped_ratio,
    calculate_dock_target, calculate_join_slice_rect,
};
pub use tree::{Direction, LeafRect, NodeId, SplitAxis, SplitTree};

#[cfg(test)]
mod splitter_tests;
