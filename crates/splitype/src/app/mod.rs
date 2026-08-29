//! Application shell — CLI parsing, bootstrap, menus, actions, windows,
//! and assets.

pub mod actions;
pub mod assets;
pub mod bootstrap;
pub mod cli;
pub mod menus;
pub(crate) mod shell;
pub mod window;

pub use workspace::WindowPanelKind;
