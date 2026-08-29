//! Search panel seams: the host proxy for coordination-layer actions and
//! the snapshot/IME interfaces for the input element.
//!
//! The panel presentation lives in this crate; whatever needs the editor
//! entity (query execution, jumping, replacing, focus routing) goes
//! through [`SearchHost`], which the coordinating crate implements by
//! re-entering its entity.

use std::ops::Range;

use gpui::{App, Bounds, FocusHandle, KeyDownEvent, Pixels, Window};

use crate::state::SearchActiveField;

/// Coordination-layer actions the search panel may request while
/// rendering or receiving input.
pub trait SearchHost: Send + Sync + 'static {
    /// Run the search with the current query and scope.
    fn execute_search(&self, cx: &mut App);

    /// Repaint the editor.
    fn notify(&self, cx: &mut App);

    /// Toggle the replace input row.
    fn toggle_show_replace(&self, cx: &mut App);

    /// Toggle match-case and re-run the search.
    fn toggle_match_case(&self, cx: &mut App);

    /// Toggle whole-word matching and re-run the search.
    fn toggle_whole_word(&self, cx: &mut App);

    /// Toggle regex matching and re-run the search.
    fn toggle_use_regex(&self, cx: &mut App);

    /// Toggle preserve-case on replace and repaint.
    fn toggle_preserve_case(&self, cx: &mut App);

    /// Toggle the search scope (current tab / worktree) and re-run.
    fn toggle_scope(&self, cx: &mut App);

    /// Focus the query input.
    fn focus_query(&self, window: &mut Window, cx: &mut App);

    /// Focus the replace input.
    fn focus_replace(&self, window: &mut Window, cx: &mut App);

    /// Route a key-down inside the search inputs (escape/tab/enter/...).
    fn handle_key_down(&self, event: &KeyDownEvent, window: &mut Window, cx: &mut App);

    /// Move to and jump to the previous match.
    fn prev_match(&self, window: &mut Window, cx: &mut App);

    /// Move to and jump to the next match.
    fn next_match(&self, window: &mut Window, cx: &mut App);

    /// Activate match `index` and jump to it.
    fn activate_match(&self, index: usize, window: &mut Window, cx: &mut App);

    /// Replace the active match.
    fn replace_current(&self, window: &mut Window, cx: &mut App);

    /// Replace all matches.
    fn replace_all(&self, cx: &mut App);

    /// Toggle the match-details drawer of match `index`.
    fn toggle_match_expanded(&self, index: usize, cx: &mut App);

    /// Collapse the results drawer.
    fn collapse_results(&self, cx: &mut App);

    /// Record an input field's rendered bounds (IME popup positioning).
    fn set_input_last_bounds(
        &self,
        field: SearchActiveField,
        bounds: Bounds<Pixels>,
        cx: &mut App,
    );
}

/// Owned snapshot of one search input field, as the input element needs
/// it to lay out and paint.
#[derive(Default)]
pub struct SearchInputSnapshot {
    pub text: String,
    pub marked_range: Option<Range<usize>>,
    pub selection_range: Range<usize>,
    pub cursor_offset: usize,
    pub focus_handle: Option<FocusHandle>,
}

/// Read-only view of the search panel state for the input element.
pub trait SearchStateView: Send + Sync + 'static {
    /// Snapshot of the field's state.
    fn snapshot(&self, field: SearchActiveField, cx: &App) -> SearchInputSnapshot;
}

/// IME registration for a search input field. GPUI requires the platform
/// input handler to bind to a concrete entity, so the coordinating crate
/// implements this by re-entering its entity.
pub trait SearchIme: Send + Sync + 'static {
    /// Register the platform input handler for the field at `bounds`.
    fn handle_input(
        &self,
        field: SearchActiveField,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut App,
    );
}
