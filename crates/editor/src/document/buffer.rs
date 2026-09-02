//! The authoritative in-memory state of one open document.

use std::path::PathBuf;

use gpui::Context;

use editor_contracts::{DocumentId, DocumentSnapshot};

/// The process-level single source of truth for one document.
///
/// Editors never hold a private text copy: every tab references its buffer,
/// and every editor observes the buffers it shows. A pane edit lands in the
/// buffer via [`DocumentBuffer::set_text`], which bumps the revision and
/// notifies every observer — each observer then pushes a fresh
/// [`DocumentSnapshot`] down to its own panes.
pub struct DocumentBuffer {
    pub id: DocumentId,
    /// Authoritative Markdown text.
    pub text: String,
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
}

impl DocumentBuffer {
    pub fn new(text: String, path: Option<PathBuf>) -> Self {
        Self::restore(DocumentId::new(), text, path, false)
    }

    /// Rebuilds a buffer from a persisted snapshot, preserving identity.
    pub fn restore(id: DocumentId, text: String, path: Option<PathBuf>, dirty: bool) -> Self {
        Self {
            id,
            text: normalize_line_endings(text),
            revision: 1,
            path,
            dirty,
            discarded: false,
            cached_word_count: None,
        }
    }

    pub fn snapshot(&self) -> DocumentSnapshot {
        DocumentSnapshot::new(self.id, self.revision, self.text.clone(), self.path.clone())
    }

    /// Applies a pane-produced edit: normalizes line endings, bumps the
    /// revision, and marks the document dirty. Unchanged text is a no-op so
    /// echo writes from observers cannot loop.
    pub fn set_text(&mut self, text: String, cx: &mut Context<Self>) {
        let text = normalize_line_endings(text);
        if text == self.text {
            return;
        }
        self.text = text;
        self.revision = self.revision.wrapping_add(1);
        self.dirty = true;
        self.cached_word_count = None;
        cx.notify();
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
