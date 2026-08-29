//! Search and replace query execution engine.

use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

use gpui::{App, Context, EntityId, KeyDownEvent, Window};

use crate::editor::engine::controller::Editor;
use editor_search::SearchQuery;
use editor_search::{SearchActiveField, SearchMatch, SearchScope};

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
        _: &crate::editor::commands::actions::ToggleSearch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_search(window, cx);
    }

    pub(crate) fn on_toggle_replace(
        &mut self,
        _: &crate::editor::commands::actions::ToggleReplace,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_replace(window, cx);
    }

    pub(crate) fn on_find_next(
        &mut self,
        _: &crate::editor::commands::actions::FindNext,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.find_next(window, cx);
    }

    pub(crate) fn on_find_previous(
        &mut self,
        _: &crate::editor::commands::actions::FindPrevious,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.find_previous(window, cx);
    }

    pub(crate) fn on_replace_current(
        &mut self,
        _: &crate::editor::commands::actions::ReplaceCurrent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_current_search_match(window, cx);
    }

    pub(crate) fn on_replace_all(
        &mut self,
        _: &crate::editor::commands::actions::ReplaceAll,
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

    /// Synchronizes search match highlights to all block entities in the active document.
    pub fn sync_search_highlights_to_document(&mut self, cx: &mut App) {
        let Some(doc) = self.active_doc() else {
            return;
        };

        if !self.search.visible || self.search.matches.is_empty() {
            self.clear_search_highlights_from_document(cx);
            return;
        }

        let active_match = self.search.current_match().cloned();

        // 1. Group matches by block entity_id for WYSIWYG
        let mut matches_by_entity: std::collections::HashMap<EntityId, Vec<(Range<usize>, bool)>> =
            std::collections::HashMap::new();
        for m in &self.search.matches {
            if let Some(entity_id) = m.entity_id {
                let is_active = if let Some(curr) = active_match.as_ref() {
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

        // 2. Sync SourceCode and Preview panes for the active tab
        let search_matches = self.search.matches.clone();
        let search_input_text = self.search.search_input.text.clone();
        let match_case = self.search.match_case;
        let whole_word = self.search.whole_word;
        let use_regex = self.search.use_regex;
        let active_match_index = self.search.active_match_index;
        let doc_entity_ids: Vec<EntityId> = self
            .active_doc()
            .map(|d| d.blocks().iter().map(|e| e.entity.entity_id()).collect())
            .unwrap_or_default();

        if let Some(tab) = self.active_tab_mut() {
            for pane_state in tab.panes.values_mut() {
                if let Some(source) = pane_state.as_source_code_mut() {
                    if !source.text.is_empty() {
                        let raw_text = &source.text;
                        let mut source_matches = Vec::new();
                        let mut cur_line = 1usize;
                        let mut cur_byte = 0usize;
                        for line in raw_text.split_inclusive('\n') {
                            for m in &search_matches {
                                if m.line_number == cur_line {
                                    let mut cur_col = 1usize;
                                    for (ch_idx, _) in line.char_indices() {
                                        if cur_col == m.column_number {
                                            let match_len = m.preview_match.len();
                                            let r = (cur_byte + ch_idx)..(cur_byte + ch_idx + match_len);
                                            let is_active = if let Some(curr) = active_match.as_ref() {
                                                curr.line_number == m.line_number
                                                    && curr.column_number == m.column_number
                                            } else {
                                                false
                                            };
                                            source_matches.push((r, is_active));
                                            break;
                                        }
                                        cur_col += 1;
                                    }
                                }
                            }
                            cur_byte += line.len();
                            cur_line += 1;
                        }
                        source.search_matches = source_matches;
                    }
                }

                if let Some(preview) = pane_state.as_preview_mut() {
                    if !preview.blocks.is_empty() {
                        let preview_query = SearchQuery::new(
                            &search_input_text,
                            match_case,
                            whole_word,
                            use_regex,
                        );
                        for (b_idx, preview_block) in preview.blocks.iter_mut().enumerate() {
                            let mut preview_matches = Vec::new();
                            let rendered_text = preview_block.display_text().to_string();
                            let doc_entity_id = doc_entity_ids.get(b_idx).copied();
                            let entity_matches: Vec<(usize, &SearchMatch)> = search_matches
                                .iter()
                                .enumerate()
                                .filter(|(_, m)| m.entity_id == doc_entity_id)
                                .collect();

                            for (raw_idx, raw) in
                                preview_query.find_matches(&rendered_text, 1).into_iter().enumerate()
                            {
                                let is_active =
                                    entity_matches.get(raw_idx).is_some_and(|(global_idx, _)| {
                                        active_match_index == Some(*global_idx)
                                    });
                                preview_matches.push((raw.byte_range, is_active));
                            }
                            preview_block.search_matches = preview_matches;
                        }
                    }
                }
            }
        }
    }

    /// Clears search highlights from all block entities in the active document and all pane views.
    pub fn clear_search_highlights_from_document(&mut self, cx: &mut App) {
        if let Some(doc) = self.active_doc() {
            for entry in doc.blocks() {
                entry.entity.update(cx, |block, cx| {
                    if !block.search_matches.is_empty() {
                        block.search_matches.clear();
                        cx.notify();
                    }
                });
            }
        }
        if let Some(tab) = self.active_tab_mut() {
            for pane_state in tab.panes.values_mut() {
                if let Some(source) = pane_state.as_source_code_mut() {
                    if !source.search_matches.is_empty() {
                        source.search_matches.clear();
                    }
                }
                if let Some(preview) = pane_state.as_preview_mut() {
                    for preview_block in &mut preview.blocks {
                        if !preview_block.search_matches.is_empty() {
                            preview_block.search_matches.clear();
                        }
                    }
                }
            }
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

    /// Searches inside the currently active document's blocks.
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

            for raw in query.find_matches(&text, line_counter) {
                matches.push(SearchMatch {
                    file_path: active_file_path.clone(),
                    file_name: file_name.clone(),
                    block_id: Some(block_id),
                    entity_id: Some(entity_id),
                    line_number: raw.line_number,
                    column_number: raw.column_number,
                    byte_range: raw.byte_range,
                    preview_prefix: raw.preview_prefix,
                    preview_match: raw.preview_match,
                    preview_suffix: raw.preview_suffix,
                });
            }

            let block_lines = text.lines().count().max(1);
            line_counter += block_lines;
        }
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
    pub fn jump_to_active_search_match(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) {
        let Some(active_idx) = self.search.active_match_index else {
            return;
        };
        let Some(match_item) = self.search.matches.get(active_idx).cloned() else {
            return;
        };

        let active_pane = self.active_pane_id();
        let pane_kind = self
            .session
            .root
            .tree
            .find_leaf_kind(active_pane.0)
            .unwrap_or(crate::editor::engine::controller::EditorPaneKind::Wysiwyg);

        if let Some(entity_id) = match_item.entity_id {
            match pane_kind {
                crate::editor::engine::controller::EditorPaneKind::Wysiwyg => {
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
                }
                crate::editor::engine::controller::EditorPaneKind::SourceCode => {
                    self.sync_source_pane(active_pane, cx);
                    if let Some(source) = self.pane_state_mut(active_pane).and_then(|p| p.as_source_code_mut()) {
                        let raw_text = &source.text;
                        let mut target_range = 0..0;
                        let mut cur_line = 1usize;
                        let mut cur_byte = 0usize;
                        for line in raw_text.split_inclusive('\n') {
                            if cur_line == match_item.line_number {
                                let mut cur_col = 1usize;
                                for (ch_idx, _) in line.char_indices() {
                                    if cur_col == match_item.column_number {
                                        let match_len = match_item.preview_match.len();
                                        target_range = (cur_byte + ch_idx)..(cur_byte + ch_idx + match_len);
                                        break;
                                    }
                                    cur_col += 1;
                                }
                                break;
                            }
                            cur_byte += line.len();
                            cur_line += 1;
                        }
                        source.selection = if target_range.is_empty() { None } else { Some(target_range.clone()) };
                        source.cursor = target_range.end;
                    }
                }
                crate::editor::engine::controller::EditorPaneKind::Preview => {
                    self.refresh_preview_blocks(active_pane, cx);
                }
            }

            self.sync_search_highlights_to_document(cx);
            self.request_autoscroll(
                active_pane,
                crate::editor::engine::controller::AutoscrollStrategy::Center,
                cx,
            );
            window.refresh();
            cx.notify();
        } else if let Some(file_path) = match_item.file_path {
            self.open_file_in_panel(
                &file_path,
                crate::editor::engine::controller::OpenFileMode::Persistent,
                window,
                cx,
            );

            let active_pane = self.active_pane_id();
            let pane_kind = self
                .session
                .root
                .tree
                .find_leaf_kind(active_pane.0)
                .unwrap_or(crate::editor::engine::controller::EditorPaneKind::Wysiwyg);

            match pane_kind {
                crate::editor::engine::controller::EditorPaneKind::Wysiwyg => {
                    let mut found_target = None;
                    if let Some(doc) = self.active_doc() {
                        let mut line_counter = 1usize;
                        for entry in doc.blocks() {
                            let block_text = entry.entity.read(cx).display_text().to_string();
                            let block_lines = block_text.lines().count().max(1);
                            if match_item.line_number >= line_counter
                                && match_item.line_number < line_counter + block_lines
                            {
                                found_target = Some((entry.entity.clone(), entry.entity.entity_id()));
                                break;
                            }
                            line_counter += block_lines;
                        }
                    }
                    if let Some((target_entity, entity_id)) = found_target {
                        self.focus_block(entity_id);
                        let range = match_item.byte_range.clone();
                        target_entity.update(cx, |block, cx| {
                            block.selected_range = range;
                            block.selection_reversed = false;
                            block.start_cursor_blink(cx);
                            cx.notify();
                        });
                    }
                }
                crate::editor::engine::controller::EditorPaneKind::SourceCode => {
                    self.sync_source_pane(active_pane, cx);
                    if let Some(source) = self.pane_state_mut(active_pane).and_then(|p| p.as_source_code_mut()) {
                        let raw_text = &source.text;
                        let mut target_range = 0..0;
                        let mut cur_line = 1usize;
                        let mut cur_byte = 0usize;
                        for line in raw_text.split_inclusive('\n') {
                            if cur_line == match_item.line_number {
                                let mut cur_col = 1usize;
                                for (ch_idx, _) in line.char_indices() {
                                    if cur_col == match_item.column_number {
                                        let match_len = match_item.preview_match.len();
                                        target_range = (cur_byte + ch_idx)..(cur_byte + ch_idx + match_len);
                                        break;
                                    }
                                    cur_col += 1;
                                }
                                break;
                            }
                            cur_byte += line.len();
                            cur_line += 1;
                        }
                        source.selection = if target_range.is_empty() { None } else { Some(target_range.clone()) };
                        source.cursor = target_range.end;
                    }
                }
                crate::editor::engine::controller::EditorPaneKind::Preview => {
                    self.refresh_preview_blocks(active_pane, cx);
                }
            }

            self.sync_search_highlights_to_document(cx);
            self.request_autoscroll(
                active_pane,
                crate::editor::engine::controller::AutoscrollStrategy::Center,
                cx,
            );
            window.refresh();
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
                block.data.text = markdown::inline::text::BlockText::plain(new_text);
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
                    block.data.text = markdown::inline::text::BlockText::plain(current_text);
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
