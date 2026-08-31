//! Panel framework of the window shell.
//!
//! The trait contracts and domain types live in `core_contracts::panel`; this
//! module hosts the window-side implementations: the registry and the
//! missing-plugin placeholder.

pub mod missing;
pub mod registry;

pub use missing::MissingPanelView;
pub use registry::*;
