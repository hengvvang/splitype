//! Search and replace query execution engine.

use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

use gpui::{App, Context, EntityId, KeyDownEvent, Window};
use regex::RegexBuilder;

use crate::editor::engine::controller::Editor;
use crate::editor::search::state::{SearchActiveField, SearchMatch, SearchScope};
use crate::model::parse::BlockId;

impl Editor {
    /// Executes search with the current query, scope, and filter settings.
    pub fn execute_search(&mut self, cx: &mut Context<Self>) {
        let query = self.search.query().trim().to_string();
        if query.is_empty() {
            self.search.matches.clear();
            self.search.active_match_index = None;
            cx.notify();
            return;
        }

        let mut matches = Vec::new();

        match self.search.scope {
            SearchScope::CurrentTab => {
                self.search_in_current_document(&query, &mut matches, cx);
            }
            SearchScope::Worktree => {
                self.search_in_worktree_files(&query, &mut matches, cx);
            }
        }

        self.search.matches = matches;
        if self.search.matches.is_empty() {
            self.search.active_match_index = None;
        } else {
            // Keep previous index if still in range, otherwise select first match.
            self.search.active_match_index = Some(
                self.search
                    .active_match_index
                    .map(|idx| idx.min(self.search.matches.len() - 1))
                    .unwrap_or(0),
            );
        }

        cx.notify();
    }

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
            if keystroke.modifiers.shift {
                self.search.prev_match();
            } else {
                self.search.next_match();
            }
            self.jump_to_active_search_match(window, cx);
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
                input.cursor = 0;
                input.selected_range = 0..0;
            }
            "end" => {
                input.cursor = input.text.len();
                input.selected_range = input.cursor..input.cursor;
            }
            _ => {
                // Printable character check
                if !keystroke.modifiers.control && !keystroke.modifiers.platform && !keystroke.modifiers.alt {
                    let insert_text = match keystroke.key.as_str() {
                        "space" => " ",
                        k if k.chars().count() == 1 => k,
                        _ => "",
                    };
                    if !insert_text.is_empty() {
                        input.insert_str(insert_text);
                    } else {
                        return;
                    }
                } else {
                    return;
                }
            }
        }

        if is_query_field {
            self.execute_search(cx);
        } else {
            cx.notify();
        }
        cx.stop_propagation();
    }

    /// Searches inside the currently active document's blocks.
    fn search_in_current_document(
        &self,
        query: &str,
        matches: &mut Vec<SearchMatch>,
        cx: &App,
    ) {
        let active_file_path = self.tab().file.path.clone();
        let file_name = active_file_path
            .as_ref()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| "Untitled".to_string());

        let blocks = self.doc().blocks();
        let mut line_counter = 1usize;

        for entry in blocks {
            let entity = &entry.entity;
            let entity_id = entity.entity_id();
            let block_id = entity.read_with(cx, |b, _| b.data.id);
            let text = entity.read_with(cx, |b, _| b.display_text().to_string());

            self.find_matches_in_text(
                &text,
                query,
                active_file_path.clone(),
                file_name.clone(),
                Some(block_id),
                Some(entity_id),
                line_counter,
                matches,
            );

            // Increment line count based on newlines inside block + 1 for block break
            let block_lines = text.lines().count().max(1);
            line_counter += block_lines;
        }
    }

    /// Searches inside all files belonging to open worktree directories.
    fn search_in_worktree_files(
        &self,
        query: &str,
        matches: &mut Vec<SearchMatch>,
        _cx: &App,
    ) {
        let mut search_dirs = Vec::new();
        if let Some(ref path) = self.tab().file.path {
            if let Some(parent) = path.parent() {
                search_dirs.push(parent.to_path_buf());
            }
        }

        if search_dirs.is_empty() {
            if let Ok(current_dir) = std::env::current_dir() {
                search_dirs.push(current_dir);
            }
        }

        let mut visited_files = std::collections::HashSet::new();

        for dir in search_dirs {
            self.collect_worktree_matches(&dir, query, &mut visited_files, matches);
        }
    }

    /// Recursively searches files in a worktree directory.
    fn collect_worktree_matches(
        &self,
        dir: &Path,
        query: &str,
        visited: &mut std::collections::HashSet<PathBuf>,
        matches: &mut Vec<SearchMatch>,
    ) {
        let Ok(read_dir) = fs::read_dir(dir) else {
            return;
        };

        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                // Skip hidden dirs, target, node_modules, .git
                if !name_str.starts_with('.') && name_str != "target" && name_str != "node_modules" {
                    self.collect_worktree_matches(&path, query, visited, matches);
                }
            } else if path.is_file() {
                if visited.contains(&path) {
                    continue;
                }
                visited.insert(path.clone());

                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                if matches!(ext, "md" | "markdown" | "txt" | "rs" | "toml" | "json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        let file_name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();

                        for (line_idx, line) in content.lines().enumerate() {
                            self.find_matches_in_text(
                                line,
                                query,
                                Some(path.clone()),
                                file_name.clone(),
                                None,
                                None,
                                line_idx + 1,
                                matches,
                            );
                        }
                    }
                }
            }
        }
    }

    /// Scans text line or block string for matches matching state filters.
    fn find_matches_in_text(
        &self,
        text: &str,
        query: &str,
        file_path: Option<PathBuf>,
        file_name: String,
        block_id: Option<BlockId>,
        entity_id: Option<EntityId>,
        line_number: usize,
        matches: &mut Vec<SearchMatch>,
    ) {
        if text.is_empty() || query.is_empty() {
            return;
        }

        if self.search.use_regex {
            let mut builder = RegexBuilder::new(query);
            builder.case_insensitive(!self.search.match_case);
            if let Ok(regex) = builder.build() {
                for mat in regex.find_iter(text) {
                    let range = mat.range();
                    let prefix_start = range.start.saturating_sub(30);
                    let suffix_end = (range.end + 30).min(text.len());

                    matches.push(SearchMatch {
                        file_path: file_path.clone(),
                        file_name: file_name.clone(),
                        block_id,
                        entity_id,
                        line_number,
                        byte_range: range.clone(),
                        preview_prefix: text[prefix_start..range.start].to_string(),
                        preview_match: text[range.clone()].to_string(),
                        preview_suffix: text[range.end..suffix_end].to_string(),
                    });
                }
            }
        } else {
            let search_text = if self.search.match_case {
                text.to_string()
            } else {
                text.to_lowercase()
            };
            let search_query = if self.search.match_case {
                query.to_string()
            } else {
                query.to_lowercase()
            };

            let mut start = 0usize;
            while let Some(rel_idx) = search_text[start..].find(&search_query) {
                let match_start = start + rel_idx;
                let match_end = match_start + query.len();
                start = match_end;

                if self.search.whole_word {
                    let before_is_word = match_start > 0
                        && text[..match_start]
                            .chars()
                            .last()
                            .is_some_and(|c| c.is_alphanumeric() || c == '_');
                    let after_is_word = match_end < text.len()
                        && text[match_end..]
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_alphanumeric() || c == '_');
                    if before_is_word || after_is_word {
                        continue;
                    }
                }

                let prefix_start = match_start.saturating_sub(30);
                let suffix_end = (match_end + 30).min(text.len());

                matches.push(SearchMatch {
                    file_path: file_path.clone(),
                    file_name: file_name.clone(),
                    block_id,
                    entity_id,
                    line_number,
                    byte_range: match_start..match_end,
                    preview_prefix: text[prefix_start..match_start].to_string(),
                    preview_match: text[match_start..match_end].to_string(),
                    preview_suffix: text[match_end..suffix_end].to_string(),
                });
            }
        }
    }

    /// Jumps to the currently selected search match.
    pub fn jump_to_active_search_match(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) {
        let Some(active_idx) = self.search.active_match_index else {
            return;
        };
        let Some(match_item) = self.search.matches.get(active_idx).cloned() else {
            return;
        };

        // If match belongs to current file:
        if let Some(entity_id) = match_item.entity_id {
            self.focus_block(entity_id);
            if let Some(block) = self.doc().block_entity_by_id(entity_id) {
                let range = match_item.byte_range.clone();
                block.update(cx, |block, cx| {
                    block.selected_range = range;
                    block.selection_reversed = false;
                    block.start_cursor_blink(cx);
                    cx.notify();
                });
            }
            let active_pane = self.active_pane_id();
            self.request_autoscroll(
                active_pane,
                crate::editor::engine::controller::AutoscrollStrategy::Center,
                cx,
            );
            cx.notify();
        } else if let Some(file_path) = match_item.file_path {
            // Match is in another file in the worktree: open or switch tab
            self.open_file_in_tab(file_path, window, cx);
        }
    }

    /// Opens a file in the active editor tab set or switches to it if already open.
    pub fn open_file_in_tab(&mut self, path: PathBuf, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        // Check if tab already open
        if let Some(existing_idx) = self
            .tab_list_mut()
            .iter()
            .position(|tab| tab.file.path.as_ref() == Some(&path))
        {
            self.activate_tab(existing_idx, cx);
            cx.notify();
            return;
        }

        // Otherwise open in a new tab
        if let Ok(content) = fs::read_to_string(&path) {
            self.new_untitled_tab(cx);
            let active_idx = self.tab_list_mut().active_index();
            self.tab_mut().file.path = Some(path);
            self.rebuild_document_from_markdown(&content, cx);
            self.activate_tab(active_idx, cx);
            cx.notify();
        }
    }

    /// Replaces the current search match in the document.
    pub fn replace_current_search_match(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) {
        let Some(active_idx) = self.search.active_match_index else {
            return;
        };
        let Some(match_item) = self.search.matches.get(active_idx).cloned() else {
            return;
        };
        let Some(entity_id) = match_item.entity_id else {
            return;
        };
        let Some(block) = self.doc().block_entity_by_id(entity_id) else {
            return;
        };

        let replace_str = self.search.replace_query().to_string();
        let range = match_item.byte_range.clone();

        self.prepare_undo_capture(
            crate::editor::document::protocol::UndoCaptureKind::NonCoalescible,
            cx,
        );

        block.update(cx, |block, cx| {
            let current_text = block.display_text().to_string();
            if range.start <= current_text.len() && range.end <= current_text.len() {
                let mut new_text = current_text[..range.start].to_string();
                new_text.push_str(&replace_str);
                new_text.push_str(&current_text[range.end..]);
                block.data.text = crate::model::inline::text::BlockText::plain(new_text);
                block.selected_range = range.start..(range.start + replace_str.len());
                block.refresh_cached_display_text();
                block.sync_render_cache();
                cx.notify();
            }
        });

        self.mark_dirty(cx);
        self.finalize_pending_undo_capture(cx);

        // Re-execute search to refresh matches
        self.execute_search(cx);
        self.jump_to_active_search_match(window, cx);
    }

    /// Replaces all search matches in the active document.
    pub fn replace_all_search_matches(&mut self, cx: &mut Context<Self>) {
        if self.search.matches.is_empty() {
            return;
        }

        let replace_str = self.search.replace_query().to_string();

        self.prepare_undo_capture(
            crate::editor::document::protocol::UndoCaptureKind::NonCoalescible,
            cx,
        );

        // Group matches by block entity ID
        let mut matches_by_entity: std::collections::HashMap<EntityId, Vec<Range<usize>>> =
            std::collections::HashMap::new();

        for m in &self.search.matches {
            if let Some(entity_id) = m.entity_id {
                matches_by_entity
                    .entry(entity_id)
                    .or_default()
                    .push(m.byte_range.clone());
            }
        }

        for (entity_id, mut ranges) in matches_by_entity {
            if let Some(block) = self.doc().block_entity_by_id(entity_id) {
                // Sort ranges descending so replacing from tail does not shift preceding offsets
                ranges.sort_by_key(|r| std::cmp::Reverse(r.start));

                block.update(cx, |block, cx| {
                    let mut current_text = block.display_text().to_string();
                    for range in ranges {
                        if range.start <= current_text.len() && range.end <= current_text.len() {
                            let mut new_text = current_text[..range.start].to_string();
                            new_text.push_str(&replace_str);
                            new_text.push_str(&current_text[range.end..]);
                            current_text = new_text;
                        }
                    }
                    block.data.text = crate::model::inline::text::BlockText::plain(current_text);
                    block.refresh_cached_display_text();
                    block.sync_render_cache();
                    cx.notify();
                });
            }
        }

        self.mark_dirty(cx);
        self.finalize_pending_undo_capture(cx);

        // Re-execute search
        self.execute_search(cx);
    }
}
