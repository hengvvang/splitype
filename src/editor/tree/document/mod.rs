//! Runtime ownership for the editor block tree.
//!
//! [`Document`] is the only mutable owner of block ordering and parent-child
//! relationships inside the editor. It also maintains a cached
//! [`BlockIndex`] so hot-path lookups do not re-run a full DFS on every
//! focus, scroll, or mutation event.

pub(crate) mod index;
pub(crate) mod mutations;
pub(crate) mod queries;
pub(crate) mod serialization;

pub(crate) use index::*;

use gpui::*;

use crate::editor::tree::block::Block;

/// Canonical owner of the runtime block tree.
#[derive(Clone)]
pub(crate) struct Document {
    pub roots: Vec<Entity<Block>>,
    pub(crate) index: BlockIndex,
    pub(crate) structure_version: u64,
    pub(crate) metadata_rebuild_version: u64,
}
