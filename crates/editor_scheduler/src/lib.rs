//! editor_scheduler — the editor family's coordination layer.
//!
//! Home of the `Editor` aggregate root and everything that schedules work
//! across the editor family: session and file flows (`engine/`), command
//! dispatch (`commands/`), input routing (`input/`, `navigation/`,
//! `history/`), the mode coordination shells (`panes/`), the document
//! view shell (`document/`), the outline HUD coordination (`outline/`),
//! the search engine (`search/`) and the editor chrome (`chrome/`).
//!
//! No mode implementation lives here — the mode crates
//! (`editor_wysiwyg`, `editor_source_code`, `editor_preview`) own their
//! presentation and input; this layer routes to them and synchronizes
//! between them through the contract seams in `editor_model`. Window-
//! shell concerns (recent-files menu, explorer sync, keybinding
//! installation) are reached through `EditorHost` / stay in the app
//! composition root.

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
pub use editor_wysiwyg::document::Block;
pub use engine::{Editor, EditorSession};

pub use navigation::{
    NavigationExecutionPlan, NavigationIntent, NavigationMode, NavigationTarget,
};
