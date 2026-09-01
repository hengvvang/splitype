//! Panel contracts — the universal SPI for window-level panels.
//!
//! This module owns the domain vocabulary and trait contracts that any panel
//! plugin (built-in or third-party) programs against. The window shell crate
//! depends on these contracts and hosts the registry implementation; it must
//! never add plugin-specific types here.

pub mod descriptor;
pub mod id;
pub mod kind;
pub mod view;

pub use descriptor::PanelDescriptor;
pub use id::PanelId;
pub use kind::PanelKind;
pub use view::{PanelRenderContext, PanelView};
