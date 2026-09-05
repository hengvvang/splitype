//! Reusable UI components — atomic and molecular design system building blocks.
//!
//! Components here consume only `theme`, `config`, `splitter`, and gpui,
//! providing pure styling and interaction primitives with zero domain logic.

pub mod split;
pub mod ui_components;

// Re-export modules and types for backwards compatibility
pub use ui_components::{
    button, dialog, empty_state, menu_item, popover, select, stepper, switch,
};
pub use ui_components::*;

