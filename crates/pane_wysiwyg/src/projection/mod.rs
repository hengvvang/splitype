//! Inline WYSIWYG projection engine: 3D cursor mapping, delimiter expansion, offsets, and source map.

pub mod edits;
pub mod engine;
pub mod lifecycle;
pub mod offsets;
pub mod source_map;

pub use engine::*;
pub use source_map::*;
