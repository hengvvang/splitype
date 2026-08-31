use std::ops::Range;
use gpui::{App, Bounds, FocusHandle, KeyDownEvent, Pixels, Window};
use super::state::SearchActiveField;

pub trait SearchHost: Send + Sync + 'static {
    fn execute_search(&self, cx: &mut App);
    fn notify(&self, cx: &mut App);
    fn toggle_show_replace(&self, cx: &mut App);
    fn toggle_match_case(&self, cx: &mut App);
    fn toggle_whole_word(&self, cx: &mut App);
    fn toggle_use_regex(&self, cx: &mut App);
    fn toggle_preserve_case(&self, cx: &mut App);
    fn toggle_scope(&self, cx: &mut App);
    fn focus_query(&self, window: &mut Window, cx: &mut App);
    fn focus_replace(&self, window: &mut Window, cx: &mut App);
    fn handle_key_down(&self, event: &KeyDownEvent, window: &mut Window, cx: &mut App);
    fn prev_match(&self, window: &mut Window, cx: &mut App);
    fn next_match(&self, window: &mut Window, cx: &mut App);
    fn activate_match(&self, index: usize, window: &mut Window, cx: &mut App);
    fn replace_current(&self, window: &mut Window, cx: &mut App);
    fn replace_all(&self, cx: &mut App);
    fn toggle_match_expanded(&self, index: usize, cx: &mut App);
    fn collapse_results(&self, cx: &mut App);
    fn set_input_last_bounds(
        &self,
        field: SearchActiveField,
        bounds: Bounds<Pixels>,
        cx: &mut App,
    );
}

#[derive(Default)]
pub struct SearchInputSnapshot {
    pub text: String,
    pub marked_range: Option<Range<usize>>,
    pub selection_range: Range<usize>,
    pub cursor_offset: usize,
    pub focus_handle: Option<FocusHandle>,
}

pub trait SearchStateView: Send + Sync + 'static {
    fn snapshot(&self, field: SearchActiveField, cx: &App) -> SearchInputSnapshot;
}

pub trait SearchIme: Send + Sync + 'static {
    fn handle_input(
        &self,
        field: SearchActiveField,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut App,
    );
}
