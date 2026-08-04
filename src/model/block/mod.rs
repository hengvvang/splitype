//! Block-level model: kinds, records, and Markdown syntax detection.

pub mod callout;
pub mod fence;
pub mod id;
pub mod kind;
pub mod record;

pub use callout::CalloutKind;
pub use id::BlockId;
pub use kind::BlockKind;
pub use record::BlockData;
