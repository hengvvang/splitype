//! Tiled layout engine — the pure layout core shared by the window-level
//! area layout (`SplitterRoot<WindowAreaKind>`) and the editor's inner
//! panel layout (`SplitterRoot<EditorInnerPanelKind>`).
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
pub use policy::{ClonedContainer, DragPolicy};
pub use root::SplitterRoot;
pub use sessions::{BorderMenuState, CornerDragModifier};
pub use tree::{Axis, NodeId, SplitTree};
