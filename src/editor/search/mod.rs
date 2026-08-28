//! In-editor search and replace subsystem.

pub mod engine;
pub mod ime;
pub mod input_element;
pub mod query;
pub mod state;
pub mod ui;

pub use query::{RawMatch, SearchQuery};
pub use state::{SearchActiveField, SearchMatch, SearchPanelState, SearchScope, SearchTextInput};
