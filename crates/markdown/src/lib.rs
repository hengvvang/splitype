//! Pure Markdown domain model — data types, syntax parsers, and AST.
//!
//! This layer has no GPUI entity or window dependencies.

pub use primitives::*;
pub use sum_tree;

pub mod block;
pub mod inline;
pub mod parse;

#[cfg(test)]
mod tests;
