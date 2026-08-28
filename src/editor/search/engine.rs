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
        let query = self.search.query().to_string();
        if query.is_empty() {
            self.search.matches.clear();
            self.search.active_match_index = None;
            self.clear_search_highlights_from_document(cx);
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

    /// Synchronizes search match highlights to all block entities in the active document.
    pub fn sync_search_highlights_to_document(&self, cx: &mut App) {
        let Some(doc) = self.active_doc() else {
            return;
        };

        if !self.search.visible || self.search.matches.is_empty() {
            self.clear_search_highlights_from_document(cx);
            return;
        }

        let active_match = self.search.current_match();

        // Group matches by block entity_id
        let mut matches_by_entity: std::collections::HashMap<EntityId, Vec<(Range<usize>, bool)>> =
            std::collections::HashMap::new();

        for m in &self.search.matches {
            if let Some(entity_id) = m.entity_id {
                let is_active = if let Some(curr) = active_match {
                    curr.entity_id == Some(entity_id) && curr.byte_range == m.byte_range
                } else {
                    false
                };
                matches_by_entity
                    .entry(entity_id)
                    .or_default()
                    .push((m.byte_range.clone(), is_active));
            }
        }

        for entry in doc.blocks() {
            let entity = &entry.entity;
            let entity_id = entity.entity_id();
            let new_matches = matches_by_entity.remove(&entity_id).unwrap_or_default();
            entity.update(cx, |block, cx| {
                if block.search_matches != new_matches {
                    block.search_matches = new_matches;
                    cx.notify();
                }
            });
        }
    }

    /// Clears search highlights from all block entities in the active document.
    pub fn clear_search_highlights_from_document(&self, cx: &mut App) {
        let Some(doc) = self.active_doc() else {
            return;
        };

        for entry in doc.blocks() {
            entry.entity.update(cx, |block, cx| {
                if !block.search_matches.is_empty() {
                    block.search_matches.clear();
                    cx.notify();
                }
            });
        }
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

    /// Searches inside the currently active document's blocks.
    fn search_in_current_document(
        &self,
        query: &str,
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

        let Some(doc) = self.active_doc() else {
            return;
        };
        let blocks = doc.blocks();
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
        query: &str,
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

    /// Scans text line or block string for matches matching state filters with full UTF-8 Unicode safety.
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

        let pattern = if self.search.use_regex {
            query.to_string()
        } else {
            let escaped = regex::escape(query);
            if self.search.whole_word {
                format!(r"\b{}\b", escaped)
            } else {
                escaped
            }
        };

        let mut builder = RegexBuilder::new(&pattern);
        builder.case_insensitive(!self.search.match_case);

        let Ok(regex) = builder.build() else {
            return;
        };

        for mat in regex.find_iter(text) {
            let range = mat.range();
            let matched_slice = &text[range.clone()];

            let prefix_text = &text[..range.start];
            let relative_line = prefix_text.matches('\n').count();
            let actual_line = line_number + relative_line;

            let last_nl = prefix_text.rfind('\n').map(|p| p + 1).unwrap_or(0);
            let same_line_prefix = &prefix_text[last_nl..];
            let column_number = same_line_prefix.chars().count() + 1;

            let suffix_text = &text[range.end..];
            let next_nl = suffix_text.find('\n').unwrap_or(suffix_text.len());
            let same_line_suffix = &suffix_text[..next_nl];

            // Safe char-based prefix extraction on the same line (up to 20 Unicode chars)
            let preview_prefix: String = same_line_prefix
                .chars()
                .rev()
                .take(20)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();

            // Safe char-based suffix extraction on the same line (up to 20 Unicode chars)
            let preview_suffix: String = same_line_suffix
                .chars()
                .take(20)
                .collect();

            matches.push(SearchMatch {
                file_path: file_path.clone(),
                file_name: file_name.clone(),
                block_id,
                entity_id,
                line_number: actual_line,
                column_number,
                byte_range: range.clone(),
                preview_prefix,
                preview_match: matched_slice.replace(['\r', '\n'], " "),
                preview_suffix,
            });
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

        if let Some(entity_id) = match_item.entity_id {
            self.focus_block(entity_id);
            if let Some(doc) = self.active_doc() {
                if let Some(block) = doc.block_entity_by_id(entity_id) {
                    let range = match_item.byte_range.clone();
                    block.update(cx, |block, cx| {
                        block.selected_range = range;
                        block.selection_reversed = false;
                        block.start_cursor_blink(cx);
                        cx.notify();
                    });
                }
            }
            let active_pane = self.active_pane_id();
            self.request_autoscroll(
                active_pane,
                crate::editor::engine::controller::AutoscrollStrategy::Center,
                cx,
            );
            self.sync_search_highlights_to_document(cx);
            cx.notify();
        } else if let Some(file_path) = match_item.file_path {
            self.open_file_in_panel(
                &file_path,
                crate::editor::engine::controller::OpenFileMode::Persistent,
                window,
                cx,
            );
            self.sync_search_highlights_to_document(cx);
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
        let Some(doc) = self.active_doc() else {
            return;
        };
        let Some(block) = doc.block_entity_by_id(entity_id) else {
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
            if range.start <= current_text.len()
                && range.end <= current_text.len()
                && current_text.is_char_boundary(range.start)
                && current_text.is_char_boundary(range.end)
            {
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

        let Some(doc) = self.active_doc() else {
            return;
        };

        for (entity_id, mut ranges) in matches_by_entity {
            if let Some(block) = doc.block_entity_by_id(entity_id) {
                ranges.sort_by_key(|r| std::cmp::Reverse(r.start));

                block.update(cx, |block, cx| {
                    let mut current_text = block.display_text().to_string();
                    for range in ranges {
                        if range.start <= current_text.len()
                            && range.end <= current_text.len()
                            && current_text.is_char_boundary(range.start)
                            && current_text.is_char_boundary(range.end)
                        {
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

        self.execute_search(cx);
    }
}
