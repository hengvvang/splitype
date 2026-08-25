//! Editor actions, commands registry, keybindings, and menu integrations.

pub mod actions;
pub mod keybindings;
pub(crate) mod menu_actions;
pub mod registry;

pub use actions::*;
