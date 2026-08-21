//! Pure Markdown domain model — data types, syntax parsers, and tree parsing.
//!
//! This layer has no GPUI entity or window dependencies. It may use GPUI's
//! plain data types (e.g. `SharedString`) but never `App`, `Window`, or
//! entity handles. Everything here is testable without a runtime.
//!
//! Layout: `parse/` holds the parsing contract types ([`parse::BlockKind`],
//! [`parse::BlockData`]) and the parser; `block/` holds the block-level
//! content models (table, HTML, math, Mermaid, image, references) and
//! `inline/` the inline text model — mirroring the CommonMark block-level /
//! inline-level split.

pub mod block;
pub mod extension;
pub mod inline;
pub mod parse;
pub mod tree;
