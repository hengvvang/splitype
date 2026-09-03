//! The source code editor engine: one gpui entity per pane, projecting the
//! shared document buffer into a local editable text copy.
//!
//! The editor owns cursor state, folds, soft wrapping, and rendering caches;
//! the document text itself is committed back to the shared buffer as edit
//! transactions, and undo history lives entirely in the buffer (see
//! `editor_contracts::EditTransaction`).

pub mod edit;
pub mod ime;
pub mod keys;
pub mod mouse;
pub mod movement;
pub mod render;

use std::ops::Range;
use std::sync::Arc;
use std::time::Instant;

use config::settings::PluginSettings;
use editor_contracts::{CursorHint, DocumentSnapshot, EditTransaction, OutlineNode, PaneHost};
use gpui::*;
use syntax_highlighter::highlight::{CodeHighlightResult, highlight_code_block};

use crate::buffer::LineMap;
use crate::display_map::{DisplaySnapshot, FoldMap, RowIndex, TabMap, WrapState};
use crate::selection::Selections;
use crate::settings::SourceCodeSettings;

/// A continuous typing run, used to group consecutive insertions/deletions
/// into one buffer-level undo transaction.
#[derive(Clone, Copy, Debug)]
pub struct TypingRun {
    /// Cursor offset after the previous keystroke of the run.
    pub insert_at: usize,
    /// Cursor hint captured at the start of the run.
    pub start_hint: CursorHint,
}

/// The source code editor engine entity.
pub struct SourceCodeEditor {
    pub host: Option<Arc<dyn PaneHost>>,
    pub scroll: Option<ScrollHandle>,
    pub focus_handle: FocusHandle,

    // ── Document projection ──────────────────────────────────────────────
    text: String,
    line_map: LineMap,
    synced_revision: Option<u64>,
    /// A local edit exists that the host's next snapshot has not echoed yet.
    pending_edit: bool,

    // ── Editing state ────────────────────────────────────────────────────
    selections: Selections,
    marked_range: Option<Range<usize>>,
    is_dragging: bool,
    drag_anchor: Option<usize>,
    typing_run: Option<TypingRun>,

    // ── Display transformation ───────────────────────────────────────────
    settings: SourceCodeSettings,
    folds: FoldMap,
    wrap: WrapState,
    /// Viewport width the wrap state was computed for; `None` = dirty.
    wrap_width_px: Option<f32>,

    // ── Caches ───────────────────────────────────────────────────────────
    highlight_cache: Option<CodeHighlightResult>,
    /// Theme family the highlight cache was computed for.
    highlight_theme: String,
    /// Flattened display rows, rebuilt only when the text, folds, or wrap
    /// state change (never per frame).
    rows_cache: Option<Arc<RowIndex>>,
    /// Outline headings keyed by the revision they were extracted at.
    outline_cache: Option<(u64, Vec<OutlineNode>)>,
    search_matches: Vec<(Range<usize>, bool)>,
    last_bounds: Bounds<Pixels>,
    cursor_blink_epoch: Option<Instant>,
    /// Shaped rows of the last frame, for hit-testing mouse positions.
    pub(crate) frame_rows: Vec<render::RowFrame>,
    /// A local edit awaiting commit at the end of the effect cycle. The
    /// pane's key handler runs inside the host editor's own update, so
    /// commits must not re-enter the editor entity synchronously.
    deferred_commit: Option<(bool, CursorHint)>,
}

impl SourceCodeEditor {
    pub fn new(document: &DocumentSnapshot, cx: &mut Context<Self>) -> Self {
        let settings = PluginSettings::<SourceCodeSettings>::get(cx);
        let mut editor = Self {
            host: None,
            scroll: None,
            focus_handle: cx.focus_handle(),
            text: String::new(),
            line_map: LineMap::default(),
            synced_revision: None,
            pending_edit: false,
            selections: Selections::new(0),
            marked_range: None,
            is_dragging: false,
            drag_anchor: None,
            typing_run: None,
            settings,
            folds: FoldMap::new(),
            wrap: WrapState::default(),
            wrap_width_px: None,
            highlight_cache: None,
            highlight_theme: String::new(),
            rows_cache: None,
            outline_cache: None,
            search_matches: Vec::new(),
            last_bounds: Bounds::default(),
            cursor_blink_epoch: None,
            frame_rows: Vec::new(),
            deferred_commit: None,
        };
        editor.apply_document_text(&document.text, document.revision);
        editor
    }

    // ── Document synchronization ─────────────────────────────────────────

    /// Replaces the local text with a document snapshot, rebuilding derived
    /// state. Folds are pruned to the new line count.
    fn apply_document_text(&mut self, text: &str, revision: u64) {
        self.text = normalize_line_endings(text);
        self.rebuild_derived();
        self.synced_revision = Some(revision);
        self.marked_range = None;
        self.is_dragging = false;
        self.drag_anchor = None;
        self.typing_run = None;
        // Any queued commit now refers to pre-sync text; drop it.
        self.deferred_commit = None;
    }

    /// Converges the editor to a document snapshot; no-ops when the editor
    /// is already at this revision or when it was the edit's originator.
    pub fn sync_document(&mut self, document: &DocumentSnapshot, cx: &mut Context<Self>) {
        let settings = PluginSettings::<SourceCodeSettings>::get(cx);
        if settings != self.settings {
            self.settings = settings;
            self.invalidate_wrap();
        }
        if self.pending_edit {
            self.synced_revision = Some(document.revision);
            self.pending_edit = false;
            return;
        }
        // Revision equality implies text equality: the buffer only bumps
        // its revision when the text changes, and local edits are covered
        // by the pending-edit path above. Avoids an O(n) string compare
        // per pane per frame.
        if self.synced_revision == Some(document.revision) {
            return;
        }
        self.apply_document_text(&document.text, document.revision);
        if let Some(hint) = document.restore_cursor {
            self.restore_cursor_hint(hint);
        }
        cx.notify();
    }

    pub fn document_text(&self) -> String {
        self.text.clone()
    }

    // ── Cursor and selection access ──────────────────────────────────────

    pub fn cursor(&self) -> usize {
        self.selections.cursor()
    }

    pub fn cursor_hint(&self) -> CursorHint {
        CursorHint::from_offset(&self.text, self.cursor())
    }

    pub fn restore_cursor_hint(&mut self, hint: CursorHint) {
        let offset = hint.to_offset(&self.text);
        self.selections.set_single_point(offset);
    }

    pub fn selections(&self) -> &Selections {
        &self.selections
    }

    pub fn selections_mut(&mut self) -> &mut Selections {
        &mut self.selections
    }

    pub fn marked_range(&self) -> Option<Range<usize>> {
        self.marked_range.clone()
    }

    pub fn set_marked_range(&mut self, range: Option<Range<usize>>) {
        self.marked_range = range;
    }

    pub fn selected_text(&self) -> Option<String> {
        self.selections
            .primary_range()
            .map(|range| self.text[range].to_string())
    }

    /// Ends the current typing run (any cursor movement does this).
    pub fn reset_typing_run(&mut self) {
        self.typing_run = None;
    }

    /// Whether an edit inserting `inserted` at `insert_pos` continues the
    /// current typing run: a single-character insertion at the run's insert
    /// point, or an update while an IME composition is active.
    pub fn merge_for_insert(&self, insert_pos: usize, inserted: &str) -> bool {
        if self.marked_range.is_some() {
            return true;
        }
        let Some(run) = &self.typing_run else {
            return false;
        };
        inserted.chars().count() == 1 && insert_pos == run.insert_at
    }

    /// Records one local edit's typing-run bookkeeping. Merged edits keep
    /// the run's start hint and update its insert point; anything else
    /// starts (or resets to) a fresh run.
    fn record_edit_run(
        &mut self,
        merge: bool,
        cursor_before: CursorHint,
        insert_pos: Option<usize>,
    ) {
        if merge {
            if let Some(run) = &mut self.typing_run {
                if let Some(pos) = insert_pos {
                    run.insert_at = pos;
                }
                return;
            }
        }
        self.typing_run = insert_pos.map(|pos| TypingRun {
            insert_at: pos,
            start_hint: cursor_before,
        });
    }

    /// Rebuilds every text-derived cache after a local edit: line map,
    /// fold pruning, wrap invalidation, highlight cache, and selection
    /// clamping.
    fn rebuild_derived(&mut self) {
        self.line_map = LineMap::new(&self.text);
        self.selections.clamp_and_sort(self.text.len());
        let line_count = self.line_map.line_count() as u32;
        self.folds.prune_to_line_count(line_count);
        self.invalidate_wrap();
        self.highlight_cache = None;
        self.rows_cache = None;
        self.outline_cache = None;
    }

    /// Commits a local edit through the pane host after refreshing the
    /// cursor blink phase. The commit is deferred to the end of the effect
    /// cycle: the pane's key handler runs inside the host editor's own
    /// update, and a synchronous [`PaneHost::commit_edit`] would re-enter
    /// the editor entity.
    fn commit_local_edit(
        &mut self,
        merge: bool,
        cursor_before: CursorHint,
        cx: &mut Context<Self>,
    ) {
        self.start_cursor_blink();
        // Guard the local text against any snapshot broadcast that arrives
        // before the deferred commit lands.
        self.pending_edit = true;
        self.deferred_commit = Some((merge, cursor_before));
        let entity = cx.entity();
        cx.defer(move |cx| {
            entity.update(cx, |editor, cx| {
                if let Some((merge, cursor_before)) = editor.deferred_commit.take() {
                    editor.commit_edit(merge, cursor_before, cx);
                }
            });
        });
        cx.notify();
    }

    // ── Commit machinery ─────────────────────────────────────────────────

    /// Builds the edit transaction for the current local state. `cursor_before`
    /// is the caret hint before the edit being committed.
    pub fn take_transaction(
        &mut self,
        merge: bool,
        cursor_before: CursorHint,
    ) -> Option<EditTransaction> {
        // Normalize so the local text matches what the buffer will store.
        if self.text.contains('\r') {
            self.text = normalize_line_endings(&self.text);
            self.line_map = LineMap::new(&self.text);
            self.selections.clamp_and_sort(self.text.len());
        }
        self.pending_edit = true;
        let cursor_after = self.cursor_hint();
        Some(EditTransaction::new(
            self.text.clone(),
            merge,
            cursor_before,
            cursor_after,
        ))
    }

    /// Commits the current state through the pane host.
    pub fn commit_edit(&mut self, merge: bool, cursor_before: CursorHint, cx: &mut App) {
        if let Some(transaction) = self.take_transaction(merge, cursor_before) {
            if let Some(host) = self.host.clone() {
                host.commit_edit(transaction, cx);
            }
        }
    }

    /// Scrolls the viewport so the primary caret is visible.
    pub fn scroll_cursor_into_view(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(scroll) = self.scroll.clone() else {
            return;
        };
        let snapshot = self.snapshot();
        let cursor_row = snapshot.offset_to_display_point(self.cursor()).row;
        let line_height = self.line_height_px(cx);
        let padding = self.editor_padding_px(cx);
        let cursor_y = cursor_row as f32 * line_height + padding;
        let viewport = scroll.bounds();
        let viewport_height = f32::from(viewport.size.height);
        let offset_y = f32::from(scroll.offset().y);
        let top = -offset_y;
        let bottom = top + viewport_height;

        let target = if cursor_y + line_height > bottom {
            -(cursor_y + line_height - viewport_height).max(0.0)
        } else if cursor_y < top + padding {
            -(cursor_y - padding).max(0.0)
        } else {
            return;
        };
        scroll.set_offset(point(px(0.0), px(target)));
        window.refresh();
        cx.notify();
    }

    // ── Display transformation ───────────────────────────────────────────

    /// Builds the display snapshot. The flattened row index is cached on
    /// the editor and invalidated only by text/fold/wrap changes.
    pub fn snapshot(&self) -> DisplaySnapshot<'_> {
        let rows = self.rows_cache.clone().unwrap_or_else(|| {
            Arc::new(RowIndex::build(
                self.line_map.line_count(),
                &self.folds,
                &self.wrap,
            ))
        });
        DisplaySnapshot::new(
            &self.text,
            &self.line_map,
            TabMap::new(self.settings.tab_size),
            &self.folds,
            &self.wrap,
            rows,
        )
    }

    /// Warms the row-index cache. Called from the render path (which has
    /// `&mut self`) before the element reads the snapshot.
    pub fn ensure_rows_cache(&mut self) {
        if self.rows_cache.is_none() {
            self.rows_cache = Some(Arc::new(RowIndex::build(
                self.line_map.line_count(),
                &self.folds,
                &self.wrap,
            )));
        }
    }

    /// Ensures the wrap state matches the given viewport width, recomputing
    /// when the text, width, or wrap settings changed.
    pub fn ensure_wrap(&mut self, viewport_width_px: f32, cx: &App) -> bool {
        if !self.settings.word_wrap {
            if !self.wrap.points.is_empty() || self.wrap_width_px.is_some() {
                self.wrap = WrapState::default();
                self.wrap_width_px = None;
                self.rows_cache = None;
                return true;
            }
            return false;
        }
        if self.wrap_width_px == Some(viewport_width_px) {
            return false;
        }
        let char_width = self.char_width_px(cx);
        let text_width_px =
            (viewport_width_px - self.gutter_width_px(cx) - 12.0).max(char_width * 8.0);
        let max_columns = (text_width_px / char_width).floor().max(8.0) as u32;
        let tab_map = TabMap::new(self.settings.tab_size);
        let mut points = Vec::with_capacity(self.line_map.line_count());
        for row in 0..self.line_map.line_count() {
            let line = self.line_str(row);
            points.push(Self::wrap_points_for_line(line, max_columns, &tab_map));
        }
        self.wrap = WrapState::new(viewport_width_px, points);
        self.wrap_width_px = Some(viewport_width_px);
        self.rows_cache = None;
        true
    }

    /// Computes the byte offsets where a line wraps given a maximum column
    /// count (tab-expanded, hard wrap at the column that exceeds the limit).
    fn wrap_points_for_line(line: &str, max_columns: u32, tab_map: &TabMap) -> Vec<usize> {
        let mut points = Vec::new();
        let mut display_column = 0u32;
        let mut wrap_span_columns = 0u32;
        for (byte_offset, ch) in line.char_indices() {
            let char_columns = if ch == '\t' {
                tab_map.tab_size - (display_column % tab_map.tab_size)
            } else {
                1
            };
            // Break before this character when it would overflow the row,
            // but never leave an empty row behind.
            if wrap_span_columns > 0
                && wrap_span_columns + char_columns > max_columns
                && byte_offset > 0
            {
                points.push(byte_offset);
                wrap_span_columns = 0;
            }
            display_column += char_columns;
            wrap_span_columns += char_columns;
        }
        points
    }

    fn invalidate_wrap(&mut self) {
        self.wrap_width_px = None;
    }

    /// The foldable region whose header is the primary cursor's line.
    pub fn foldable_at_cursor(&self) -> Option<crate::display_map::FoldRange> {
        let row = self.line_map.offset_to_point(self.cursor()).row;
        self.folds.foldable_at(&self.text, &self.line_map, row)
    }

    /// Toggles the fold containing or heading the primary cursor's line.
    pub fn toggle_fold_at_cursor(&mut self) {
        let row = self.line_map.offset_to_point(self.cursor()).row;
        if self.folds.is_folded(row) {
            self.folds.unfold(row);
            self.rows_cache = None;
            return;
        }
        // A cursor inside a fold should unfold it.
        if self.folds.is_row_hidden(row) {
            let header = self.folds.header_of_hidden(row);
            if let Some(header) = header {
                self.folds.unfold(header);
                self.rows_cache = None;
                return;
            }
        }
        if let Some(range) = self.folds.foldable_at(&self.text, &self.line_map, row) {
            self.folds.fold(range);
            self.rows_cache = None;
        }
    }

    pub fn unfold_all(&mut self) {
        self.folds.unfold_all();
        self.rows_cache = None;
    }

    // ── Text and layout accessors ────────────────────────────────────────

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn line_map(&self) -> &LineMap {
        &self.line_map
    }

    pub fn line_str(&self, row: usize) -> &str {
        let start = self.line_map.line_start(row);
        let len = self.line_map.line_len(row);
        &self.text[start..start + len]
    }

    pub fn line_count(&self) -> usize {
        self.line_map.line_count()
    }

    pub fn settings(&self) -> SourceCodeSettings {
        self.settings
    }

    pub fn folds(&self) -> &FoldMap {
        &self.folds
    }

    pub fn search_matches(&self) -> &[(Range<usize>, bool)] {
        &self.search_matches
    }

    pub fn set_search_matches(&mut self, matches: Vec<(Range<usize>, bool)>) {
        self.search_matches = matches;
    }

    pub fn last_bounds(&self) -> Bounds<Pixels> {
        self.last_bounds
    }

    pub fn set_last_bounds(&mut self, bounds: Bounds<Pixels>) {
        self.last_bounds = bounds;
    }

    /// Starts (or restarts) the cursor blink phase.
    pub fn start_cursor_blink(&mut self) {
        self.cursor_blink_epoch = Some(Instant::now());
    }

    pub fn cursor_blink_epoch(&self) -> Option<Instant> {
        self.cursor_blink_epoch
    }

    // ── Syntax highlighting cache ────────────────────────────────────────

    /// Returns cached Markdown highlights. The cache is invalidated on
    /// every text edit (`rebuild_derived`) and recomputed only when the
    /// theme family changed — never re-derived per frame.
    pub fn highlight(&mut self, theme_name: &str) -> Option<&CodeHighlightResult> {
        if self.highlight_cache.is_none() || self.highlight_theme != theme_name {
            self.highlight_cache = highlight_code_block(Some("markdown"), &self.text);
            self.highlight_theme = theme_name.to_string();
        }
        self.highlight_cache.as_ref()
    }

    /// Returns the Markdown outline headings, cached by document revision.
    /// Local edits invalidate the cache (`rebuild_derived`), so this is
    /// O(1) on frames without text changes instead of re-parsing the whole
    /// document every frame.
    pub fn cached_outline_headings(&mut self) -> &[OutlineNode] {
        let revision = self.synced_revision.unwrap_or(0);
        let stale = self
            .outline_cache
            .as_ref()
            .is_none_or(|(cached_rev, _)| *cached_rev != revision);
        if stale {
            let headings = crate::outline::extract_outline_headings(&self.text);
            self.outline_cache = Some((revision, headings));
        }
        &self.outline_cache.as_ref().expect("cache just filled").1
    }

    // ── Metrics ──────────────────────────────────────────────────────────

    pub fn line_height_px(&self, cx: &App) -> f32 {
        let theme = cx.global::<theme::ThemeManager>().current_arc();
        let font_size = theme.typography.code_size.max(12.0);
        (font_size * theme.typography.text_line_height).round()
    }

    pub fn char_width_px(&self, cx: &App) -> f32 {
        let theme = cx.global::<theme::ThemeManager>().current_arc();
        theme.typography.code_size.max(12.0) * 0.6
    }

    pub fn editor_padding_px(&self, cx: &App) -> f32 {
        let theme = cx.global::<theme::ThemeManager>().current_arc();
        theme.dimensions.editor_padding
    }

    pub fn gutter_width_px(&self, cx: &App) -> f32 {
        let theme = cx.global::<theme::ThemeManager>().current_arc();
        let font_size = theme.typography.code_size.max(12.0);
        if self.settings.line_numbers {
            crate::gutter::GutterLayout::new(self.line_count(), font_size).width()
        } else {
            8.0
        }
    }
}

/// Normalizes CRLF / CR line endings to LF so pane text always matches what
/// the shared buffer stores.
pub fn normalize_line_endings(text: &str) -> String {
    if text.contains('\r') {
        text.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        text.to_string()
    }
}
