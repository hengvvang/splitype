//! Syntax highlighting, bracket matching, and indentation guides.

pub mod bracket;
pub mod highlight;
pub mod indent_guides;

pub use bracket::find_matching_bracket;
pub use highlight::{
    CodeHighlightResult, CodeHighlightSpan, CodeLanguageKey, highlight_code_block,
    prewarm_code_highlight_registry,
};
pub use indent_guides::compute_indent_guide_columns;
