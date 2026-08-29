//! In-editor search and replace subsystem.
//!
//! The pure engine (`SearchQuery`/`RawMatch`) and the panel state types
//! live in the `editor_search` crate (re-exported below for the
//! migration in progress); the editor-facing glue (engine, IME, input
//! element, UI) stays here until the `Editor` entity converges.

pub mod engine;
pub mod ime;
pub mod input_element;
pub mod ui;

pub use editor_search::{RawMatch, SearchQuery};
pub use editor_search::{
    SearchActiveField, SearchMatch, SearchPanelState, SearchScope, SearchTextInput,
};
