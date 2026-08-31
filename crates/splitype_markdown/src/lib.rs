//! Pure Markdown domain model — AST data structures, block and inline parsers, and table models.

pub mod block;
pub mod footnotes;
pub mod inline;
pub mod parse;
pub mod table;

pub use footnotes::*;
pub use parse::*;
