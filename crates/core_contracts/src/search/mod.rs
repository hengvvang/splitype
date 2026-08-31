//! Search and replace query execution, state models, and UI overlay.

pub mod host;
pub mod input_element;
pub mod query;
pub mod state;
pub mod ui;

#[cfg(test)]
mod query_tests;

pub use host::{SearchHost, SearchIme, SearchInputSnapshot, SearchStateView};
pub use input_element::SearchInputElement;
pub use query::{RawMatch, SearchQuery, compute_preserve_case_replacement};
pub use state::{
    SearchActiveField, SearchMatch, SearchPanelState, SearchScope, SearchTextInput,
    ceil_char_boundary, floor_char_boundary,
};
pub use ui::render_search_panel_overlay;
