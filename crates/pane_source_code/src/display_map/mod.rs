//! Layered display transformation pipeline (Zed DisplayMap architecture).

pub mod display_point;
pub mod fold_map;
pub mod snapshot;
pub mod tab_map;
pub mod wrap_map;

pub use display_point::DisplayPoint;
pub use fold_map::{FoldMap, FoldRange};
pub use snapshot::DisplaySnapshot;
pub use tab_map::TabMap;
pub use wrap_map::WrapMap;
