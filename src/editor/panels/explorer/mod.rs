//! Explorer sidebar state and file-tree model.
//!
//! Owned by the Editor entity (`WindowPanels::explorer`), so these files
//! live inside `editor` — the explorer VIEW (interactions, rendering) is
//! the top-level `src/explorer` module, which depends on this state.
//! The outline panel has its own state in `crate::editor::panels::outline`.

pub(crate) mod state;
pub(crate) mod undo;
pub(crate) mod utils;
pub(crate) mod worktree;
