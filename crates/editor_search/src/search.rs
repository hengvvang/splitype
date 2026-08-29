//! editor_search — the search & replace panel and its pure matching
//! engine.
//!
//! `query` holds the precompiled, context-free matcher ([`SearchQuery`],
//! [`RawMatch`]); `state` holds the panel UI state ([`SearchPanelState`],
//! [`SearchMatch`], [`SearchTextInput`]) and the Unicode-safe character
//! boundary helpers; `ui`/`input_element` own the panel presentation and
//! input field; `host` declares the seams back to the coordinating crate
//! ([`SearchHost`], [`SearchStateView`], [`SearchIme`]).
//!
//! The cross-mode match contract is pure data: modes receive
//! `(Range<usize>, bool)` ranges through `editor::Pane::set_search_matches`
//! (F7).

pub mod query;
pub mod state;
mod host;
mod input_element;
mod ui;

pub use host::{SearchHost, SearchIme, SearchInputSnapshot, SearchStateView};
pub use query::{RawMatch, SearchQuery};
pub use state::{
    SearchActiveField, SearchMatch, SearchPanelState, SearchScope, SearchTextInput,
};
pub use ui::render_search_panel_overlay;

#[cfg(test)]
mod query_tests;
