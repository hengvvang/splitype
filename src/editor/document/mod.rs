//! Runtime ownership for the editor block tree.
//!
//! [Document] is the only mutable owner of block ordering and parent-child
//! relationships inside the editor. It also maintains a cached
//! [BlockIndex] so hot-path lookups do not re-run a full DFS on every
//! focus, scroll, or mutation event.

pub mod block;
pub(crate) mod index;
pub(crate) mod loader;
#[cfg(test)]
mod loader_tests;
pub(crate) mod mutations;
pub mod protocol;
pub(crate) mod queries;
pub(crate) mod serialization;
pub(crate) mod serialize;

pub(crate) use index::*;
pub use block::Block;
pub use protocol::BlockEvent;

use gpui::*;

/// Canonical owner of the runtime block tree.
#[derive(Clone)]
pub struct Document {
    pub roots: Vec<Entity<Block>>,
    pub(crate) tree: splitype_model::tree::SumTree<crate::model::parse::BlockData>,
    pub(crate) index: BlockIndex,
    pub(crate) structure_version: u64,
    pub(crate) metadata_rebuild_version: u64,
}
