//! Undo/redo delta model of the WYSIWYG world.
//!
//! `delta` holds the pure transaction/delta data types; the editor-side
//! undo stack (capture, coalescing, restore) stays in the coordinating
//! crate until the `Editor` entity converges.

pub mod delta;

pub use delta::{DocDelta, Transaction};

