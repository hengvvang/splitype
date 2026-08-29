//! editor_search — the search & replace panel and its pure matching
//! engine.
//!
//! `query` holds the precompiled, context-free matcher ([`SearchQuery`],
//! [`RawMatch`]); `state` holds the panel UI state ([`SearchPanelState`],
//! [`SearchMatch`], [`SearchTextInput`]) and the Unicode-safe character
//! boundary helpers. The engine and panel glue that talk to the `Editor`
//! entity stay in the coordinating crate until the editor converges.
//!
//! The cross-mode match contract is pure data: modes receive
//! `(Range<usize>, bool)` ranges through `editor::Pane::set_search_matches`
//! (F7).

pub mod query;
pub mod state;

pub use query::{RawMatch, SearchQuery};
pub use state::{
    SearchActiveField, SearchMatch, SearchPanelState, SearchScope, SearchTextInput,
};

#[cfg(test)]
mod query_tests;
