//! Pure Markdown-to-block-tree parser.
//!
//! Converts raw Markdown text into a flat list of [`BlockData`] with
//! parent-child relationships expressed through [`BlockId`] references.
//! This module has no GPUI context or entity dependencies — it operates
//! entirely on the plain data types in `crate::markdown::block` and `crate::markdown::inline`.

pub(crate) mod code_and_text;
pub(crate) mod footnotes;
pub(crate) mod helpers;
pub(crate) mod lists;
pub(crate) mod pipeline;
pub(crate) mod quotes;

pub use pipeline::{
    ParseMode, build_preview_blocks_from_lines, build_wysiwyg_blocks_from_lines,
    parse_document_with_mode, parse_preview_document, parse_wysiwyg_document,
};
