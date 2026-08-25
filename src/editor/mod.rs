//! Document editing runtime — engine, document tree, projection, history, input, commands, panes, plugins, export, and chrome.

pub mod chrome;
pub mod commands;
pub(crate) mod corner_drag_preview;
pub mod document;
pub mod engine;
pub mod export;
pub mod file;
pub mod geometry;
pub mod history;
pub mod input;
pub mod panes;
pub mod plugins;
pub mod projection;

pub use commands::actions;
pub use commands::keybindings;
pub use commands::registry as command_registry;
pub use document::Block;
pub use document::protocol::BlockEvent;
pub use engine::{Editor, EditorSession};
pub(crate) use panes::PreviewState;

#[cfg(test)]
mod tests;
