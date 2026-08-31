//! Search and replace contracts: query execution, state models, and the
//! host/IME seams. Presentation lives in the `ui` crate
//! (`ui::render_search_panel_overlay`).

pub mod host;
pub mod query;
pub mod state;

#[cfg(test)]
mod query_tests;

pub use host::{SearchHost, SearchIme, SearchInputSnapshot, SearchStateView};
pub use query::{RawMatch, SearchQuery, compute_preserve_case_replacement};
pub use state::{
    SearchActiveField, SearchMatch, SearchPanelState, SearchScope, SearchTextInput,
    ceil_char_boundary, floor_char_boundary,
};
