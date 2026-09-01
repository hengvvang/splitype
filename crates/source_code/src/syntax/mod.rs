//! Local syntax helpers: bracket matching and indentation guides.
//!
//! Shared highlighting services come from the `syntax_highlighter` crate.

pub mod bracket;
pub mod indent_guides;

pub use bracket::find_matching_bracket;
pub use indent_guides::compute_indent_guide_columns;
