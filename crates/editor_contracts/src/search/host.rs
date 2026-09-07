use gpui::{App, Window};

pub trait SearchHost: Send + Sync + 'static {
    fn toggle_show_replace(&self, cx: &mut App);
    fn toggle_match_case(&self, cx: &mut App);
    fn toggle_whole_word(&self, cx: &mut App);
    fn toggle_use_regex(&self, cx: &mut App);
    fn toggle_preserve_case(&self, cx: &mut App);
    fn toggle_scope(&self, cx: &mut App);
    fn focus_query(&self, window: &mut Window, cx: &mut App);
    fn focus_replace(&self, window: &mut Window, cx: &mut App);
    fn set_query(&self, query: String, cx: &mut App);
    fn set_replace(&self, replace: String, cx: &mut App);
    fn close(&self, cx: &mut App);
    fn history_prev(&self, cx: &mut App);
    fn history_next(&self, cx: &mut App);
    fn prev_match(&self, window: &mut Window, cx: &mut App);
    fn next_match(&self, window: &mut Window, cx: &mut App);
    fn activate_match(&self, index: usize, window: &mut Window, cx: &mut App);
    fn replace_current(&self, window: &mut Window, cx: &mut App);
    fn replace_all(&self, cx: &mut App);
    fn toggle_match_expanded(&self, index: usize, cx: &mut App);
    fn collapse_results(&self, cx: &mut App);
}
