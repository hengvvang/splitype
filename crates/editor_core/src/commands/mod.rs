//! Editor actions, commands registry, keybindings, and menu integrations.

pub mod actions;
pub mod edit_command;
pub(crate) mod menu_actions;
pub mod registry;

pub use actions::*;
pub use edit_command::*;
