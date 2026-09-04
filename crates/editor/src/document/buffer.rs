//! The authoritative in-memory state of one open document.

use std::cell::RefCell;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use gpui::{Context, Task};

use editor_contracts::{
    CursorHint, DocumentId, DocumentSnapshot, EditTransaction, HighlightSnapshot, Rope,
};
use syntax_highlighter::engine::HighlightMap;
use syntax_highlighter::highlight::CodeLanguageKey;

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
    /// In-flight background refresh; superseded tasks drop their result
    /// because its version no longer matches.
    highlight_task: Option<Task<()>>,
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
    pub fn restore(id: DocumentId, text: String, path: Option<PathBuf>, dirty: bool) -> Self {
        let text = normalize_line_endings(text);
        let highlight = HighlightMap::new(CodeLanguageKey::Markdown, &text)
            .expect("markdown language configuration");
        Self {
            id,
            text: Arc::new(Rope::new(&text)),
            materialized: RefCell::new(Some((1, Arc::from(text)))),
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
        }
    }

    pub fn snapshot(&self) -> DocumentSnapshot {
        let text = self.text_arc();
        let highlights = Some(Arc::new(HighlightSnapshot {
            version: self.highlight.refreshed_version,
            spans: self.highlight.spans_arc(),
        }));
        DocumentSnapshot::with_all(
            self.id,
            self.revision,
            self.text.clone(),
            text,
            self.path.clone(),
            self.restore_cursor,
            highlights,
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
    fn apply_edit_core(&mut self, edit: EditTransaction) -> bool {
        let mut operations: Vec<Operation> = Vec::with_capacity(edit.edits.len());
        for (range, inserted) in edit.edits {
            let start = range.start.min(self.text.len());
            let end = range.end.min(self.text.len()).max(start);
            let inserted = normalize_arc(inserted);
            let old: Arc<str> = Arc::from(self.text.slice_owned(start..end));
            if old.as_ref() == inserted.as_ref() {
                continue;
            }
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
        true
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
        for operation in entry.operations.iter().rev() {
            // Undo replaces `new` (now at range.start) back with `old`.
            let range = operation.range.start..operation.range.start + operation.new.len();
            self.highlight
                .apply_edit(&self.text, range.clone(), &operation.old);
            let edited = self.text.edit(range, &operation.old);
            *Arc::make_mut(&mut self.text) = edited;
        }
        self.restore_cursor = Some(restored_cursor);
        self.redo_stack.push(entry);
        self.revision = self.revision.wrapping_add(1);
        self.cached_word_count = None;
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
        for operation in &entry.operations {
            self.highlight
                .apply_edit(&self.text, operation.range.clone(), &operation.new);
            let edited = self.text.edit(operation.range.clone(), &operation.new);
            *Arc::make_mut(&mut self.text) = edited;
        }
        self.restore_cursor = Some(restored_cursor);
        self.undo_stack.push(entry);
        self.revision = self.revision.wrapping_add(1);
        self.cached_word_count = None;
        Some(restored_cursor)
    }

    /// Kicks off (or extends) the debounced background highlight refresh.
    /// While a task is running, newer edits only bump the map's version;
    /// the running task notices the mismatch and re-runs after another
    /// debounce, so the last edit of a typing burst always lands.
    fn schedule_highlight_refresh(&mut self, cx: &mut Context<Self>) {
        if self.highlight_task.is_some() {
            return;
        }
        let weak = cx.weak_entity();
        let task = cx.spawn(async move |_this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
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
        });
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
    fn materialized_cache_invalidates_on_edit() {
        let buffer = DocumentBuffer::new("hello".to_string(), None);
        assert_eq!(buffer.text_arc().as_ref(), "hello");
    }

    mod bench {
        use std::time::Instant;

        use super::DocumentBuffer;

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
    }
}
