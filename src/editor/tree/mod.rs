//! Document tree — the block entity, its container, and tree-level
//! load/serialize/annotation support.
//!
//! `block.rs` holds the `Block` entity and its core data/editing behavior;
//! the edit-mode enumeration, tree-metadata flags, inline projection
//! engine, and code-language input are split into sibling files so each
//! concern stays under ~1.3k lines.

pub mod block;
pub mod block_code_language;
pub mod block_edit_mode;
pub mod block_metadata;
pub mod block_projection;
pub mod document;
pub mod footnotes;
pub mod loader;
pub mod serialize;
