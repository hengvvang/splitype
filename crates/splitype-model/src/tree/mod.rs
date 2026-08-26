//! Tree data structures for interval indexing and summaries.

pub mod sum_tree;
pub mod types;

pub use sum_tree::{BlockSummary, Item, Summary, SumTree};
pub use types::{PixelHeight, PixelY, ScrollAnchor, SourceByteOffset, VisualCharOffset};
