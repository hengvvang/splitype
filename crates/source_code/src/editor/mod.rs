//! The source code editor engine: one gpui entity per pane, projecting the
//! shared document buffer into a local editable text copy.
//!
//! The editor owns cursor state, folds, soft wrapping, and rendering caches;
//! the document text itself is committed back to the shared buffer as edit
//! transactions, and undo history lives entirely in the buffer (see
//! `editor_contracts::EditTransaction`).
//!
//! The text layer mirrors Zed's architecture: an immutable chunk [`Rope`]
//! with incremental line indexing (edits are O(chunks), never O(document)),
//! a local `text_version` that every derived cache keys on, a
//! content-addressed per-line wrap cache, and a background highlight
//! pipeline that re-derives syntax spans and outline headings after an
//! idle debounce (stale-while-revalidate, like Zed's async highlighting).

pub mod context_menu;
pub mod edit;
pub mod ime;
pub mod keys;
pub mod mouse;
pub mod movement;
pub mod render;

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::Arc;
use std::time::{Duration, Instant};

use config::settings::PluginSettings;
use editor_contracts::{
    CursorHint, DocumentSnapshot, EditTransaction, HighlightSnapshot, OutlineNode, PaneHost,
};
use gpui::*;
use syntax_highlighter::highlight::CodeHighlightSpan;

use crate::Rope;
use crate::display_map::{DisplaySnapshot, FoldMap, FoldRange, RowIndex, TabMap, WrapState};
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

/// Per-line span index keyed by the highlight version it was built from.
type SpanIndexCache = (u64, Arc<Vec<Vec<CodeHighlightSpan>>>);

/// Background outline/folds pipeline: recomputes outline headings and
/// foldable regions after an idle debounce, keeping the previous result
/// visible while typing (stale-while-revalidate). Highlight spans arrive
/// through the document snapshot instead — the buffer's engine owns them.
struct HighlightState {
    /// Outline headings computed alongside the last run.
    outline: Option<Arc<Vec<OutlineNode>>>,
    /// Foldable regions (fences and heading sections) from the last
    /// run, sorted by header row.
    folds: Option<Arc<Vec<FoldRange>>>,
    /// Bumped on every schedule; guards stale completions.
    generation: u64,
    /// In-flight debounce + compute task.
    task: Option<Task<()>>,
}

impl HighlightState {
    fn new() -> Self {
        Self {
            outline: None,
            folds: None,
            generation: 0,
            task: None,
        }
    }
}

/// The source code editor engine entity.
pub struct SourceCodeEditor {
    pub host: Option<Arc<dyn PaneHost>>,
    pub scroll: Option<ScrollHandle>,
    pub focus_handle: FocusHandle,
    /// Context menu currently open in this pane, if any.
    pub context_menu: Option<context_menu::SourceContextMenu>,

    // ── Document projection ──────────────────────────────────────────────
    text: Rope,
    /// Local text version, bumped on every local or applied text change.
    /// All derived caches key on this, not on the shared buffer revision.
    text_version: u64,
    synced_revision: Option<u64>,
    /// A local edit exists that the host's next snapshot has not echoed yet.
    pending_edit: bool,
    /// Replacements of the local edits awaiting commit, in application
    /// (back-to-front) order with each range in the coordinates preceding
    /// that edit.
    commit_edits: Vec<(Range<usize>, Arc<str>)>,
    /// Latest highlights from the document snapshot; may lag a few
    /// revisions (stale-while-revalidate).
    highlights: Option<Arc<HighlightSnapshot>>,
    /// Per-line span index built from `highlights`, keyed by its version.
    spans_by_line_cache: RefCell<Option<SpanIndexCache>>,

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
    /// Column budget the wrap points were computed for (pure numbers, so
    /// incremental wrap patching needs no font access).
    wrap_max_columns: Option<u32>,
    /// Content-addressed per-line wrap points (line hash → points). Edits
    /// never invalidate it; it is rebuilt when the wrap width changes.
    wrap_cache: HashMap<u64, Arc<Vec<usize>>>,
    /// Union byte span (original-text coordinates) of the local edits
    /// since the last `after_text_change`; drives incremental wrap and
    /// row-index patching.
    edit_span: Option<(usize, usize)>,
    /// Total bytes inserted by the pending local edits.
    edit_inserted: usize,
    /// Final-text span still awaiting wrap-point patching (consumed by
    /// `ensure_wrap` on the next frame).
    dirty_wrap_span: Option<(usize, usize)>,

    // ── Caches ───────────────────────────────────────────────────────────
    rows_cache: Option<Arc<RowIndex>>,
    /// Line count the `rows_cache` was built for; a mismatch means the
    /// index is stale.
    rows_line_count: Option<usize>,
    /// Cached matching-bracket result for (text_version, cursor).
    bracket_cache: Option<(u64, usize, Option<usize>)>,
    /// The bracket offset resolved for the current frame (computed in
    /// `render`, consumed by the element's prepaint).
    pub(crate) bracket_offset: Option<usize>,
    highlight: HighlightState,
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
            context_menu: None,
            text: Rope::new(""),
            text_version: 0,
            synced_revision: None,
            pending_edit: false,
            commit_edits: Vec::new(),
            highlights: None,
            spans_by_line_cache: RefCell::new(None),
            selections: Selections::new(0),
            marked_range: None,
            is_dragging: false,
            drag_anchor: None,
            typing_run: None,
            settings,
            folds: FoldMap::new(),
            wrap: WrapState::default(),
            wrap_width_px: None,
            wrap_max_columns: None,
            wrap_cache: HashMap::new(),
            edit_span: None,
            edit_inserted: 0,
            dirty_wrap_span: None,
            rows_cache: None,
            rows_line_count: None,
            bracket_cache: None,
            bracket_offset: None,
            highlight: HighlightState::new(),
            search_matches: Vec::new(),
            last_bounds: Bounds::default(),
            cursor_blink_epoch: None,
            frame_rows: Vec::new(),
            deferred_commit: None,
        };
        editor.apply_document(document.rope.clone(), document.revision, cx);
        editor.highlights = document.highlights.clone();
        editor
    }

    // ── Document synchronization ─────────────────────────────────────────

    /// Replaces the local text with the document rope, rebuilding derived
    /// state. Folds are pruned to the new line count.
    fn apply_document(&mut self, rope: Arc<Rope>, revision: u64, cx: &mut Context<Self>) {
        self.text = rope.as_ref().clone();
        self.selections.clamp_and_sort(self.text.len());
        self.edit_span = None;
        self.edit_inserted = 0;
        self.commit_edits.clear();
        *self.spans_by_line_cache.borrow_mut() = None;
        self.after_text_change();
        self.schedule_highlight(cx);
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
        // Highlights track the buffer's engine; adopt every broadcast.
        if let Some(highlights) = &document.highlights {
            if self.highlights.as_ref().map(|h| h.version) != Some(highlights.version) {
                self.highlights = Some(highlights.clone());
                *self.spans_by_line_cache.borrow_mut() = None;
                cx.notify();
            }
        }
        if self.pending_edit {
            self.synced_revision = Some(document.revision);
            self.pending_edit = false;
            return;
        }
        // Revision equality implies text equality: the buffer only bumps
        // its revision when the text changes, and local edits are covered
        // by the pending-edit path above. Avoids an O(n) compare per pane
        // per frame.
        if self.synced_revision == Some(document.revision) {
            return;
        }
        let had_text = !self.text.is_empty();
        self.apply_document(document.rope.clone(), document.revision, cx);
        if let Some(hint) = document.restore_cursor {
            self.restore_cursor_hint(hint);
        }
        if had_text {
            cx.notify();
        }
    }

    pub fn document_text(&self) -> String {
        self.text.materialize()
    }

    // ── Cursor and selection access ──────────────────────────────────────

    pub fn cursor(&self) -> usize {
        self.selections.cursor()
    }

    pub fn cursor_hint(&self) -> CursorHint {
        CursorHint::from_rope(&self.text, self.cursor())
    }

    pub fn restore_cursor_hint(&mut self, hint: CursorHint) {
        let offset = hint.to_rope_offset(&self.text);
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
            .map(|range| self.text.slice_owned(range))
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

    /// Applies a local rope edit without touching derived caches. Callers
    /// must follow up with `after_text_change` + `schedule_highlight` once
    /// per logical edit. Edits are applied back-to-front, so each edit's
    /// coordinates are also original-text coordinates; the accumulated
    /// span drives incremental wrap/row-index patching, and the same
    /// replacements are handed to the buffer on commit.
    pub(crate) fn replace_local(&mut self, range: Range<usize>, replacement: &str) {
        let (start, end) = (
            range.start.min(self.text.len()),
            range.end.min(self.text.len()),
        );
        match &mut self.edit_span {
            None => self.edit_span = Some((start, end)),
            Some(span) => {
                span.0 = span.0.min(start);
                span.1 = span.1.max(end);
            }
        }
        self.edit_inserted += replacement.len();
        self.commit_edits.push((start..end, Arc::from(replacement)));
        self.text = self.text.edit(start..end, replacement);
    }

    /// Applies a local rope edit and updates every text-derived cache. The
    /// rope maintains its line index incrementally; this only bumps the
    /// version, prunes folds, patches soft-wrap points for the touched
    /// lines, drops derived caches, and schedules the background highlight
    /// refresh.
    pub fn edit_text(&mut self, range: Range<usize>, inserted: &str, cx: &mut Context<Self>) {
        self.replace_local(range, inserted);
        self.after_text_change();
        self.schedule_highlight(cx);
    }

    /// Bumps the text version and invalidates text-derived caches. When the
    /// change came from local edits (`replace_local`), the display caches
    /// update incrementally: wrap points are patched for the touched lines
    /// and the row index survives when the visual row count is unchanged.
    fn after_text_change(&mut self) {
        self.text_version = self.text_version.wrapping_add(1);
        self.bracket_cache = None;
        let line_count = self.text.line_count();
        let pruned = self.folds.prune_to_line_count(line_count as u32);

        let Some(span) = self.edit_span.take() else {
            // Whole-document replacement: every derived display cache is
            // stale; wrap points rebuild lazily on the next frame.
            self.edit_inserted = 0;
            self.rows_cache = None;
            self.rows_line_count = None;
            self.wrap = WrapState::default();
            self.wrap_width_px = None;
            self.wrap_max_columns = None;
            self.dirty_wrap_span = None;
            return;
        };

        // The final-text span is the original span extended by the total
        // inserted bytes; every changed byte lies within it. Batches that
        // arrive before the next frame merge conservatively (over-covering
        // rows is harmless: unchanged lines are content-cache hits).
        let new_span = (span.0, span.1 + self.edit_inserted);
        self.dirty_wrap_span = Some(match self.dirty_wrap_span {
            None => new_span,
            Some(old) => (
                old.0.min(new_span.0),
                old.1.max(new_span.1) + self.edit_inserted,
            ),
        });
        self.edit_inserted = 0;

        if pruned || self.rows_line_count != Some(line_count) {
            self.rows_cache = None;
            self.rows_line_count = None;
        }
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
        self.pending_edit = true;
        let cursor_after = CursorHint::from_rope(&self.text, self.cursor());
        if self.commit_edits.is_empty() {
            return None;
        }
        let edits = std::mem::take(&mut self.commit_edits);
        Some(EditTransaction::from_edits(
            edits,
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
                self.text.line_count(),
                &self.folds,
                &self.wrap,
            ))
        });
        DisplaySnapshot::new(
            &self.text,
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
                self.text.line_count(),
                &self.folds,
                &self.wrap,
            )));
            self.rows_line_count = Some(self.text.line_count());
        }
    }

    /// Ensures the wrap state matches the given viewport width. On a width
    /// change all lines rebuild (content-addressed, so unchanged lines are
    /// free); otherwise only the lines touched by edits since the last
    /// frame are patched, and the row index survives when no line's visual
    /// row count changed.
    pub fn ensure_wrap(&mut self, viewport_width_px: f32, cx: &App) -> bool {
        if !self.settings.word_wrap {
            let had_wrap = !self.wrap.points.is_empty() || self.wrap_width_px.is_some();
            self.dirty_wrap_span = None;
            if had_wrap {
                self.wrap = WrapState::default();
                self.wrap_width_px = None;
                self.wrap_max_columns = None;
                self.rows_cache = None;
                self.rows_line_count = None;
                return true;
            }
            return false;
        }
        if self.wrap_width_px != Some(viewport_width_px) {
            return self.rebuild_wrap(viewport_width_px, cx);
        }
        let Some(span) = self.dirty_wrap_span.take() else {
            return false;
        };
        if self.wrap.points.len() != self.text.line_count() {
            // A newline was inserted or removed: resizing the points array
            // row-accurately needs the old edit coordinates; rebuild.
            return self.rebuild_wrap(viewport_width_px, cx);
        }
        let max_columns = self.wrap_max_columns.unwrap_or(8);
        let tab_map = TabMap::new(self.settings.tab_size);
        let (first_row, _) = self.text.offset_to_point(span.0.min(self.text.len()));
        let (last_row, _) = self.text.offset_to_point(span.1.min(self.text.len()));
        let mut row_count_changed = false;
        for row in first_row..=last_row.min(self.text.line_count() - 1) {
            let line = self.text.line_str(row);
            let hash = hash_str(line);
            let entry = self.wrap_cache.entry(hash).or_insert_with(|| {
                Arc::new(Self::wrap_points_for_line(line, max_columns, &tab_map))
            });
            if self.wrap.points[row].len() != entry.len() {
                row_count_changed = true;
            }
            self.wrap.points[row] = entry.as_ref().clone();
        }
        if row_count_changed {
            self.rows_cache = None;
            self.rows_line_count = None;
            return true;
        }
        false
    }

    /// Rebuilds the wrap state for a new viewport width.
    fn rebuild_wrap(&mut self, viewport_width_px: f32, cx: &App) -> bool {
        let char_width = self.char_width_px(cx);
        let text_width_px =
            (viewport_width_px - self.gutter_width_px(cx) - 12.0).max(char_width * 8.0);
        let max_columns = (text_width_px / char_width).floor().max(8.0) as u32;
        let tab_map = TabMap::new(self.settings.tab_size);

        // Rebuild the cache when it grows past the number of live lines
        // (stale entries from deleted lines are dropped).
        if self.wrap_cache.len() > self.text.line_count().max(64) * 2 + 64 {
            self.wrap_cache.clear();
        }

        let mut points = Vec::with_capacity(self.text.line_count());
        for row in 0..self.text.line_count() {
            let line = self.text.line_str(row);
            let hash = hash_str(line);
            let entry = self.wrap_cache.entry(hash).or_insert_with(|| {
                Arc::new(Self::wrap_points_for_line(line, max_columns, &tab_map))
            });
            points.push(entry.as_ref().clone());
        }
        self.wrap = WrapState::new(viewport_width_px, points);
        self.wrap_width_px = Some(viewport_width_px);
        self.wrap_max_columns = Some(max_columns);
        self.dirty_wrap_span = None;
        self.rows_cache = None;
        self.rows_line_count = None;
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

    /// The foldable region headed by `row`, from the last background
    /// highlight run (stale-while-typing, like Zed's background parsing).
    pub fn foldable_at(&self, row: u32) -> Option<FoldRange> {
        let ranges = self.highlight.folds.as_deref()?;
        let idx = ranges
            .binary_search_by_key(&row, |range| range.start_row)
            .ok()?;
        Some(ranges[idx])
    }

    /// Toggles the fold headed by `row` (folded → unfold; foldable → fold).
    pub fn toggle_fold_at_row(&mut self, row: u32) {
        if self.folds.is_folded(row) {
            self.folds.unfold(row);
        } else if let Some(range) = self.foldable_at(row) {
            self.folds.fold(range);
        }
        self.rows_cache = None;
        self.rows_line_count = None;
    }

    /// Toggles the fold containing or heading the primary cursor's line.
    pub fn toggle_fold_at_cursor(&mut self) {
        let row = self.text.offset_to_point(self.cursor()).0;
        if self.folds.is_folded(row as u32) {
            self.folds.unfold(row as u32);
            self.rows_cache = None;
            self.rows_line_count = None;
            return;
        }
        // A cursor inside a fold should unfold it.
        if self.folds.is_row_hidden(row as u32) {
            let header = self.folds.header_of_hidden(row as u32);
            if let Some(header) = header {
                self.folds.unfold(header);
                self.rows_cache = None;
                self.rows_line_count = None;
                return;
            }
        }
        if let Some(range) = self.foldable_at(row as u32) {
            self.folds.fold(range);
            self.rows_cache = None;
            self.rows_line_count = None;
        }
    }

    pub fn unfold_all(&mut self) {
        self.folds.unfold_all();
        self.rows_cache = None;
    }

    // ── Text and layout accessors ────────────────────────────────────────

    pub fn text(&self) -> &Rope {
        &self.text
    }

    pub fn text_version(&self) -> u64 {
        self.text_version
    }

    pub fn line_str(&self, row: usize) -> &str {
        self.text.line_str(row)
    }

    pub fn line_count(&self) -> usize {
        self.text.line_count()
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

    // ── Highlight pipeline ────────────────────────────────────────────────

    /// Schedules a background outline/folds refresh after an idle debounce.
    /// The current (possibly stale) result keeps rendering while the task
    /// runs; completions whose generation moved are discarded and the
    /// debounce restarts.
    fn schedule_highlight(&mut self, cx: &mut Context<Self>) {
        self.highlight.generation = self.highlight.generation.wrapping_add(1);
        if self.highlight.task.is_some() {
            return; // the running task restarts the debounce when stale
        }
        let task = cx.spawn(async move |entity, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(150))
                    .await;
                let Ok((text, generation)) = entity.update(cx, |editor, _cx| {
                    (editor.text.materialize(), editor.highlight.generation)
                }) else {
                    return;
                };
                let computed = cx
                    .background_executor()
                    .spawn(async move {
                        let outline = crate::outline::extract_outline_headings(&text);
                        let rope = Rope::new(&text);
                        let mut folds = FoldMap::discover_markdown_folds(&rope);
                        folds.sort_by_key(|range| range.start_row);
                        (outline, folds)
                    })
                    .await;
                let Ok(done) = entity.update(cx, |editor, cx| {
                    if editor.highlight.generation == generation {
                        editor.highlight.outline = Some(Arc::new(computed.0));
                        editor.highlight.folds = Some(Arc::new(computed.1));
                        editor.highlight.task = None;
                        cx.notify();
                        true
                    } else {
                        false
                    }
                }) else {
                    return;
                };
                if done {
                    break;
                }
            }
        });
        self.highlight.task = Some(task);
    }

    /// Highlight spans bucketed per buffer line, from the document's
    /// highlight engine (stale-while-typing, like Zed's async
    /// highlighting). The per-line index is cached by highlight version.
    pub fn highlight_spans_by_line(&self) -> Option<Arc<Vec<Vec<CodeHighlightSpan>>>> {
        let highlights = self.highlights.as_ref()?;
        if let Some((version, cached)) = self.spans_by_line_cache.borrow().as_ref() {
            if *version == highlights.version {
                return Some(cached.clone());
            }
        }
        let by_line = Arc::new(index_spans_by_line(&self.text, &highlights.spans));
        *self.spans_by_line_cache.borrow_mut() = Some((highlights.version, by_line.clone()));
        Some(by_line)
    }

    /// Cached outline headings from the last background refresh. Empty
    /// until the first background highlight completes (the editor never
    /// extracts the outline synchronously on the UI thread).
    pub fn cached_outline_headings(&self) -> Arc<Vec<OutlineNode>> {
        self.highlight.outline.clone().unwrap_or_default()
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
            // Fold chevrons need a gutter column even without line numbers.
            20.0
        }
    }

    // ── Matching bracket cache ───────────────────────────────────────────

    /// Finds the matching bracket offset for the primary cursor, cached
    /// per (text_version, cursor).
    pub fn matching_bracket(&mut self) -> Option<usize> {
        let cursor = self.cursor();
        if let Some((version, cached_cursor, result)) = &self.bracket_cache {
            if *version == self.text_version && *cached_cursor == cursor {
                return *result;
            }
        }
        let text = self.text.materialize();
        let result = crate::syntax::find_matching_bracket(&text, cursor);
        self.bracket_cache = Some((self.text_version, cursor, result));
        result
    }
}

fn hash_str(text: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// Splits flat highlight spans into per-buffer-line buckets (document
/// coordinates preserved). A visual row then looks up its spans in O(1)
/// instead of scanning the whole span list, and wrap segments filter them
/// by range. Multi-line spans (e.g. fenced code) are split per line.
fn index_spans_by_line(rope: &Rope, spans: &[CodeHighlightSpan]) -> Vec<Vec<CodeHighlightSpan>> {
    let mut by_line: Vec<Vec<CodeHighlightSpan>> = vec![Vec::new(); rope.line_count()];
    for span in spans {
        if span.range.start >= span.range.end {
            continue;
        }
        let (start_row, _) = rope.offset_to_point(span.range.start);
        let (end_row, _) = rope.offset_to_point(span.range.end - 1);
        for row in start_row..=end_row.min(by_line.len() - 1) {
            let line_start = rope.line_start(row);
            let line_end = line_start + rope.line_len(row);
            let start = span.range.start.max(line_start);
            let end = span.range.end.min(line_end);
            if start < end {
                by_line[row].push(CodeHighlightSpan {
                    range: start..end,
                    class: span.class,
                });
            }
        }
    }
    by_line
}

#[cfg(test)]
mod tests {
    use super::{Rope, index_spans_by_line};
    use syntax_highlighter::highlight::{CodeHighlightClass, highlight_code_block};

    #[test]
    fn span_index_buckets_per_line_within_line_bounds() {
        let text = "aa **bb** cc\n```rust\nfn main() {}\n```\nlast";
        let rope = Rope::new(text);
        let result = highlight_code_block(Some("markdown"), text).expect("highlight");
        let by_line = index_spans_by_line(&rope, &result.spans);
        assert_eq!(by_line.len(), rope.line_count());
        // Every bucketed span stays within its line's bounds.
        for (row, spans) in by_line.iter().enumerate() {
            let line_start = rope.line_start(row);
            let line_end = line_start + rope.line_len(row);
            for span in spans {
                assert!(
                    span.range.start >= line_start && span.range.end <= line_end,
                    "row {row}: {:?}",
                    span.range
                );
            }
        }
        // The bold span on row 0 is bucketed to row 0.
        assert!(
            by_line[0]
                .iter()
                .any(|span| span.class == CodeHighlightClass::MarkupBold)
        );
        // The rust keyword lands on the fence-content row.
        assert!(
            by_line[2]
                .iter()
                .any(|span| span.class == CodeHighlightClass::Keyword)
        );
    }
}
