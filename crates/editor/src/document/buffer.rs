//! The authoritative in-memory state of one open document.

use std::path::PathBuf;
use std::sync::Arc;

use gpui::Context;

use editor_contracts::{CursorHint, DocumentId, DocumentSnapshot, EditTransaction};

/// Maximum retained undo steps per document.
pub const MAX_UNDO_DEPTH: usize = 100;

/// One undoable document state transition.
///
/// `before` / `after` hold full-text snapshots of the document around one
/// user-level transaction (a typing run, a paste, a cut, ...). Full-text
/// snapshots keep the model simple: Markdown documents are small, the depth
/// is bounded by [`MAX_UNDO_DEPTH`], and panes reserialize the whole
/// document on every commit anyway.
#[derive(Clone, Debug)]
pub struct DocumentEditEntry {
    pub before: Arc<str>,
    pub after: Arc<str>,
    pub cursor_before: CursorHint,
    pub cursor_after: CursorHint,
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
pub struct DocumentBuffer {
    pub id: DocumentId,
    /// Authoritative Markdown text.
    pub text: String,
    /// Cheap-to-clone mirror of `text`, so `snapshot()` stays O(1) even
    /// though it is built per pane per frame.
    text_arc: Arc<str>,
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
    /// Text snapshot at the start of the in-flight transaction, plus its
    /// bookkeeping; flushed into `undo_stack` by the next non-merged edit,
    /// undo, or redo.
    pending: Option<PendingEdit>,
    /// Cursor the next snapshot should carry when this revision was
    /// produced by undo/redo; cleared by the next regular edit.
    restore_cursor: Option<CursorHint>,
}

#[derive(Clone, Debug)]
struct PendingEdit {
    before: Arc<str>,
    cursor_before: CursorHint,
    cursor_after: CursorHint,
}

impl DocumentBuffer {
    pub fn new(text: String, path: Option<PathBuf>) -> Self {
        Self::restore(DocumentId::new(), text, path, false)
    }

    /// Rebuilds a buffer from a persisted snapshot, preserving identity.
    pub fn restore(id: DocumentId, text: String, path: Option<PathBuf>, dirty: bool) -> Self {
        let text = normalize_line_endings(text);
        Self {
            id,
            text_arc: Arc::from(text.clone()),
            text,
            revision: 1,
            path,
            dirty,
            discarded: false,
            cached_word_count: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            pending: None,
            restore_cursor: None,
        }
    }

    pub fn snapshot(&self) -> DocumentSnapshot {
        DocumentSnapshot::with_restore_cursor(
            self.id,
            self.revision,
            self.text_arc.clone(),
            self.path.clone(),
            self.restore_cursor,
        )
    }

    /// Replaces the authoritative text and refreshes the Arc mirror.
    fn set_text(&mut self, text: String) {
        self.text_arc = Arc::from(text.clone());
        self.text = text;
    }

    /// Applies a pane-produced edit: normalizes line endings, records it as
    /// one undo transaction, bumps the revision, and marks the document
    /// dirty. Unchanged text is a no-op so echo writes from observers
    /// cannot loop.
    pub fn apply_edit(&mut self, edit: EditTransaction, cx: &mut Context<Self>) {
        let text = normalize_line_endings(edit.text);
        if text == self.text {
            return;
        }

        // A regular edit invalidates any pending undo/redo cursor restore.
        self.restore_cursor = None;

        if !edit.merge || self.pending.is_none() {
            self.flush_pending();
            self.pending = Some(PendingEdit {
                before: Arc::from(self.text.clone()),
                cursor_before: edit.cursor_before,
                cursor_after: edit.cursor_after,
            });
        } else if let Some(pending) = &mut self.pending {
            pending.cursor_after = edit.cursor_after;
        }

        self.set_text(text);
        self.revision = self.revision.wrapping_add(1);
        self.dirty = true;
        self.cached_word_count = None;
        cx.notify();
    }

    /// Undoes the most recent transaction, restoring the document text that
    /// preceded it and the caret that was active before the edit. The
    /// resulting snapshot carries `restore_cursor` so every pane converges
    /// on the same position.
    pub fn undo(&mut self, cx: &mut Context<Self>) {
        self.flush_pending();
        let Some(entry) = self.undo_stack.pop() else {
            return;
        };
        let restored_cursor = entry.cursor_before;
        self.set_text(entry.before.to_string());
        self.restore_cursor = Some(restored_cursor);
        self.redo_stack.push(entry);
        self.finish_history_step(cx);
    }

    /// Reapplies the most recently undone transaction.
    pub fn redo(&mut self, cx: &mut Context<Self>) {
        self.flush_pending();
        let Some(entry) = self.redo_stack.pop() else {
            return;
        };
        let restored_cursor = entry.cursor_after;
        self.set_text(entry.after.to_string());
        self.restore_cursor = Some(restored_cursor);
        self.undo_stack.push(entry);
        self.finish_history_step(cx);
    }

    fn finish_history_step(&mut self, cx: &mut Context<Self>) {
        self.revision = self.revision.wrapping_add(1);
        self.cached_word_count = None;
        cx.notify();
    }

    /// Closes the in-flight transaction and records it on the undo stack.
    fn flush_pending(&mut self) {
        let Some(pending) = self.pending.take() else {
            return;
        };
        if pending.before.as_ref() == self.text.as_str() {
            return;
        }
        self.undo_stack.push(DocumentEditEntry {
            before: pending.before,
            after: Arc::from(self.text.clone()),
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
        let count = crate::view::words::count_words(&self.text);
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

#[cfg(test)]
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
    /// `buffer.read(cx).snapshot()`. The buffer keeps an `Arc<str>` mirror
    /// so this is a refcount bump, not a full-text copy.
    #[test]
    #[ignore = "perf benchmark"]
    fn bench_snapshot_clone_per_frame() {
        bench_snapshot("64KB", 64);
        bench_snapshot("1MB", 1024);
    }
}
