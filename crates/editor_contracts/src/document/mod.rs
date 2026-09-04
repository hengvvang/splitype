//! Document-level contracts: the snapshot panes consume, the tab-kind
//! vocabulary, and the host service the window shell implements.

pub mod host;
pub mod snapshot;
pub mod tab_kind;

pub use host::DocumentHost;
pub use snapshot::{DocumentId, DocumentSnapshot};
pub use tab_kind::TabKind;
