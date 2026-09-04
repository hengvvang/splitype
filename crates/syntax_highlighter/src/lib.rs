//! Unified syntax parsing, tree-sitter highlighting, and embedded graphics
//! (LaTeX math, Mermaid diagrams, markup helpers).

pub mod engine;
pub mod graphics;
pub mod highlight;
pub mod language;

pub use engine::HighlightMap;
pub use graphics::*;
pub use highlight::*;
pub use language::*;
