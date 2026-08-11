//! Block-level model: kinds, data, and Markdown syntax detection.

pub mod callout;
pub mod data;
pub mod fence;
pub mod id;
pub mod kind;

pub use callout::CalloutKind;
pub use data::BlockData;
pub use id::BlockId;
pub use kind::BlockKind;
