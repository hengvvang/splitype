//! Panel framework of the window shell.
//!
//! The trait contracts and domain types live in `core_contracts::panel` so
//! plugins can depend on them without pulling in the shell. This module
//! hosts the window-side registry implementation and re-exports the contract
//! types for convenience.

pub mod registry;

pub use core_contracts::{
    DocumentPanel, PanelCapabilities, PanelDescriptor, PanelHost, PanelId, PanelKind,
    PanelRenderContext, PanelView,
};
pub use registry::*;
