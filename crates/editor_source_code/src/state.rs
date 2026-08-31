use std::ops::Range;
use std::sync::{Arc, Mutex};

use gpui::{Bounds, FocusHandle, Pixels};

use crate::highlight::{
    CodeHighlightResult, highlight_code_block,
};

/// Pure-Rust state for a raw Markdown source code editor pane.
#[derive(Clone, Debug, Default)]
pub struct SourceCodeState {
    pub text: String,
    pub line_ranges: Vec<Range<usize>>,
    pub cursor: usize,
    pub selection: Option<Range<usize>>,
    pub marked_range: Option<Range<usize>>,
    pub last_bounds: Option<Bounds<Pixels>>,
    pub search_matches: Vec<(Range<usize>, bool)>,
    pub synced_doc_hash: u64,
    pub synced_revision: Option<u64>,
    pub synced_tab_index: Option<usize>,
    pub is_dragging: bool,
    pub drag_anchor: Option<usize>,
    pub focus_handle: Arc<Mutex<Option<FocusHandle>>>,
    pub highlight_cache: Option<CodeHighlightResult>,
    pub highlight_hash: u64,
}

impl SourceCodeState {
    /// Creates a new SourceCodeState from initial text.
    pub fn from_text(text: impl Into<String>) -> Self {
        let text = text.into();
        let mut state = Self {
            text,
            ..Default::default()
        };
        state.rebuild_lines();
        state
    }

    /// Rebuilds cached line byte ranges.
    pub fn rebuild_lines(&mut self) {
        let mut lines = Vec::new();
        let mut start = 0;
        for part in self.text.split('\n') {
            lines.push(start..start + part.len());
            start += part.len() + 1;
        }
        if lines.is_empty() {
            lines.push(0..0);
        }
        self.line_ranges = lines;
    }

    /// Total number of lines in the buffer.
    #[inline]
    pub fn line_count(&self) -> usize {
        if self.line_ranges.is_empty() {
            1
        } else {
            self.line_ranges.len()
        }
    }

    /// Returns the byte range of a given 0-indexed line.
    #[inline]
    pub fn line_range(&self, line_index: usize) -> Range<usize> {
        if line_index < self.line_ranges.len() {
            self.line_ranges[line_index].clone()
        } else if let Some(last) = self.line_ranges.last() {
            last.end..last.end
        } else {
            0..0
        }
    }

    /// Returns start byte offset of a given 0-indexed line.
    #[inline]
    pub fn line_start_offset(&self, line_index: usize) -> usize {
        self.line_range(line_index).start
    }

    /// Returns end byte offset (before '\n') of a given 0-indexed line.
    #[inline]
    pub fn line_end_offset(&self, line_index: usize) -> usize {
        self.line_range(line_index).end
    }

    /// Returns string slice of a given 0-indexed line.
    #[inline]
    pub fn line_str(&self, line_index: usize) -> &str {
        let range = self.line_range(line_index);
        let start = range.start.min(self.text.len());
        let end = range.end.min(self.text.len());
        &self.text[start..end]
    }

    /// Returns (0-indexed line, 0-indexed byte column within that line).
    pub fn line_and_column(&self, offset: usize) -> (usize, usize) {
        let clamped = offset.min(self.text.len());
        if self.line_ranges.is_empty() {
            return (0, clamped);
        }
        let line_idx = match self.line_ranges.binary_search_by(|r| {
            if clamped < r.start {
                std::cmp::Ordering::Greater
            } else if clamped > r.end {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        }) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1).min(self.line_ranges.len() - 1),
        };
        let line_start = self.line_ranges[line_idx].start;
        let col = clamped.saturating_sub(line_start);
        (line_idx, col)
    }

    /// Returns the byte offset corresponding to a given (0-indexed line, 0-indexed byte column).
    pub fn offset_at_line_col(&self, line_index: usize, col: usize) -> usize {
        let range = self.line_range(line_index);
        let line_len = range.end.saturating_sub(range.start);
        let clamped_col = col.min(line_len);
        let target = range.start + clamped_col;
        clamp_to_char_boundary(&self.text, target)
    }

    /// Start a mouse drag selection session.
    pub fn start_drag(&mut self, offset: usize) {
        let clamped = offset.min(self.text.len());
        self.cursor = clamped;
        self.selection = None;
        self.is_dragging = true;
        self.drag_anchor = Some(clamped);
    }

    /// Update mouse drag selection session with a new target offset.
    pub fn update_drag(&mut self, offset: usize) {
        let Some(anchor) = self.drag_anchor else {
            self.cursor = offset.min(self.text.len());
            return;
        };
        let target = offset.min(self.text.len());
        self.cursor = target;
        if anchor == target {
            self.selection = None;
        } else {
            let start = anchor.min(target);
            let end = anchor.max(target);
            self.selection = Some(start..end);
        }
    }

    /// End mouse drag selection session.
    pub fn end_drag(&mut self) {
        self.is_dragging = false;
        self.drag_anchor = None;
    }

    /// Select word around a given byte offset.
    pub fn select_word_at(&mut self, offset: usize) {
        if self.text.is_empty() {
            return;
        }
        let pos = offset.min(self.text.len());
        let s = self.text.as_str();

        let mut start = pos;
        while start > 0 {
            let prev = prev_char_boundary(s, start);
            let ch = s[prev..start].chars().next().unwrap_or(' ');
            if ch.is_alphanumeric() || ch == '_' {
                start = prev;
            } else {
                break;
            }
        }

        let mut end = pos;
        while end < s.len() {
            let next = next_char_boundary(s, end);
            let ch = s[end..next].chars().next().unwrap_or(' ');
            if ch.is_alphanumeric() || ch == '_' {
                end = next;
            } else {
                break;
            }
        }

        if start < end {
            self.selection = Some(start..end);
            self.cursor = end;
        } else {
            self.move_to(pos, false);
        }
    }

    /// Select entire line at given line index.
    pub fn select_line_at(&mut self, line_index: usize) {
        let range = self.line_range(line_index);
        self.selection = Some(range.clone());
        self.cursor = range.end;
    }

    /// Update the buffer's full text from an external sync.
    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.rebuild_lines();
        self.cursor = self.cursor.min(self.text.len());
        if let Some(sel) = self.selection.as_mut() {
            sel.start = sel.start.min(self.text.len());
            sel.end = sel.end.min(self.text.len());
            if sel.start == sel.end {
                self.selection = None;
            }
        }
        self.refresh_highlight();
    }

    /// Refresh syntax highlighting cache if text has changed.
    pub fn refresh_highlight(&mut self) {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.text.hash(&mut h);
        let hash = h.finish();
        if hash != self.highlight_hash || self.highlight_cache.is_none() {
            self.highlight_cache = highlight_code_block(Some("markdown"), &self.text);
            self.highlight_hash = hash;
        }
    }

    /// Returns the currently selected text slice, if any.
    pub fn selected_text(&self) -> Option<&str> {
        let sel = self.selection.as_ref()?;
        if sel.start < sel.end && sel.end <= self.text.len() {
            Some(&self.text[sel.start..sel.end])
        } else {
            None
        }
    }

    /// Inserts text at the current cursor position, replacing selection if any.
    pub fn insert_text(&mut self, inserted: &str) {
        if let Some(sel) = self.selection.take() {
            let start = sel.start.min(self.text.len());
            let end = sel.end.min(self.text.len());
            self.text.replace_range(start..end, inserted);
            self.cursor = start + inserted.len();
        } else {
            let pos = self.cursor.min(self.text.len());
            self.text.insert_str(pos, inserted);
            self.cursor = pos + inserted.len();
        }
        self.rebuild_lines();
        self.refresh_highlight();
    }

    /// Deletes text backward (Backspace).
    pub fn delete_backward(&mut self) {
        if let Some(sel) = self.selection.take() {
            let start = sel.start.min(self.text.len());
            let end = sel.end.min(self.text.len());
            self.text.replace_range(start..end, "");
            self.cursor = start;
        } else if self.cursor > 0 {
            let prev = prev_char_boundary(&self.text, self.cursor);
            self.text.replace_range(prev..self.cursor, "");
            self.cursor = prev;
        }
        self.rebuild_lines();
        self.refresh_highlight();
    }

    /// Deletes text forward (Delete).
    pub fn delete_forward(&mut self) {
        if let Some(sel) = self.selection.take() {
            let start = sel.start.min(self.text.len());
            let end = sel.end.min(self.text.len());
            self.text.replace_range(start..end, "");
            self.cursor = start;
        } else if self.cursor < self.text.len() {
            let next = next_char_boundary(&self.text, self.cursor);
            self.text.replace_range(self.cursor..next, "");
        }
        self.rebuild_lines();
        self.refresh_highlight();
    }

    /// Moves cursor to a specific byte offset.
    pub fn move_to(&mut self, offset: usize, extend_selection: bool) {
        let target = offset.min(self.text.len());
        if extend_selection {
            let anchor = match self.selection.as_ref() {
                Some(sel) => {
                    if self.cursor == sel.end {
                        sel.start
                    } else {
                        sel.end
                    }
                }
                None => self.cursor,
            };
            let (start, end) = if target < anchor {
                (target, anchor)
            } else {
                (anchor, target)
            };
            self.selection = if start == end { None } else { Some(start..end) };
        } else {
            self.selection = None;
        }
        self.cursor = target;
    }

    pub fn move_left(&mut self, extend_selection: bool) {
        if !extend_selection && self.selection.is_some() {
            let start = self.selection.take().unwrap().start;
            self.cursor = start;
            return;
        }
        if self.cursor > 0 {
            let prev = prev_char_boundary(&self.text, self.cursor);
            self.move_to(prev, extend_selection);
        }
    }

    pub fn move_right(&mut self, extend_selection: bool) {
        if !extend_selection && self.selection.is_some() {
            let end = self.selection.take().unwrap().end;
            self.cursor = end;
            return;
        }
        if self.cursor < self.text.len() {
            let next = next_char_boundary(&self.text, self.cursor);
            self.move_to(next, extend_selection);
        }
    }

    pub fn move_up(&mut self, extend_selection: bool) {
        let (cur_line, col) = self.line_and_column(self.cursor);
        if cur_line > 0 {
            let target_line = cur_line - 1;
            let target_offset = self.offset_at_line_col(target_line, col);
            self.move_to(target_offset, extend_selection);
        } else {
            self.move_to(0, extend_selection);
        }
    }

    pub fn move_down(&mut self, extend_selection: bool) {
        let (cur_line, col) = self.line_and_column(self.cursor);
        let total_lines = self.line_count();
        if cur_line + 1 < total_lines {
            let target_line = cur_line + 1;
            let target_offset = self.offset_at_line_col(target_line, col);
            self.move_to(target_offset, extend_selection);
        } else {
            self.move_to(self.text.len(), extend_selection);
        }
    }

    pub fn move_to_line_start(&mut self, extend_selection: bool) {
        let (cur_line, _) = self.line_and_column(self.cursor);
        let range = self.line_range(cur_line);
        self.move_to(range.start, extend_selection);
    }

    pub fn move_to_line_end(&mut self, extend_selection: bool) {
        let (cur_line, _) = self.line_and_column(self.cursor);
        let range = self.line_range(cur_line);
        self.move_to(range.end, extend_selection);
    }

    pub fn select_all(&mut self) {
        if !self.text.is_empty() {
            self.selection = Some(0..self.text.len());
            self.cursor = self.text.len();
        }
    }
}

fn prev_char_boundary(s: &str, mut idx: usize) -> usize {
    if idx == 0 {
        return 0;
    }
    idx -= 1;
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

fn next_char_boundary(s: &str, mut idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    idx += 1;
    while idx < s.len() && !s.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

fn clamp_to_char_boundary(s: &str, mut idx: usize) -> usize {
    idx = idx.min(s.len());
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

// ── Pane plugin contract ─────────────────────────────────────────────────

use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement,
    StatefulInteractiveElement, Styled, Window, div,
};
use editor_model::{
    EditorDocument, PaneKindId, PaneOutlineHost, PaneRenderContext, PaneView,
};
use editor_outline::OutlineNode;
use editor_search::{SearchMatch, SearchQuery};
use theme::Theme;

use crate::element::{
    NullSourceIme, SnapshotSourceStateView, SourceCodeViewElement, SourceViewSnapshot,
};

impl SourceCodeState {
    pub fn snapshot(&self, cx: &App) -> SourceViewSnapshot {
        SourceViewSnapshot {
            text: self.text.clone(),
            line_ranges: self.line_ranges.clone(),
            cursor: self.cursor,
            selection: self.selection.clone(),
            highlight_spans: self
                .highlight_cache
                .as_ref()
                .map(|h| h.spans.clone())
                .unwrap_or_default(),
            focus_handle: self.focus_handle(cx),
        }
    }
}

impl PaneView for SourceCodeState {
    fn kind(&self) -> PaneKindId {
        PaneKindId::SOURCE_CODE
    }

    fn focus_handle(&self, cx: &App) -> Option<FocusHandle> {
        let mut guard = self.focus_handle.lock().unwrap();
        if guard.is_none() {
            *guard = Some(cx.focus_handle());
        }
        guard.clone()
    }

    fn cursor_position(&self, _cx: &App) -> Option<(usize, usize)> {
        let (line, col) = self.line_and_column(self.cursor);
        Some((line + 1, col + 1))
    }

    fn document_source(&self, _doc: &dyn EditorDocument, _cx: &App) -> String {
        self.text.clone()
    }

    fn sync_document_text(&mut self, text: &str, revision: u64, _cx: &mut App) {
        let hash = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            text.hash(&mut h);
            h.finish()
        };
        if self.synced_revision != Some(revision) || hash != self.synced_doc_hash {
            if self.text != text {
                self.set_text(text.to_string());
            }
            self.synced_doc_hash = hash;
            self.synced_revision = Some(revision);
        }
    }

    fn serialize_text(&self, _cx: &App) -> Option<String> {
        Some(self.text.clone())
    }

    fn outline_headings(&self, _cx: &App) -> Vec<OutlineNode> {
        crate::outline::extract_outline_headings(&self.text)
    }

    fn navigate_to_outline(&mut self, index: usize, theme: &Theme, _cx: &mut App) {
        let headings = self.outline_headings(_cx);
        if let Some(node) = headings.get(index) {
            crate::outline::navigate_to_node(self, node);
            let font_size = theme.typography.code_size.max(12.0);
            let line_height = (font_size * theme.typography.text_line_height).round().max(18.0);
            let _target_y = (node.block_index as f32 * line_height) - 40.0;
        }
    }

    fn search_matches(&self, query: &SearchQuery, _cx: &App) -> Vec<SearchMatch> {
        crate::search::search_in_source(self, query)
    }

    fn replace_match(
        &mut self,
        match_item: &SearchMatch,
        replace_with: &str,
        _cx: &mut App,
    ) {
        let range = match_item.byte_range.clone();
        crate::search::replace_in_source(self, range, replace_with);
    }

    fn replace_all_matches(
        &mut self,
        query: &SearchQuery,
        replace_with: &str,
        _cx: &mut App,
    ) {
        let matches = self.search_matches(query, _cx);
        let replacements: Vec<(std::ops::Range<usize>, String)> = matches
            .into_iter()
            .map(|m| (m.byte_range, replace_with.to_string()))
            .collect();
        crate::search::replace_all_in_source(self, replacements);
    }

    fn navigate_to_search_match(&mut self, match_item: &SearchMatch, _cx: &mut App) {
        let range = match_item.byte_range.clone();
        let len = self.text.len();
        self.cursor = range.end.min(len);
        self.selection = Some(range.start.min(len)..range.end.min(len));
        self.refresh_highlight();
    }

    fn apply_line_prefix(&mut self, prefix: &str, _cx: &mut App) {
        let (cur_line, _) = self.line_and_column(self.cursor);
        let start = self.line_start_offset(cur_line);
        let end = self.line_end_offset(cur_line);
        let line_text = self.text[start..end].to_string();
        let stripped = line_text
            .trim_start_matches(|c| c == '#' || c == '>' || c == '-' || c == '*' || c == '+' || c == ' ' || c == '\t');
        let new_line = format!("{prefix}{stripped}");
        let prefix_len = prefix.len();
        self.text.replace_range(start..end, &new_line);
        self.cursor = start + prefix_len;
        self.selection = None;
        self.refresh_highlight();
    }

    fn apply_snippet(&mut self, snippet: &str, caret_offset: usize, _cx: &mut App) {
        let pos = self.cursor;
        self.text.insert_str(pos, snippet);
        self.cursor = pos + caret_offset;
        self.selection = None;
        self.refresh_highlight();
    }

    fn apply_wrapped_or_template(
        &mut self,
        empty_template: &str,
        caret_offset_in_empty: usize,
        wrap_prefix: &str,
        wrap_suffix: &str,
        _cx: &mut App,
    ) {
        if let Some(sel) = self.selection.take() {
            let text = self.text[sel.start..sel.end].to_string();
            let wrapped = format!("{wrap_prefix}{text}{wrap_suffix}");
            self.text.replace_range(sel.start..sel.end, &wrapped);
            self.selection = Some(
                sel.start + wrap_prefix.len()..sel.start + wrap_prefix.len() + text.len(),
            );
            self.cursor = sel.start + wrap_prefix.len() + text.len();
        } else {
            let pos = self.cursor;
            self.text.insert_str(pos, empty_template);
            self.cursor = pos + caret_offset_in_empty;
        }
        self.refresh_highlight();
    }

    fn apply_clear_format(&mut self, _cx: &mut App) {
        if let Some(sel) = self.selection.take() {
            let selected = &self.text[sel.start..sel.end];
            let plain = selected
                .trim_matches(|c| c == '*' || c == '_' || c == '~' || c == '`' || c == '=' || c == '$')
                .to_string();
            self.text.replace_range(sel.start..sel.end, &plain);
            self.cursor = sel.start + plain.len();
            self.refresh_highlight();
        }
    }

    fn render(
        &mut self,
        ctx: &PaneRenderContext,
        _window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let theme = cx.global::<theme::ThemeManager>().current_arc();
        let c = &theme.colors;
        let pane_id = ctx.pane_id;

        let focus_handle = self.focus_handle(cx).unwrap();
        let snapshot = self.snapshot(cx);
        let view: Arc<dyn crate::element::SourceStateView> =
            Arc::new(SnapshotSourceStateView(snapshot));
        let ime: Arc<dyn crate::element::SourceIme> = Arc::new(NullSourceIme);

        let outline_host: Arc<dyn editor_outline::OutlineHost> = Arc::new(PaneOutlineHost {
            pane_id: ctx.pane_id,
            host: ctx.host.clone(),
        });
        let outline_hud = editor_outline::render_floating_outline_hud(
            ctx.pane_id.0,
            &self.outline_headings(cx),
            None,
            false,
            &theme,
            &outline_host,
        );

        let host_for_key = ctx.host.clone();
        let host_for_mouse = ctx.host.clone();
        let host_for_move = ctx.host.clone();
        let host_for_up = ctx.host.clone();
        div()
            .id(ElementId::Name(format!("tiled-source-editor-{pane_id}").into()))
            .key_context("SourceCode")
            .track_focus(&focus_handle)
            .w_full()
            .h_full()
            .relative()
            .bg(c.editor_background)
            .font(theme::TypographyStore::default_font(theme::TypographyScope::Code))
            .on_key_down(move |event, window, cx| {
                if host_for_key.handle_pane_key_down(pane_id, event, window, cx) {
                    cx.stop_propagation();
                }
            })
            .on_mouse_down(
                MouseButton::Left,
                move |event, window, cx| {
                    host_for_mouse.focus_pane(pane_id, window, cx);
                    host_for_mouse.handle_pane_mouse_down(pane_id, event, window, cx);
                },
            )
            .on_mouse_move(move |event, window, cx| {
                host_for_move.handle_pane_mouse_move(pane_id, event, window, cx);
            })
            .on_mouse_up(
                MouseButton::Left,
                move |event, window, cx| {
                    host_for_up.handle_pane_mouse_up(pane_id, event, window, cx);
                },
            )
            .child(
                div()
                    .id(ElementId::Name(format!("tiled-source-scroll-{pane_id}").into()))
                    .w_full()
                    .h_full()
                    .overflow_y_scroll()
                    .track_scroll(ctx.scroll)
                    .child(SourceCodeViewElement {
                        view,
                        ime,
                        host: ctx.host.clone(),
                        pane_id,
                    }),
            )
            .child(outline_hud)
            .into_any_element()
    }

    fn handle_key_down(
        &mut self,
        pane_id: editor_model::PaneId,
        event: &gpui::KeyDownEvent,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
        host: &dyn editor_model::PaneHost,
    ) -> bool {
        crate::input::handle_key_down(self, pane_id, event, window, cx, host)
    }

    fn handle_mouse_down(
        &mut self,
        _pane_id: editor_model::PaneId,
        event: &gpui::MouseDownEvent,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) {
        crate::input::handle_mouse_down(self, event, window, cx);
    }

    fn handle_mouse_move(
        &mut self,
        _pane_id: editor_model::PaneId,
        event: &gpui::MouseMoveEvent,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) {
        crate::input::handle_mouse_move(self, event, window, cx);
    }

    fn handle_mouse_up(
        &mut self,
        _pane_id: editor_model::PaneId,
        _event: &gpui::MouseUpEvent,
        _window: &mut gpui::Window,
        _cx: &mut gpui::App,
    ) {
        crate::input::handle_mouse_up(self);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
