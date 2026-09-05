//! editor — the editor domain's resource and container layer.
//!
//! Home of the `Editor` aggregate root and everything that manages document
//! buffers, tab view lifecycles, and pane layout hosting.

pub mod actions;
pub mod assets;
pub mod document;
pub mod editor;
pub mod input;
pub mod layout;
pub mod outline;
pub mod plugin;
pub mod search;
pub mod session;
pub mod settings;
pub mod view;

pub use plugin::*;
