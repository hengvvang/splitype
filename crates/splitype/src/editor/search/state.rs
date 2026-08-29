//! Search and replace state models and text input buffers.

use std::ops::Range;
use std::path::PathBuf;
use gpui::{Bounds, EntityId, FocusHandle, Pixels};

use markdown::parse::BlockId;

#[inline]
pub fn floor_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        s.len()
    } else {
        let mut i = index;
        while i > 0 && !s.is_char_boundary(i) {
            i -= 1;
        }
        i
    }
}

#[inline]
pub fn ceil_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        s.len()
    } else {
        let mut i = index;
        while i < s.len() && !s.is_char_boundary(i) {
            i += 1;
        }
        i
    }
}

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

/// Single line text input buffer with cursor, selection, IME marked range, and editing operations.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct SearchTextInput {
    pub text: String,
    pub selection: Range<usize>,
    pub reversed: bool,
    pub marked_range: Option<Range<usize>>,
    pub last_bounds: Option<Bounds<Pixels>>,
    pub history: Vec<String>,
    pub history_index: Option<usize>,
}

impl SearchTextInput {
    pub fn new(initial: impl Into<String>) -> Self {
        let text = initial.into();
        let len = text.len();
        Self {
            text,
            selection: len..len,
            reversed: false,
            marked_range: None,
            last_bounds: None,
            history: Vec::new(),
            history_index: None,
        }
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
        let end = self.text.len();
        self.selection = end..end;
        self.reversed = false;
        self.marked_range = None;
    }

    pub fn selection_range(&self) -> Range<usize> {
        let (start, end) = if self.reversed {
            (self.selection.end, self.selection.start)
        } else {
            (self.selection.start, self.selection.end)
        };
        let s = floor_char_boundary(&self.text, start);
        let e = ceil_char_boundary(&self.text, end.max(s));
        s..e
    }

    pub fn cursor(&self) -> usize {
        let raw = if self.reversed {
            self.selection.start
        } else {
            self.selection.end
        };
        floor_char_boundary(&self.text, raw)
    }

    pub fn replace_range(&mut self, range: Range<usize>, new_text: &str) {
        let start = floor_char_boundary(&self.text, range.start);
        let end = ceil_char_boundary(&self.text, range.end.max(start));
        self.text.replace_range(start..end, new_text);
        let cursor = start + new_text.len();
        self.selection = cursor..cursor;
        self.reversed = false;
        self.marked_range = None;
    }

    pub fn insert_str(&mut self, s: &str) {
        let range = self.selection_range();
        self.replace_range(range, s);
    }

    pub fn delete_backward(&mut self) {
        let range = self.selection_range();
        if !range.is_empty() {
            self.replace_range(range, "");
            return;
        }
        let cursor = self.cursor();
        if cursor > 0 && !self.text.is_empty() {
            let prev = self.text[..cursor]
                .char_indices()
                .last()
                .map(|(idx, _)| idx)
                .unwrap_or(0);
            self.replace_range(prev..cursor, "");
        }
    }

    pub fn delete_forward(&mut self) {
        let range = self.selection_range();
        if !range.is_empty() {
            self.replace_range(range, "");
            return;
        }
        let cursor = self.cursor();
        if cursor < self.text.len() {
            let next = self.text[cursor..]
                .char_indices()
                .nth(1)
                .map(|(idx, _)| cursor + idx)
                .unwrap_or(self.text.len());
            self.replace_range(cursor..next, "");
        }
    }

    pub fn move_left(&mut self, select: bool) {
        let cursor = self.cursor();
        if cursor > 0 {
            let prev = self.text[..cursor]
                .char_indices()
                .last()
                .map(|(idx, _)| idx)
                .unwrap_or(0);
            if select {
                let anchor = floor_char_boundary(
                    &self.text,
                    if self.reversed {
                        self.selection.end
                    } else {
                        self.selection.start
                    },
                );
                self.selection = prev..anchor;
                self.reversed = prev < anchor;
            } else {
                self.selection = prev..prev;
                self.reversed = false;
            }
        } else if !select {
            self.selection = 0..0;
            self.reversed = false;
        }
    }

    pub fn move_right(&mut self, select: bool) {
        let cursor = self.cursor();
        if cursor < self.text.len() {
            let next = self.text[cursor..]
                .char_indices()
                .nth(1)
                .map(|(idx, _)| cursor + idx)
                .unwrap_or(self.text.len());
            if select {
                let anchor = floor_char_boundary(
                    &self.text,
                    if self.reversed {
                        self.selection.end
                    } else {
                        self.selection.start
                    },
                );
                self.selection = anchor..next;
                self.reversed = false;
            } else {
                self.selection = next..next;
                self.reversed = false;
            }
        } else if !select {
            let end = self.text.len();
            self.selection = end..end;
            self.reversed = false;
        }
    }

    pub fn move_home(&mut self, select: bool) {
        if select {
            let anchor = floor_char_boundary(
                &self.text,
                if self.reversed {
                    self.selection.end
                } else {
                    self.selection.start
                },
            );
            self.selection = 0..anchor;
            self.reversed = true;
        } else {
            self.selection = 0..0;
            self.reversed = false;
        }
    }

    pub fn move_end(&mut self, select: bool) {
        let end = self.text.len();
        if select {
            let anchor = floor_char_boundary(
                &self.text,
                if self.reversed {
                    self.selection.end
                } else {
                    self.selection.start
                },
            );
            self.selection = anchor..end;
            self.reversed = false;
        } else {
            self.selection = end..end;
            self.reversed = false;
        }
    }

    pub fn select_all(&mut self) {
        self.selection = 0..self.text.len();
        self.reversed = false;
    }

    pub fn push_history(&mut self, text: &str) {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }
        if self.history.last().map(|s| s.as_str()) != Some(trimmed) {
            self.history.push(trimmed.to_string());
        }
        self.history_index = None;
    }

    pub fn history_prev(&mut self) -> bool {
        if self.history.is_empty() {
            return false;
        }
        let next_idx = match self.history_index {
            Some(idx) => {
                if idx == 0 {
                    0
                } else {
                    idx - 1
                }
            }
            None => self.history.len().saturating_sub(1),
        };
        self.history_index = Some(next_idx);
        if let Some(entry) = self.history.get(next_idx).cloned() {
            self.set_text(entry);
            self.select_all();
            true
        } else {
            false
        }
    }

    pub fn history_next(&mut self) -> bool {
        if self.history.is_empty() {
            return false;
        }
        match self.history_index {
            Some(idx) => {
                if idx + 1 < self.history.len() {
                    let next_idx = idx + 1;
                    self.history_index = Some(next_idx);
                    if let Some(entry) = self.history.get(next_idx).cloned() {
                        self.set_text(entry);
                        self.select_all();
                        return true;
                    }
                } else {
                    self.history_index = None;
                    self.set_text(String::new());
                    return true;
                }
            }
            None => {}
        }
        false
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
    /// Column number (1-indexed, in Unicode characters).
    pub column_number: usize,
    /// Character / byte range of the matched text within the line or block.
    pub byte_range: Range<usize>,
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
    /// Search query input buffer.
    pub search_input: SearchTextInput,
    /// Replacement string input buffer.
    pub replace_input: SearchTextInput,
    /// Active input field.
    pub active_field: SearchActiveField,
    /// Case-sensitive matching flag (Aa).
    pub match_case: bool,
    /// Whole-word matching flag (ab).
    pub whole_word: bool,
    /// Regular expression matching flag (.*).
    pub use_regex: bool,
    /// Preserve case flag in replace (AB).
    pub preserve_case: bool,
    /// Search scope: current tab file or whole worktree.
    pub scope: SearchScope,
    /// List of matched entries.
    pub matches: Vec<SearchMatch>,
    /// Current active match index.
    pub active_match_index: Option<usize>,
    /// Whether the search results drawer list is expanded below the inputs.
    pub results_expanded: bool,
    /// Indices of match items that are expanded to show full details.
    pub expanded_match_indices: std::collections::HashSet<usize>,
    /// Focus handle for the search query text input.
    pub search_focus_handle: FocusHandle,
    /// Focus handle for the replace text input.
    pub replace_focus_handle: FocusHandle,
    /// Monotonic generation counter to invalidate outdated asynchronous searches.
    pub search_generation: u64,
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
            preserve_case: false,
            scope: SearchScope::CurrentTab,
            matches: Vec::new(),
            active_match_index: None,
            results_expanded: true,
            expanded_match_indices: std::collections::HashSet::new(),
            search_focus_handle: cx.focus_handle(),
            replace_focus_handle: cx.focus_handle(),
            search_generation: 0,
        }
    }

    /// Toggles expanded details state for a specific search match item.
    pub fn toggle_match_expanded(&mut self, idx: usize) {
        if self.expanded_match_indices.contains(&idx) {
            self.expanded_match_indices.remove(&idx);
        } else {
            self.expanded_match_indices.insert(idx);
        }
    }

    /// Whether a specific search match item is expanded to show details.
    pub fn is_match_expanded(&self, idx: usize) -> bool {
        self.expanded_match_indices.contains(&idx)
    }

    pub fn query(&self) -> &str {
        &self.search_input.text
    }

    pub fn replace_query(&self) -> &str {
        &self.replace_input.text
    }

    /// Returns the current match count formatted as "current of total" or empty if none.
    pub fn match_status_label(&self) -> String {
        if self.matches.is_empty() {
            if self.query().is_empty() {
                String::new()
            } else {
                "No results".to_string()
            }
        } else {
            let active = self.active_match_index.map(|i| i + 1).unwrap_or(1);
            format!("{}/{}", active, self.matches.len())
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
