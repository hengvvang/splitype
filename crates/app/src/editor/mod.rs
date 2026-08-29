//! Document editing runtime — engine, document tree, projection, history, input, commands, panes, and chrome.
//!
//! The WYSIWYG world (block tree, projection, delta history, text
//! layout/measure, native-table grid data) now lives in the
//! `editor_wysiwyg` crate; this module keeps the editor coordination
//! layer until the `Editor` entity converges there.

pub mod chrome;
pub mod commands;
pub mod engine;
pub mod geometry;
pub mod history;
pub mod input;
pub mod navigation;
pub mod panes;
pub mod search;

pub use commands::actions;
pub use commands::keybindings;
pub use commands::registry as command_registry;
pub use editor_wysiwyg::document::Block;
pub use engine::{Editor, EditorSession};

pub use navigation::{
    NavigationExecutionPlan, NavigationIntent, NavigationMode, NavigationTarget,
};


