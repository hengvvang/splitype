//! Buffer coordinate primitives and line indexing.

pub mod anchor;
pub mod line_map;
pub mod point;

pub use anchor::{Anchor, Bias};
pub use line_map::LineMap;
pub use point::BufferPoint;
