//! editor_core — the editor domain's resource and container layer.
//!
//! Home of the `Editor` aggregate root and everything that manages document
//! resources, session/file state, tab lifecycles, and pane layout hosting.

pub mod chrome;
pub mod commands;
pub mod document;
pub mod engine;
pub mod geometry;
pub mod history;
pub mod input;
pub mod navigation;
pub mod outline;
pub mod panes;
pub mod search;

pub use commands::actions;
pub use commands::registry as command_registry;
pub use engine::{Editor, EditorSession};

pub use navigation::{
    NavigationExecutionPlan, NavigationIntent, NavigationMode, NavigationTarget,
};
