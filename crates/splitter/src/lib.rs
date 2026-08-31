//! Tiled layout engine — the pure layout core shared by the window-level
//! panel layout (`SplitterRoot<PanelKindId>`) and the editor's inner
//! pane layout (`SplitterRoot<PaneKind>`).
//!
//! This module owns:
//! - [`SplitterContainer`] — the panel container (one leaf): the panel
//!   type `T` is the identity, and each container records its own
//!   interaction state.
//! - [`SplitTree`] — the recursive binary split tree; every leaf is a
//!   [`SplitterContainer`], so splitting a leaf creates a second container
//!   and both hang on the same tree.
//! - [`SplitterRoot`] — one initialized split region: the tree plus the
//!   tree-level state (id pool, activation, splitter drags).
//! - The drag-session records and the [`DragPolicy`] defaults.
//!
//! It depends only on gpui's geometry types (`Point`/`Size`/`Pixels`);
//! rendering and Editor state live in `src/editor`.

pub mod container;
pub mod interaction;
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
