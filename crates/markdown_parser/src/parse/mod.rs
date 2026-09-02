//! Markdown-to-block-tree parsing.
//!
//! This module owns the parsing contract types — [`BlockKind`] (the syntax
//! elements the parser recognizes), [`BlockData`] (the parser's output
//! container), [`BlockId`], and [`CodeFenceOpening`] (fence recognition
//! metadata) — together with the pure parser implementation ([`pipeline`]
//! and its helpers). No GPUI context or entity dependencies: everything
//! operates on the plain data types in [`crate::block`] and
//! [`crate::inline`].

pub mod code_and_text;
pub mod data;
pub mod fence;
pub mod footnotes;
pub mod helpers;
pub mod id;
pub mod indent;
pub mod kind;
pub mod lists;
pub mod pipeline;
pub mod quotes;

pub use data::BlockData;
pub use fence::{safe_code_fence, safe_code_fence_with_info};
pub use id::BlockId;
pub use kind::BlockKind;
pub use pipeline::{
    ParseMode, parse_document_with_mode, parse_preview_document, parse_wysiwyg_document,
};
