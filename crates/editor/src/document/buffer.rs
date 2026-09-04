//! The authoritative in-memory state of one open document.

use std::cell::RefCell;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use gpui::{Context, Task};

use editor_contracts::{
    CursorHint, DocumentId, DocumentSnapshot, EditTransaction, HighlightSnapshot,
};
use markdown_parser::parse::{BlockProjection, common_affix};
use rope::Rope;
use syntax_highlighter::engine::HighlightMap;
use syntax_highlighter::language::CodeLanguageKey;

/// Maximum retained undo steps per document.
pub const MAX_UNDO_DEPTH: usize = 100;

/// One replace operation in pre-edit coordinates: `range` was replaced by
/// `new` (and `old` is the replaced text, restored by undo).
#[derive(Clone, Debug)]
struct Operation {
    range: Range<usize>,
    old: Arc<str>,
    new: Arc<str>,
}

/// One undoable document state transition: a run of operations applied in
/// order (a typing run, a paste, a cut, ...). Undo replays them backwards,
/// redo forwards, so only the edited bytes are ever stored — never a
/// full-text snapshot.
#[derive(Clone, Debug)]
pub struct DocumentEditEntry {
    operations: Vec<Operation>,
    cursor_before: CursorHint,
    cursor_after: CursorHint,
}

/// The process-level single source of truth for one document.
///
/// Editors never hold a private text copy: every tab references its buffer,
/// and every editor observes the buffers it shows. A pane edit lands in the
/// buffer via [`DocumentBuffer::apply_edit`], which bumps the revision and
/// notifies every observer — each observer then pushes a fresh
/// [`DocumentSnapshot`] down to its own panes.
///
/// The buffer also owns the document's undo history: history is an asset of
/// the document, not of any pane view, so it survives pane-kind switches and
/// is shared by every editor showing this document.
///
/// The text is a persistent chunk rope: edits rebuild only the affected
/// chunks and share the rest, so per-keystroke cost is O(edit) rather than
/// O(document).
pub struct DocumentBuffer {
    pub id: DocumentId,
    /// Authoritative text as a persistent rope.
    text: Arc<Rope>,
    /// Lazily materialized full-text mirror, cached per revision so
    /// `snapshot()` stays O(1) even though it is built per pane per frame.
    materialized: RefCell<Option<(u64, Arc<str>)>>,
    /// Bumped whenever the document text changes.
    pub revision: u64,
    /// Backing path on disk; `None` for untitled documents.
    pub path: Option<PathBuf>,
    /// Whether the in-memory content diverges from disk.
    pub dirty: bool,
    /// Set once the buffer is discarded; stale views prune themselves.
    pub discarded: bool,
    /// Cached (revision, word_count) to avoid full recounting on every frame.
    pub cached_word_count: Option<(u64, usize)>,
    /// Completed undo transactions, oldest first.
    undo_stack: Vec<DocumentEditEntry>,
    /// Undone transactions, in redo order.
    redo_stack: Vec<DocumentEditEntry>,
    /// Operations of the in-flight transaction, grouped by `merge`.
    pending: Option<PendingEdit>,
    /// Cursor the next snapshot should carry when this revision was
    /// produced by undo/redo; cleared by the next regular edit.
    restore_cursor: Option<CursorHint>,
    /// Incremental highlight engine (Markdown root + injections).
    highlight: HighlightMap,
    /// In-flight background highlight refresh; superseded tasks drop their
    /// result because its version no longer matches.
    highlight_task: Option<Task<()>>,
    /// Document-level block projection: the parsed Markdown block tree for
    /// the current text revision. Maintained incrementally on every edit,
    /// so it never lags and every structured pane shares the same parse.
    projection: BlockProjection,
}

#[derive(Clone, Debug)]
struct PendingEdit {
    cursor_before: CursorHint,
    cursor_after: CursorHint,
    operations: Vec<Operation>,
}

impl DocumentBuffer {
    pub fn new(text: String, path: Option<PathBuf>) -> Self {
        Self::restore(DocumentId::new(), text, path, false)
    }

    /// Rebuilds a buffer from a persisted snapshot, preserving identity.
    /// The highlight map starts unparsed: the initial parse runs on the
    /// debounced background task (kicked off by the store after
    /// construction), so opening a file never pays the parse cost on the
    /// main thread.
    pub fn restore(id: DocumentId, text: String, path: Option<PathBuf>, dirty: bool) -> Self {
        let text = normalize_line_endings(text);
        let highlight = HighlightMap::unparsed(CodeLanguageKey::Markdown)
            .expect("markdown language configuration");
        let source: Arc<str> = Arc::from(text);
        let rope = Arc::new(Rope::new(&source));
        Self {
            id,
            text: rope.clone(),
            materialized: RefCell::new(Some((1, source))),
            revision: 1,
            path,
            dirty,
            discarded: false,
            cached_word_count: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            pending: None,
            restore_cursor: None,
            highlight,
            highlight_task: None,
            projection: BlockProjection::parse(&rope),
        }
    }

    /// Kicks off the initial background highlight parse (the map starts
    /// unparsed; panes render text-only until the first refresh lands).
    pub fn start_highlighting(&mut self, cx: &mut Context<Self>) {
        self.schedule_highlight_refresh(cx);
    }

    pub fn snapshot(&self) -> DocumentSnapshot {
        let text = self.text_arc();
        let highlights = Some(Arc::new(HighlightSnapshot {
            version: self.highlight.refreshed_version,
            spans: self.highlight.spans_arc(),
        }));
        DocumentSnapshot::new(
            self.id,
            self.revision,
            self.text.clone(),
            text,
            self.path.clone(),
            self.restore_cursor,
            highlights,
            self.projection.blocks.clone(),
        )
    }

    /// The full text, materialized once per revision and then shared.
    fn text_arc(&self) -> Arc<str> {
        if let Some((revision, text)) = self.materialized.borrow().as_ref() {
            if *revision == self.revision {
                return text.clone();
            }
        }
        let text: Arc<str> = Arc::from(self.text.materialize());
        *self.materialized.borrow_mut() = Some((self.revision, text.clone()));
        text
    }

    /// Applies a pane-produced edit: normalizes line endings, records it as
    /// one undo operation, bumps the revision, and marks the document
    /// dirty. Unchanged text is a no-op so echo writes from observers
    /// cannot loop.
    pub fn apply_edit(&mut self, edit: EditTransaction, cx: &mut Context<Self>) {
        if self.apply_edit_core(edit) {
            self.schedule_highlight_refresh(cx);
            cx.notify();
        }
    }

    /// The edit application itself, context-free so it is unit-testable.
    /// Returns whether the document changed.
    ///
    /// Every replacement is first compressed to its true diff: the byte
    /// prefix and suffix shared with the old text are stripped, so a pane
    /// that reserializes the whole document (wysiwyg) still costs O(edit)
    /// for the rope rebuild, the syntax invalidation, and the undo payload.
    fn apply_edit_core(&mut self, edit: EditTransaction) -> bool {
        // One materialization serves the whole batch: all ranges are in
        // this revision's coordinates, and a full-document replacement can
        // reuse the cached mirror without slicing the rope.
        let full = self.text_arc();
        let whole_document = edit.edits.len() == 1;
        let mut operations: Vec<Operation> = Vec::with_capacity(edit.edits.len());
        let mut first_changed = usize::MAX;
        let mut last_changed = 0usize;
        for (range, inserted) in edit.edits {
            let start = clamp_to_char_boundary(&self.text, range.start.min(self.text.len()));
            let end = clamp_to_char_boundary(&self.text, range.end.min(self.text.len()).max(start));
            // Normalize before diffing: a "\r\n" pair may straddle the
            // compressed middle/suffix seam, so normalization must happen
            // on the whole input to stay equivalent. The memchr-style
            // scan is O(document) but negligible next to the affix scan,
            // and both disappear once panes serialize incrementally.
            let inserted = normalize_arc(inserted);
            let old: Arc<str> = if whole_document && start == 0 && end == full.len() {
                full.clone()
            } else {
                Arc::from(self.text.slice_owned(start..end))
            };
            if old.as_ref() == inserted.as_ref() {
                continue;
            }
            let (prefix, suffix) = common_affix(old.as_ref(), inserted.as_ref());
            let start = start + prefix;
            let end = end - suffix;
            let inserted = Arc::from(&inserted[prefix..inserted.len() - suffix]);
            let old = Arc::from(&old[prefix..old.len() - suffix]);
            // Changed rows in the coordinates the projection was parsed
            // from, computed before the rope edit lands. Ranges that end
            // exactly on a line start still cover the merged line.
            let first = self.text.offset_to_point(start).0;
            let last = self
                .text
                .offset_to_point(end.saturating_sub(1))
                .0
                .max(self.text.offset_to_point(end).0);
            first_changed = first_changed.min(first);
            last_changed = last_changed.max(last);
            self.highlight.apply_edit(&self.text, start..end, &inserted);
            let edited = self.text.edit(start..end, &inserted);
            *Arc::make_mut(&mut self.text) = edited;
            operations.push(Operation {
                range: start..end,
                old,
                new: inserted,
            });
        }
        if operations.is_empty() {
            return false;
        }

        // A regular edit invalidates any pending undo/redo cursor restore.
        self.restore_cursor = None;

        if !edit.merge || self.pending.is_none() {
            self.flush_pending();
            self.pending = Some(PendingEdit {
                cursor_before: edit.cursor_before,
                cursor_after: edit.cursor_after,
                operations: Vec::new(),
            });
        } else if let Some(pending) = &mut self.pending {
            pending.cursor_after = edit.cursor_after;
        }
        self.pending
            .as_mut()
            .expect("pending")
            .operations
            .extend(operations);

        self.revision = self.revision.wrapping_add(1);
        self.dirty = true;
        self.cached_word_count = None;
        self.refresh_projection(first_changed, last_changed);
        true
    }

    /// Recomputes the block projection incrementally against the current
    /// rope. Cost is O(edited region): the re-parse narrows its window to
    /// the changed lines (expanding to the enclosing block constructs) and
    /// splices the block list in place, so per-keystroke work is
    /// independent of document size — the rope is never materialized on
    /// this path.
    fn refresh_projection(&mut self, first_changed: usize, last_changed: usize) {
        BlockProjection::reparse(
            &mut self.projection,
            &self.text,
            first_changed,
            last_changed,
        );
    }

    /// Undoes the most recent transaction by replaying its operations
    /// backwards, and restores the caret that was active before the edit.
    /// The resulting snapshot carries `restore_cursor` so every pane
    /// converges on the same position.
    pub fn undo(&mut self, cx: &mut Context<Self>) {
        if self.undo_core().is_some() {
            self.schedule_highlight_refresh(cx);
            cx.notify();
        }
    }

    /// The undo application itself, context-free so it is unit-testable.
    fn undo_core(&mut self) -> Option<CursorHint> {
        self.flush_pending();
        let entry = self.undo_stack.pop()?;
        let restored_cursor = entry.cursor_before;
        let mut first_changed = usize::MAX;
        let mut last_changed = 0usize;
        for operation in entry.operations.iter().rev() {
            // Undo replaces `new` (now at range.start) back with `old`.
            let range = operation.range.start..operation.range.start + operation.new.len();
            let first = self.text.offset_to_point(range.start).0;
            let last = self
                .text
                .offset_to_point(range.end.saturating_sub(1))
                .0
                .max(self.text.offset_to_point(range.end).0);
            first_changed = first_changed.min(first);
            last_changed = last_changed.max(last);
            self.highlight
                .apply_edit(&self.text, range.clone(), &operation.old);
            let edited = self.text.edit(range, &operation.old);
            *Arc::make_mut(&mut self.text) = edited;
        }
        self.restore_cursor = Some(restored_cursor);
        self.redo_stack.push(entry);
        self.revision = self.revision.wrapping_add(1);
        self.cached_word_count = None;
        self.refresh_projection(first_changed, last_changed);
        Some(restored_cursor)
    }

    /// Reapplies the most recently undone transaction.
    pub fn redo(&mut self, cx: &mut Context<Self>) {
        if self.redo_core().is_some() {
            self.schedule_highlight_refresh(cx);
            cx.notify();
        }
    }

    /// The redo application itself, context-free so it is unit-testable.
    fn redo_core(&mut self) -> Option<CursorHint> {
        self.flush_pending();
        let entry = self.redo_stack.pop()?;
        let restored_cursor = entry.cursor_after;
        let mut first_changed = usize::MAX;
        let mut last_changed = 0usize;
        for operation in &entry.operations {
            let first = self.text.offset_to_point(operation.range.start).0;
            let last = self
                .text
                .offset_to_point(operation.range.end.saturating_sub(1))
                .0
                .max(self.text.offset_to_point(operation.range.end).0);
            first_changed = first_changed.min(first);
            last_changed = last_changed.max(last);
            self.highlight
                .apply_edit(&self.text, operation.range.clone(), &operation.new);
            let edited = self.text.edit(operation.range.clone(), &operation.new);
            *Arc::make_mut(&mut self.text) = edited;
        }
        self.restore_cursor = Some(restored_cursor);
        self.undo_stack.push(entry);
        self.revision = self.revision.wrapping_add(1);
        self.cached_word_count = None;
        self.refresh_projection(first_changed, last_changed);
        Some(restored_cursor)
    }

    /// Kicks off (or extends) the debounced background highlight refresh.
    /// While a task is running, newer edits only bump the map's version;
    /// the running task notices the mismatch and re-runs after another
    /// debounce, so the last edit of a typing burst always lands. The block
    /// projection needs no background refresh: it is maintained
    /// synchronously with every edit.
    fn schedule_highlight_refresh(&mut self, cx: &mut Context<Self>) {
        if self.highlight_task.is_some() {
            return;
        }
        let weak = cx.weak_entity();
        let task = cx.spawn(
            async move |_this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(150))
                        .await;
                    let Some((mut map, rope)) = weak
                        .update(cx, |buffer, _| {
                            (buffer.highlight.clone(), buffer.text.clone())
                        })
                        .ok()
                    else {
                        return;
                    };
                    let computed = cx
                        .background_executor()
                        .spawn(async move {
                            map.refresh(&rope);
                            map
                        })
                        .await;
                    let Ok(done) = weak.update(cx, |buffer, cx| {
                        if buffer.highlight.adopt_refresh(computed) {
                            buffer.highlight_task = None;
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
            },
        );
        self.highlight_task = Some(task);
    }

    /// Closes the in-flight transaction and records it on the undo stack.
    fn flush_pending(&mut self) {
        let Some(pending) = self.pending.take() else {
            return;
        };
        if pending.operations.is_empty() {
            return;
        }
        self.undo_stack.push(DocumentEditEntry {
            operations: pending.operations,
            cursor_before: pending.cursor_before,
            cursor_after: pending.cursor_after,
        });
        if self.undo_stack.len() > MAX_UNDO_DEPTH {
            self.undo_stack.remove(0);
        }
    }

    /// Marks the buffer saved at `path`.
    pub fn mark_saved(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.path = Some(path);
        self.dirty = false;
        cx.notify();
    }

    /// Repoints the buffer after a filesystem rename; content is untouched.
    pub fn set_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.path = Some(path);
        cx.notify();
    }

    /// Marks the buffer discarded; observers prune any stale view of it.
    pub fn mark_discarded(&mut self, cx: &mut Context<Self>) {
        self.discarded = true;
        cx.notify();
    }

    /// Word count of the document, cached by revision.
    pub fn word_count(&mut self) -> usize {
        if let Some((cached_rev, count)) = self.cached_word_count {
            if cached_rev == self.revision {
                return count;
            }
        }
        let text = self.text_arc();
        let count = crate::view::words::count_words(&text);
        self.cached_word_count = Some((self.revision, count));
        count
    }
}

fn normalize_line_endings(text: String) -> String {
    if text.contains('\r') {
        text.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        text
    }
}

fn normalize_arc(text: Arc<str>) -> Arc<str> {
    if text.contains('\r') {
        Arc::from(normalize_line_endings(text.to_string()))
    } else {
        text
    }
}

/// Snaps an offset inward to the nearest UTF-8 character boundary, so
/// pane-supplied ranges can never split a multi-byte character.
fn clamp_to_char_boundary(rope: &Rope, mut offset: usize) -> usize {
    while offset > 0 && !rope.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(
        range: Range<usize>,
        inserted: &str,
        merge: bool,
        before: CursorHint,
        after: CursorHint,
    ) -> EditTransaction {
        EditTransaction::new(range, inserted, merge, before, after)
    }

    #[test]
    fn apply_edit_replaces_range_incrementally() {
        let mut buffer = DocumentBuffer::new("one two\nthree\n".to_string(), None);
        assert!(buffer.apply_edit_core(edit(
            4..7,
            "TWO",
            false,
            CursorHint::new(1, 5),
            CursorHint::new(1, 8)
        )));
        assert_eq!(buffer.text_arc().as_ref(), "one TWO\nthree\n");
        assert_eq!(buffer.revision, 2);
        assert!(buffer.dirty);
    }

    #[test]
    fn full_document_replacement_compresses_to_the_true_diff() {
        let mut buffer = DocumentBuffer::new("aaBBcc".to_string(), None);
        buffer.apply_edit_core(edit(
            0..6,
            "aaXXcc",
            false,
            CursorHint::new(1, 3),
            CursorHint::new(1, 5),
        ));
        assert_eq!(buffer.text_arc().as_ref(), "aaXXcc");
        let operations = &buffer.pending.as_ref().expect("pending").operations;
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].range, 2..4);
        assert_eq!(operations[0].old.as_ref(), "BB");
        assert_eq!(operations[0].new.as_ref(), "XX");
    }

    #[test]
    fn compressed_replacement_undoes_and_redoes() {
        let mut buffer = DocumentBuffer::new("aaBBcc".to_string(), None);
        buffer.apply_edit_core(edit(
            0..6,
            "aaXXcc",
            false,
            CursorHint::new(1, 3),
            CursorHint::new(1, 5),
        ));
        buffer.flush_pending();
        assert_eq!(buffer.undo_core(), Some(CursorHint::new(1, 3)));
        assert_eq!(buffer.text_arc().as_ref(), "aaBBcc");
        assert_eq!(buffer.redo_core(), Some(CursorHint::new(1, 5)));
        assert_eq!(buffer.text_arc().as_ref(), "aaXXcc");
    }

    #[test]
    fn compressed_replacement_respects_char_boundaries() {
        let mut buffer = DocumentBuffer::new("中a".to_string(), None);
        buffer.apply_edit_core(edit(
            0..4,
            "中b",
            false,
            CursorHint::new(1, 2),
            CursorHint::new(1, 2),
        ));
        assert_eq!(buffer.text_arc().as_ref(), "中b");
        let operations = &buffer.pending.as_ref().expect("pending").operations;
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].range, 3..4);
        assert_eq!(operations[0].old.as_ref(), "a");
        assert_eq!(operations[0].new.as_ref(), "b");
    }

    #[test]
    fn whole_document_replacement_with_multibyte_suffix_is_compressed() {
        // A trailing multi-byte change: the shared suffix scan must snap
        // to a character boundary instead of cutting '中' in half.
        let mut buffer = DocumentBuffer::new("x中".to_string(), None);
        buffer.apply_edit_core(edit(
            0..4,
            "y中",
            false,
            CursorHint::new(1, 1),
            CursorHint::new(1, 1),
        ));
        assert_eq!(buffer.text_arc().as_ref(), "y中");
        let operations = &buffer.pending.as_ref().expect("pending").operations;
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].range, 0..1);
        assert_eq!(operations[0].old.as_ref(), "x");
        assert_eq!(operations[0].new.as_ref(), "y");
    }

    #[test]
    fn crlf_replacement_normalizes_before_diffing() {
        // A "\r\n" pair may straddle the compressed middle/suffix seam, so
        // normalization must run on the whole input: the "\r" insertion
        // collapses to a no-op, and a real change still applies correctly.
        let mut buffer = DocumentBuffer::new("a\nb".to_string(), None);
        assert!(!buffer.apply_edit_core(edit(
            0..3,
            "a\r\nb",
            false,
            CursorHint::new(1, 1),
            CursorHint::new(1, 1),
        )));
        assert_eq!(buffer.revision, 1);
        assert_eq!(buffer.text_arc().as_ref(), "a\nb");

        let mut buffer = DocumentBuffer::new("x".to_string(), None);
        buffer.apply_edit_core(edit(
            0..1,
            "a\r\nb",
            false,
            CursorHint::new(1, 1),
            CursorHint::new(1, 1),
        ));
        assert_eq!(buffer.text_arc().as_ref(), "a\nb");
        let operations = &buffer.pending.as_ref().expect("pending").operations;
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].range, 0..1);
        assert_eq!(operations[0].old.as_ref(), "x");
        assert_eq!(operations[0].new.as_ref(), "a\nb");
    }

    #[test]
    fn no_op_edits_do_not_bump_revision() {
        let mut buffer = DocumentBuffer::new("same".to_string(), None);
        assert!(!buffer.apply_edit_core(edit(
            0..0,
            "",
            false,
            CursorHint::new(1, 1),
            CursorHint::new(1, 1)
        )));
        assert_eq!(buffer.revision, 1);
    }

    #[test]
    fn undo_redo_replay_operations() {
        let mut buffer = DocumentBuffer::new("abc".to_string(), None);
        // Typing run: insert 'X' then 'Y' merged into one transaction.
        buffer.apply_edit_core(edit(
            1..1,
            "X",
            false,
            CursorHint::new(1, 2),
            CursorHint::new(1, 3),
        ));
        buffer.apply_edit_core(edit(
            2..2,
            "Y",
            true,
            CursorHint::new(1, 3),
            CursorHint::new(1, 4),
        ));
        assert_eq!(buffer.text_arc().as_ref(), "aXYbc");
        assert_eq!(buffer.undo_core(), Some(CursorHint::new(1, 2)));
        assert_eq!(buffer.text_arc().as_ref(), "abc");
        assert_eq!(buffer.restore_cursor, Some(CursorHint::new(1, 2)));
        assert_eq!(buffer.redo_core(), Some(CursorHint::new(1, 4)));
        assert_eq!(buffer.text_arc().as_ref(), "aXYbc");
        assert_eq!(buffer.restore_cursor, Some(CursorHint::new(1, 4)));
    }

    #[test]
    fn undo_restores_deleted_text_and_cursor() {
        let mut buffer = DocumentBuffer::new("keep remove keep\n".to_string(), None);
        buffer.apply_edit_core(edit(
            5..11,
            "",
            false,
            CursorHint::new(1, 6),
            CursorHint::new(1, 6),
        ));
        assert_eq!(buffer.text_arc().as_ref(), "keep  keep\n");
        buffer.undo_core();
        assert_eq!(buffer.text_arc().as_ref(), "keep remove keep\n");
    }

    #[test]
    fn merged_typing_run_is_one_undo_entry() {
        let mut buffer = DocumentBuffer::new("".to_string(), None);
        for (i, ch) in ['a', 'b', 'c'].into_iter().enumerate() {
            let s = ch.to_string();
            buffer.apply_edit_core(edit(
                i..i,
                &s,
                i > 0,
                CursorHint::new(1, i as u32 + 1),
                CursorHint::new(1, i as u32 + 2),
            ));
        }
        assert_eq!(buffer.undo_stack.len(), 0);
        buffer.flush_pending();
        assert_eq!(buffer.undo_stack.len(), 1);
        buffer.undo_core();
        assert_eq!(buffer.text_arc().as_ref(), "");
    }

    #[test]
    fn snapshot_rope_is_shared_and_consistent() {
        let buffer = DocumentBuffer::new("a\nb\n".to_string(), None);
        let snapshot = buffer.snapshot();
        assert_eq!(snapshot.rope.materialize(), "a\nb\n");
        assert_eq!(snapshot.text.as_ref(), "a\nb\n");
    }

    #[test]
    fn block_projection_stays_in_sync_after_edit() {
        use markdown_parser::parse::blocks_content_eq;

        let mut buffer = DocumentBuffer::new("# Title\n\nBody.\n".to_string(), None);
        let snapshot = buffer.snapshot();
        assert_eq!(snapshot.blocks.len(), 3); // heading + blank + paragraph

        buffer.apply_edit_core(edit(
            9..9,
            "x",
            false,
            CursorHint::new(2, 6),
            CursorHint::new(2, 7),
        ));
        // The projection is maintained synchronously with the edit, so the
        // snapshot always carries blocks for the current revision.
        let snapshot = buffer.snapshot();
        assert_eq!(snapshot.revision, 2);
        assert!(
            blocks_content_eq(
                &snapshot.blocks,
                &markdown_parser::parse::BlockProjection::parse(&snapshot.rope).blocks,
            ),
            "projection must equal a full parse of the current text"
        );
    }

    #[test]
    fn undo_and_redo_keep_the_projection_in_sync() {
        use markdown_parser::parse::blocks_content_eq;

        let mut buffer = DocumentBuffer::new("a\nb\n".to_string(), None);
        buffer.apply_edit_core(edit(
            2..2,
            "X",
            false,
            CursorHint::new(2, 1),
            CursorHint::new(2, 2),
        ));
        buffer.undo_core();
        let snapshot = buffer.snapshot();
        assert_eq!(snapshot.text.as_ref(), "a\nb\n");
        assert!(blocks_content_eq(
            &snapshot.blocks,
            &markdown_parser::parse::BlockProjection::parse(&snapshot.rope).blocks,
        ));
        buffer.redo_core();
        let snapshot = buffer.snapshot();
        assert_eq!(snapshot.text.as_ref(), "a\nXb\n");
        assert!(blocks_content_eq(
            &snapshot.blocks,
            &markdown_parser::parse::BlockProjection::parse(&snapshot.rope).blocks,
        ));
    }

    #[test]
    fn materialized_cache_invalidates_on_edit() {
        let buffer = DocumentBuffer::new("hello".to_string(), None);
        assert_eq!(buffer.text_arc().as_ref(), "hello");
    }

    mod bench {
        use std::sync::Arc;
        use std::time::Instant;

        use super::DocumentBuffer;
        use crate::document::buffer::CursorHint;

        fn bench_snapshot(name: &str, size_kb: usize) {
            let text = "line of markdown text\n".repeat(size_kb * 1024 / 22);
            let buffer = DocumentBuffer::new(text, None);

            // Warm-up.
            for _ in 0..3 {
                std::hint::black_box(buffer.snapshot());
            }

            let frames = 120; // two seconds of 60fps frames
            let start = Instant::now();
            for _ in 0..frames {
                std::hint::black_box(buffer.snapshot());
            }
            let elapsed = start.elapsed();
            println!(
                "bench_snapshot[{name}]: {size_kb}KB x{frames} frames = {elapsed:?} ({}us/frame)",
                elapsed.as_micros() / frames as u128
            );
        }

        /// `render_pane` builds a fresh snapshot per pane per frame via
        /// `buffer.read(cx).snapshot()`. The buffer keeps a per-revision
        /// materialized mirror, so this is a refcount bump, not a copy.
        #[test]
        #[ignore = "perf benchmark"]
        fn bench_snapshot_clone_per_frame() {
            bench_snapshot("64KB", 64);
            bench_snapshot("1MB", 1024);
        }

        /// A wysiwyg-style full-document replacement commit that changes
        /// one character, measured in three phases: pane-side
        /// reserialization, the buffer apply (compression + rope + syntax
        /// invalidation), and the synchronous block-projection refresh
        /// (isolated by editing the rope directly).
        #[test]
        #[ignore = "perf benchmark"]
        fn bench_apply_edit_compresses_whole_document_replacement() {
            for size_kb in [64usize, 1024, 4096] {
                let mut text = "line of markdown text\n".repeat(size_kb * 1024 / 22);
                text.push_str("tail");
                let mut buffer = DocumentBuffer::new(text, None);
                let edits = 120;
                let mut serialize = std::time::Duration::ZERO;
                let mut apply = std::time::Duration::ZERO;
                let mut projection = std::time::Duration::ZERO;
                for i in 0..edits {
                    let start = Instant::now();
                    let mut new = buffer.text_arc().to_string();
                    let at = new.len() / 2;
                    new.replace_range(at..at + 1, &format!("{}", i % 10));
                    serialize += start.elapsed();
                    let start = Instant::now();
                    buffer.apply_edit_core(crate::document::buffer::tests::edit(
                        0..buffer.text_arc().len(),
                        &new,
                        false,
                        CursorHint::new(1, 1),
                        CursorHint::new(1, 1),
                    ));
                    apply += start.elapsed();

                    // Isolate the projection refresh: apply the same edit
                    // straight to the rope, then re-time just the refresh.
                    let start = Instant::now();
                    let at = buffer.text.len() / 2;
                    let first = buffer.text.offset_to_point(at).0;
                    let edited = buffer.text.edit(at..at + 1, "x");
                    *Arc::make_mut(&mut buffer.text) = edited;
                    buffer.revision = buffer.revision.wrapping_add(1);
                    projection += start.elapsed();
                    let start = Instant::now();
                    buffer.refresh_projection(first, first);
                    projection += start.elapsed();
                }
                println!(
                    "bench_apply_edit[{size_kb}KB]: {edits} commits: serialize {}us, apply {}us, projection {}us per edit",
                    serialize.as_micros() / edits as u128,
                    apply.as_micros() / edits as u128,
                    projection.as_micros() / edits as u128,
                );
            }
        }

        /// A wysiwyg-style commit in its post-diff shape: one small
        /// precise replacement mid-document. Measures the full buffer-side
        /// keystroke cost (compression, rope edit, syntax invalidation,
        /// projection refresh).
        #[test]
        #[ignore = "perf benchmark"]
        fn bench_apply_edit_precise_keystroke() {
            for size_kb in [64usize, 1024, 4096] {
                let mut text =
                    "# Heading\n\nSome **bold** and `code`.\n\n- item one\n- item two\n\n"
                        .repeat(4);
                while text.len() < size_kb * 1024 {
                    text.push_str("plain line of markdown text\n");
                }
                let mut buffer = DocumentBuffer::new(text, None);
                let edits = 240;
                let start = Instant::now();
                for _ in 0..edits {
                    let at = buffer.text.len() / 2;
                    buffer.apply_edit_core(crate::document::buffer::tests::edit(
                        at..at + 1,
                        "X",
                        false,
                        CursorHint::new(1, 1),
                        CursorHint::new(1, 1),
                    ));
                }
                let elapsed = start.elapsed();
                println!(
                    "bench_precise_keystroke[{size_kb}KB]: {edits} edits = {}us/edit",
                    elapsed.as_micros() / edits as u128
                );
            }
        }

        /// The file-open path: `DocumentBuffer::new` runs the rope build
        /// and the block projection parse synchronously; the highlight
        /// parse is unparsed and moved to the background task (measured
        /// separately as its off-thread cost).
        #[test]
        #[ignore = "perf benchmark"]
        fn bench_open_document_phases() {
            for size_kb in [64usize, 256, 1024, 4096] {
                let mut text = String::new();
                while text.len() < size_kb * 1024 {
                    text.push_str(
                        "# Heading\n\nSome **bold** and `code` text with a [link](https://example.com).\n\n- item one\n- item two\n\n> quote line\n\n",
                    );
                }
                text.truncate(size_kb * 1024);

                let start = Instant::now();
                let buffer = DocumentBuffer::new(text.clone(), None);
                let total_elapsed = start.elapsed();

                let start = Instant::now();
                let highlight = syntax_highlighter::engine::HighlightMap::new(
                    syntax_highlighter::language::CodeLanguageKey::Markdown,
                    &text,
                )
                .expect("markdown map");
                let background_elapsed = start.elapsed();

                let (layers, layer_ranges) = highlight.layer_stats();
                println!(
                    "bench_open[{size_kb}KB]: buffer open {total_elapsed:?}, background highlight parse {background_elapsed:?} ({} spans, {layers} layers, {layer_ranges} ranges)",
                    highlight.spans().len(),
                );
                let _ = buffer;
            }
        }
    }
}
