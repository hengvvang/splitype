//! Tiled layout engine — the pure layout core shared by the window-level
//! area layout (`WindowAreaKind`) and the editor's inner panel layout
//! (`EditorInnerPanelKind`).
//!
//! This module owns:
//! - [`SplitTree`] — the recursive binary split tree and its operations
//!   (split / join / swap / geometry).
//! - [`WindowLayout`] — the full layout state: the outer tree, per-Edit
//!   inner trees, and every active drag / menu session.
//! - The drag-session records and corner-drag action vocabulary.
//!
//! It depends only on gpui's geometry types (`Point`/`Size`/`Pixels`);
//! rendering and Editor state live in `src/windows` / `src/editor`.

pub mod sessions;
pub mod state;
pub mod tree;
pub mod types;

pub use sessions::*;
pub use state::*;
pub use tree::*;
pub use types::*;
