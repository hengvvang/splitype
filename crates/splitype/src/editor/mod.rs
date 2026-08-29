//! Document editing runtime — engine, document tree, projection, history, input, commands, panes, and chrome.

pub mod chrome;
pub mod commands;
pub mod document;
pub mod engine;
pub mod geometry;
pub mod history;
pub mod input;
pub mod navigation;
pub mod panes;
pub mod projection;
pub mod search;

pub use export;
pub use syntax;
pub use latex;
pub use mermaid;

pub use commands::actions;
pub use commands::keybindings;
pub use commands::registry as command_registry;
pub use document::Block;
pub use engine::{Editor, EditorSession};

pub use navigation::{
    NavigationExecutionPlan, NavigationIntent, NavigationMode, NavigationTarget,
};


#[cfg(test)]
mod tests;
