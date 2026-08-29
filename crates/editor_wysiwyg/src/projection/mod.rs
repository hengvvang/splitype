//! Inline WYSIWYG projection engine: 3D cursor mapping, delimiter expansion, and offsets.

pub mod edits;
pub mod engine;
pub mod lifecycle;
pub mod offsets;

pub use engine::*;
