//! Search and replace state models.

use std::path::PathBuf;
use gpui::{EntityId, FocusHandle};

use crate::model::parse::BlockId;

/// Active input field inside search panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SearchActiveField {
    #[default]
    Query,
    Replace,
}

/// Target scope for text search.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SearchScope {
    #[default]
    CurrentTab,
    Worktree,
}

/// Single line text input model with cursor, selection, and editing operations.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct SearchTextInput {
    pub text: String,
    pub cursor: usize,
    pub selected_range: std::ops::Range<usize>,
}

impl SearchTextInput {
    pub fn new(initial: impl Into<String>) -> Self {
        let text = initial.into();
        let len = text.len();
        Self {
            text,
            cursor: len,
            selected_range: len..len,
        }
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.cursor = self.text.len();
        self.selected_range = self.cursor..self.cursor;
    }

    pub fn insert_str(&mut self, s: &str) {
        let range = if !self.selected_range.is_empty() {
            self.selected_range.clone()
        } else {
            self.cursor..self.cursor
        };
        let start = range.start.min(self.text.len());
        let end = range.end.min(self.text.len());
        let mut new_text = String::with_capacity(self.text.len() + s.len());
        new_text.push_str(&self.text[..start]);
        new_text.push_str(s);
        new_text.push_str(&self.text[end..]);
        self.text = new_text;
        self.cursor = start + s.len();
        self.selected_range = self.cursor..self.cursor;
    }

    pub fn delete_backward(&mut self) {
        if !self.selected_range.is_empty() {
            self.insert_str("");
            return;
        }
        if self.cursor > 0 && !self.text.is_empty() {
            let prev = self.text[..self.cursor]
                .char_indices()
                .last()
                .map(|(idx, _)| idx)
                .unwrap_or(0);
            self.selected_range = prev..self.cursor;
            self.insert_str("");
        }
    }

    pub fn delete_forward(&mut self) {
        if !self.selected_range.is_empty() {
            self.insert_str("");
            return;
        }
        if self.cursor < self.text.len() {
            let next = self.text[self.cursor..]
                .char_indices()
                .nth(1)
                .map(|(idx, _)| self.cursor + idx)
                .unwrap_or(self.text.len());
            self.selected_range = self.cursor..next;
            self.insert_str("");
        }
    }

    pub fn move_left(&mut self, select: bool) {
        if self.cursor > 0 {
            let prev = self.text[..self.cursor]
                .char_indices()
                .last()
                .map(|(idx, _)| idx)
                .unwrap_or(0);
            if select {
                let anchor = if self.selected_range.start == self.cursor {
                    self.selected_range.end
                } else {
                    self.selected_range.start
                };
                self.selected_range = prev.min(anchor)..prev.max(anchor);
            } else {
                self.selected_range = prev..prev;
            }
            self.cursor = prev;
        } else if !select {
            self.selected_range = 0..0;
        }
    }

    pub fn move_right(&mut self, select: bool) {
        if self.cursor < self.text.len() {
            let next = self.text[self.cursor..]
                .char_indices()
                .nth(1)
                .map(|(idx, _)| self.cursor + idx)
                .unwrap_or(self.text.len());
            if select {
                let anchor = if self.selected_range.end == self.cursor {
                    self.selected_range.start
                } else {
                    self.selected_range.end
                };
                self.selected_range = next.min(anchor)..next.max(anchor);
            } else {
                self.selected_range = next..next;
            }
            self.cursor = next;
        } else if !select {
            self.selected_range = self.text.len()..self.text.len();
        }
    }

    pub fn select_all(&mut self) {
        self.cursor = self.text.len();
        self.selected_range = 0..self.text.len();
    }
}

/// One search match item within a document or worktree file.
#[derive(Clone, Debug, PartialEq)]
pub struct SearchMatch {
    /// Source file path (None for untitled temporary buffer).
    pub file_path: Option<PathBuf>,
    /// Display name of the file.
    pub file_name: String,
    /// Logical block identifier if match is in the active document.
    pub block_id: Option<BlockId>,
    /// Block entity ID in the active document.
    pub entity_id: Option<EntityId>,
    /// Line number (1-indexed).
    pub line_number: usize,
    /// Character / byte range of the matched text within the line or block.
    pub byte_range: std::ops::Range<usize>,
    /// Text before the match for context.
    pub preview_prefix: String,
    /// Exact matched substring.
    pub preview_match: String,
    /// Text after the match for context.
    pub preview_suffix: String,
}

/// Search panel UI and query state.
pub struct SearchPanelState {
    /// Whether the search & replace floating overlay is open.
    pub visible: bool,
    /// Whether the replace input row is expanded.
    pub show_replace: bool,
    /// Search query input state.
    pub search_input: SearchTextInput,
    /// Replacement string input state.
    pub replace_input: SearchTextInput,
    /// Active input field.
    pub active_field: SearchActiveField,
    /// Case-sensitive matching flag.
    pub match_case: bool,
    /// Whole-word matching flag.
    pub whole_word: bool,
    /// Regular expression matching flag.
    pub use_regex: bool,
    /// Search scope: current tab file or whole worktree.
    pub scope: SearchScope,
    /// List of matched entries.
    pub matches: Vec<SearchMatch>,
    /// Current active match index.
    pub active_match_index: Option<usize>,
    /// Whether the search results drawer list is expanded below the inputs.
    pub results_expanded: bool,
    /// Focus handle for the search query text input.
    pub search_focus_handle: FocusHandle,
    /// Focus handle for the replace text input.
    pub replace_focus_handle: FocusHandle,
}

impl SearchPanelState {
    /// Creates a new default search panel state.
    pub fn new(cx: &mut gpui::Context<crate::editor::engine::controller::Editor>) -> Self {
        Self {
            visible: false,
            show_replace: false,
            search_input: SearchTextInput::default(),
            replace_input: SearchTextInput::default(),
            active_field: SearchActiveField::Query,
            match_case: false,
            whole_word: false,
            use_regex: false,
            scope: SearchScope::CurrentTab,
            matches: Vec::new(),
            active_match_index: None,
            results_expanded: false,
            search_focus_handle: cx.focus_handle(),
            replace_focus_handle: cx.focus_handle(),
        }
    }

    pub fn query(&self) -> &str {
        &self.search_input.text
    }

    pub fn replace_query(&self) -> &str {
        &self.replace_input.text
    }

    /// Returns the current match count formatted as "current / total" or empty if none.
    pub fn match_status_label(&self) -> String {
        if self.matches.is_empty() {
            if self.query().is_empty() {
                String::new()
            } else {
                "No results".to_string()
            }
        } else {
            let active = self.active_match_index.map(|i| i + 1).unwrap_or(1);
            format!("{} of {}", active, self.matches.len())
        }
    }

    /// Selects the next match in the matches list, wrapping around.
    pub fn next_match(&mut self) -> Option<&SearchMatch> {
        if self.matches.is_empty() {
            self.active_match_index = None;
            return None;
        }
        let next_idx = match self.active_match_index {
            Some(curr) => (curr + 1) % self.matches.len(),
            None => 0,
        };
        self.active_match_index = Some(next_idx);
        self.matches.get(next_idx)
    }

    /// Selects the previous match in the matches list, wrapping around.
    pub fn prev_match(&mut self) -> Option<&SearchMatch> {
        if self.matches.is_empty() {
            self.active_match_index = None;
            return None;
        }
        let prev_idx = match self.active_match_index {
            Some(curr) => {
                if curr == 0 {
                    self.matches.len().saturating_sub(1)
                } else {
                    curr - 1
                }
            }
            None => 0,
        };
        self.active_match_index = Some(prev_idx);
        self.matches.get(prev_idx)
    }

    /// Gets the currently selected match.
    pub fn current_match(&self) -> Option<&SearchMatch> {
        self.active_match_index.and_then(|idx| self.matches.get(idx))
    }
}
