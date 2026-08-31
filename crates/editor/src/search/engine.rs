//! Search and replace query execution engine.

use std::fs;
use std::path::{Path, PathBuf};

use gpui::{App, Context, KeyDownEvent, Window};

use crate::editor::Editor;
use core_contracts::SearchQuery;
use core_contracts::{SearchActiveField, SearchMatch, SearchScope};

impl Editor {
    /// Toggles visibility of the Search and Replace overlay panel.
    pub fn toggle_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search.visible = !self.search.visible;
        if self.search.visible {
            self.search.active_field = SearchActiveField::Query;
            window.focus(&self.search.search_focus_handle, cx);
            self.search.search_input.select_all();
            self.execute_search(cx);
        } else {
            self.clear_search_highlights_from_document(cx);
        }
        cx.notify();
    }

    /// Toggles the replace input row and opens search panel if closed.
    pub fn toggle_replace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.search.visible {
            self.search.visible = true;
            self.search.show_replace = true;
            self.search.active_field = SearchActiveField::Replace;
            window.focus(&self.search.replace_focus_handle, cx);
            self.search.replace_input.select_all();
            self.execute_search(cx);
        } else if !self.search.show_replace {
            self.search.show_replace = true;
            self.search.active_field = SearchActiveField::Replace;
            window.focus(&self.search.replace_focus_handle, cx);
            self.search.replace_input.select_all();
        } else {
            self.search.show_replace = false;
            self.search.active_field = SearchActiveField::Query;
            window.focus(&self.search.search_focus_handle, cx);
        }
        cx.notify();
    }

    /// Navigates to the next search match and centers it.
    pub fn find_next(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.search.visible {
            self.toggle_search(window, cx);
            return;
        }
        self.search.next_match();
        self.jump_to_active_search_match(window, cx);
        cx.notify();
    }

    /// Navigates to the previous search match and centers it.
    pub fn find_previous(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.search.visible {
            self.toggle_search(window, cx);
            return;
        }
        self.search.prev_match();
        self.jump_to_active_search_match(window, cx);
        cx.notify();
    }

    pub(crate) fn on_toggle_search(
        &mut self,
        _: &crate::actions::ToggleSearch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_search(window, cx);
    }

    pub(crate) fn on_toggle_replace(
        &mut self,
        _: &crate::actions::ToggleReplace,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_replace(window, cx);
    }

    pub(crate) fn on_find_next(
        &mut self,
        _: &crate::actions::FindNext,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.find_next(window, cx);
    }

    pub(crate) fn on_find_previous(
        &mut self,
        _: &crate::actions::FindPrevious,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.find_previous(window, cx);
    }

    pub(crate) fn on_replace_current(
        &mut self,
        _: &crate::actions::ReplaceCurrent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_current_search_match(window, cx);
    }

    pub(crate) fn on_replace_all(
        &mut self,
        _: &crate::actions::ReplaceAll,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_all_search_matches(cx);
    }

    /// Executes search with the current query, scope, and filter settings.
    pub fn execute_search(&mut self, cx: &mut Context<Self>) {
        self.search.search_generation = self.search.search_generation.wrapping_add(1);
        let raw_query = self.search.query().to_string();
        if raw_query.is_empty() {
            self.search.matches.clear();
            self.search.active_match_index = None;
            self.clear_search_highlights_from_document(cx);
            cx.notify();
            return;
        }

        self.search.search_input.push_history(&raw_query);

        let search_query = SearchQuery::new(
            &raw_query,
            self.search.match_case,
            self.search.whole_word,
            self.search.use_regex,
        );

        let mut matches = Vec::new();

        match self.search.scope {
            SearchScope::CurrentTab => {
                self.search_in_current_document(&search_query, &mut matches, cx);
            }
            SearchScope::Worktree => {
                self.search_in_worktree_files(&search_query, &mut matches, cx);
            }
        }

        self.search.matches = matches;
        if self.search.matches.is_empty() {
            self.search.active_match_index = None;
            self.clear_search_highlights_from_document(cx);
        } else {
            self.search.active_match_index = Some(
                self.search
                    .active_match_index
                    .map(|idx| idx.min(self.search.matches.len() - 1))
                    .unwrap_or(0),
            );
            self.sync_search_highlights_to_document(cx);
        }

        cx.notify();
    }

    /// Tears down the search subsystem on this editor, clearing all highlights,
    /// cancelling matches, and hiding the panel.
    pub fn teardown_search(&mut self, cx: &mut Context<Self>) {
        self.search.visible = false;
        self.search.matches.clear();
        self.search.active_match_index = None;
        self.search.expanded_match_indices.clear();
        self.clear_search_highlights_from_document(cx);
        cx.notify();
    }

    /// Synchronizes search match highlights to the active pane.
    pub fn sync_search_highlights_to_document(&mut self, _cx: &mut App) {}

    /// Clears search highlights from the active pane.
    pub fn clear_search_highlights_from_document(&mut self, _cx: &mut App) {}

    /// Handles keyboard events when typing in search / replace input fields.
    pub fn handle_search_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let keystroke = &event.keystroke;

        if keystroke.key == "escape" {
            self.search.visible = false;
            self.clear_search_highlights_from_document(cx);
            cx.notify();
            cx.stop_propagation();
            return;
        }

        if keystroke.key == "tab" {
            if self.search.show_replace {
                self.search.active_field = match self.search.active_field {
                    SearchActiveField::Query => {
                        window.focus(&self.search.replace_focus_handle, cx);
                        SearchActiveField::Replace
                    }
                    SearchActiveField::Replace => {
                        window.focus(&self.search.search_focus_handle, cx);
                        SearchActiveField::Query
                    }
                };
                cx.notify();
            }
            cx.stop_propagation();
            return;
        }

        if keystroke.key == "enter" {
            if self.search.active_field == SearchActiveField::Replace {
                self.replace_current_search_match(window, cx);
            } else {
                if keystroke.modifiers.shift {
                    self.search.prev_match();
                } else {
                    self.search.next_match();
                }
                self.jump_to_active_search_match(window, cx);
            }
            cx.notify();
            cx.stop_propagation();
            return;
        }

        if (keystroke.modifiers.control || keystroke.modifiers.platform) && keystroke.key == "a" {
            match self.search.active_field {
                SearchActiveField::Query => self.search.search_input.select_all(),
                SearchActiveField::Replace => self.search.replace_input.select_all(),
            }
            cx.notify();
            cx.stop_propagation();
            return;
        }

        let is_query_field = self.search.active_field == SearchActiveField::Query;
        let input = if is_query_field {
            &mut self.search.search_input
        } else {
            &mut self.search.replace_input
        };

        match keystroke.key.as_str() {
            "up" if is_query_field => {
                if input.history_prev() {
                    self.execute_search(cx);
                }
                cx.stop_propagation();
                return;
            }
            "down" if is_query_field => {
                if input.history_next() {
                    self.execute_search(cx);
                }
                cx.stop_propagation();
                return;
            }
            "backspace" => {
                input.delete_backward();
            }
            "delete" => {
                input.delete_forward();
            }
            "left" => {
                input.move_left(keystroke.modifiers.shift);
            }
            "right" => {
                input.move_right(keystroke.modifiers.shift);
            }
            "home" => {
                input.move_home(keystroke.modifiers.shift);
            }
            "end" => {
                input.move_end(keystroke.modifiers.shift);
            }
            _ => {
                return;
            }
        }

        if is_query_field {
            self.execute_search(cx);
        } else {
            cx.notify();
        }
        cx.stop_propagation();
    }

    /// Searches inside the currently active document's blocks or source code buffer.
    fn search_in_current_document(
        &self,
        query: &SearchQuery,
        matches: &mut Vec<SearchMatch>,
        cx: &App,
    ) {
        let Some(tab) = self.active_tab() else {
            return;
        };
        let active_file_path = tab.file.path.clone();
        let file_name = active_file_path
            .as_ref()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| "Untitled".to_string());

        let active_pane = self.active_pane_id();
        let mut pane_matches = if let Some(state) = self.pane_state_ref(active_pane) {
            state.pane.search_matches(query, cx)
        } else {
            Vec::new()
        };

        for m in &mut pane_matches {
            m.file_path = active_file_path.clone();
            m.file_name = file_name.clone();
        }
        matches.extend(pane_matches);
    }

    /// Searches inside all files belonging to open worktree directories.
    fn search_in_worktree_files(
        &self,
        query: &SearchQuery,
        matches: &mut Vec<SearchMatch>,
        _cx: &App,
    ) {
        let mut search_dirs = Vec::new();
        if let Some(tab) = self.active_tab() {
            if let Some(ref path) = tab.file.path {
                if let Some(parent) = path.parent() {
                    search_dirs.push(parent.to_path_buf());
                }
            }
        }

        if search_dirs.is_empty() {
            return;
        }

        let mut visited_files = std::collections::HashSet::new();

        for dir in search_dirs {
            self.collect_worktree_matches(&dir, query, 0, &mut visited_files, matches);
            if matches.len() >= 500 {
                break;
            }
        }
    }

    /// Recursively searches files in a worktree directory up to max depth.
    fn collect_worktree_matches(
        &self,
        dir: &Path,
        query: &SearchQuery,
        depth: usize,
        visited: &mut std::collections::HashSet<PathBuf>,
        matches: &mut Vec<SearchMatch>,
    ) {
        if depth > 4 || matches.len() >= 500 {
            return;
        }

        let Ok(read_dir) = fs::read_dir(dir) else {
            return;
        };

        for entry in read_dir.flatten() {
            if matches.len() >= 500 {
                break;
            }
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if !name_str.starts_with('.')
                    && name_str != "target"
                    && name_str != "node_modules"
                    && name_str != ".git"
                    && name_str != "dist"
                    && name_str != "build"
                {
                    self.collect_worktree_matches(&path, query, depth + 1, visited, matches);
                }
            } else if path.is_file() {
                if visited.contains(&path) {
                    continue;
                }
                visited.insert(path.clone());

                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                if matches!(ext, "md" | "markdown" | "txt" | "rs" | "toml" | "json") {
                    // Avoid reading massive files synchronously
                    if let Ok(metadata) = entry.metadata() {
                        if metadata.len() > 512 * 1024 {
                            continue;
                        }
                    }

                    if let Ok(content) = fs::read_to_string(&path) {
                        let file_name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();

                        for (line_idx, line) in content.lines().enumerate() {
                            if matches.len() >= 500 {
                                break;
                            }
                            for raw in query.find_matches(line, line_idx + 1) {
                                matches.push(SearchMatch {
                                    file_path: Some(path.clone()),
                                    file_name: file_name.clone(),
                                    block_id: None,
                                    entity_id: None,
                                    line_number: raw.line_number,
                                    column_number: raw.column_number,
                                    byte_range: raw.byte_range,
                                    preview_prefix: raw.preview_prefix,
                                    preview_match: raw.preview_match,
                                    preview_suffix: raw.preview_suffix,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    /// Jumps to the currently selected search match in the active pane.
    pub fn jump_to_active_search_match(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) {
        let Some(active_idx) = self.search.active_match_index else {
            return;
        };
        let Some(match_item) = self.search.matches.get(active_idx).cloned() else {
            return;
        };

        if let Some(ref file_path) = match_item.file_path {
            self.open_file_in_panel(file_path, crate::session::TabKind::Persistent, window, cx);
        }

        let active_pane = self.active_pane_id();
        if let Some(state) = self.pane_state_mut(active_pane) {
            state.pane.navigate_to_search_match(&match_item, cx);
            if let Some(handle) = state.pane.focus_handle(cx) {
                handle.focus(window, cx);
            }
        }

        self.sync_search_highlights_to_document(cx);
        self.request_autoscroll(active_pane, core_contracts::AutoscrollStrategy::Center, cx);
        window.refresh();
        cx.notify();
    }

    /// Replaces the current search match in the document.
    /// Replaces the current search match in the document or source code.
    pub fn replace_current_search_match(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) {
        let Some(active_idx) = self.search.active_match_index else {
            return;
        };
        let Some(match_item) = self.search.matches.get(active_idx).cloned() else {
            return;
        };

        let raw_replace_str = self.search.replace_query().to_string();
        let final_replace_str = core_contracts::compute_preserve_case_replacement(
            &match_item.preview_match,
            &raw_replace_str,
            self.search.preserve_case,
        );
        let range = match_item.byte_range.clone();

        let active_pane = self.active_pane_id();
        if let Some(state) = self.pane_state_mut(active_pane) {
            let text = state
                .pane
                .replace_match(&match_item, &final_replace_str, cx);
            self.commit_pane_text(active_pane, text, cx);
        } else if let Some(ref file_path) = match_item.file_path {
            if let Ok(content) = fs::read_to_string(file_path) {
                if range.start <= content.len() && range.end <= content.len() {
                    let mut new_content = content[..range.start].to_string();
                    new_content.push_str(&final_replace_str);
                    new_content.push_str(&content[range.end..]);
                    let _ = fs::write(file_path, new_content);
                }
            }
        }

        self.execute_search(cx);
        self.jump_to_active_search_match(window, cx);
    }

    /// Replaces all search matches in the active document.
    pub fn replace_all_search_matches(&mut self, cx: &mut Context<Self>) {
        if self.search.matches.is_empty() {
            return;
        }

        let raw_replace_str = self.search.replace_query().to_string();
        let query = SearchQuery::new(
            self.search.query(),
            self.search.match_case,
            self.search.whole_word,
            self.search.use_regex,
        );
        let active_pane = self.active_pane_id();
        if let Some(state) = self.pane_state_mut(active_pane) {
            let text = state.pane.replace_all_matches(&query, &raw_replace_str, cx);
            self.commit_pane_text(active_pane, text, cx);
        }

        self.execute_search(cx);
    }
}
