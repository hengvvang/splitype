//! Explorer sidebar state and file-tree model.
//!
//! Owned by `WindowPanels::explorer` in `crate::app`. The explorer VIEW
//! (interactions, rendering) lives in this crate's sibling modules and
//! depends on this state one-way. The outline panel has its own state in
//! `crate::editor::outline`, so the editor never imports this module.

pub mod state;
pub mod undo;
pub mod utils;
pub mod worktree;
