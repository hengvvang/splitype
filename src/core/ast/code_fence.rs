//! Fenced code block opening metadata.
//!
//! Records the fence character and run length so only a matching
//! closing fence can terminate the block.

use gpui::SharedString;

/// Opening fence parsed from a fenced code block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeFenceOpening {
    /// Fence character: backtick `` ` `` or tilde `~`.
    pub ch: char,
    /// Length of the opening fence run.
    pub len: usize,
    /// Optional language / info string after the opening fence.
    pub language: Option<SharedString>,
}
