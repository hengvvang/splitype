//! Pane trait — the cross-mode plugin contract of the editor family.
//!
//! The contract lives in the `editor` crate; this module re-exports it
//! (and its `OutlineNode` data type) so the migration-in-progress code
//! keeps one canonical path. The re-export disappears with
//! `crates/splitype` in stage 3i.

pub use editor_core::{OutlineNode, Pane};
