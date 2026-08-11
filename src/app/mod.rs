//! Application shell — CLI parsing, bootstrap, menus, actions, windows,
//! and assets.

pub mod actions;
pub mod assets;
pub mod bootstrap;
pub mod cli;
pub(crate) mod cli_install;
pub mod menus;
pub(crate) mod shell;
pub mod window;
pub(crate) mod window_chrome;
pub(crate) mod window_dialogs;
pub(crate) mod window_layout;
pub mod window_panels;
