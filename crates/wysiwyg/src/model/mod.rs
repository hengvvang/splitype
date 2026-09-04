//! Runtime ownership for the editor block tree and document model.
//!
//! [Document] is the mutable owner of block ordering and parent-child
//! relationships inside the editor. It also maintains a cached
//! [BlockIndex] so hot-path lookups do not re-run a full DFS on every
//! focus, scroll, or mutation event.

pub mod block;
pub mod index;
pub mod mutations;
pub mod protocol;
pub mod queries;
pub mod references;
pub mod serialize;

pub use block::Block;
pub use index::*;
pub use protocol::BlockEvent;

use gpui::*;

/// Canonical owner of the runtime block tree.
#[derive(Clone)]
pub struct Document {
    pub roots: Vec<Entity<Block>>,
    pub index: BlockIndex,
    pub structure_version: u64,
    pub metadata_rebuild_version: u64,
}
