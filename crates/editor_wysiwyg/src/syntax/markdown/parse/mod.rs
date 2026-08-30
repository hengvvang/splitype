//! Markdown-to-block-tree parsing.
//!
//! The parse domain owns the parsing contract types — [`BlockKind`] (the
//! syntax elements the parser recognizes), [`BlockData`] (the parser's
//! output container), [`BlockId`], and [`CodeFenceOpening`] (fence
//! recognition metadata) — together with the parser execution itself
//! (`parser`, `indent`).

pub mod data;
pub mod fence;
pub mod id;
pub mod indent;
pub mod kind;
pub mod parser;

pub use data::BlockData;
pub use fence::{safe_code_fence, safe_code_fence_with_info};
pub use id::BlockId;
pub use kind::BlockKind;
pub use parser::ParseMode;
