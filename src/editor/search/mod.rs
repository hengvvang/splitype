//! Editor in-buffer and workspace search and replace subsystem.

pub mod engine;
pub mod state;
pub mod ui;

pub use state::{SearchMatch, SearchPanelState, SearchScope};
