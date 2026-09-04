//! Process-global registry of open document buffers.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use gpui::{App, AppContext, Entity};

use editor_contracts::DocumentId;

use super::buffer::DocumentBuffer;

/// Durable projection of a buffer for window-state persistence.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PersistedDocument {
    pub id: DocumentId,
    pub text: String,
    pub path: Option<PathBuf>,
    pub dirty: bool,
}

/// The process-level document registry: which documents are open, which
/// buffer backs each one, and how many views reference each buffer.
///
/// Lifetime rules (VSCode semantics):
/// - Every tab acquires a view reference; every removed tab releases one.
/// - A clean buffer is dropped when its last view releases.
/// - A dirty buffer persists in the registry until it is saved or explicitly
///   discarded, so another editor opening the same path picks up the
///   in-memory content instead of stale disk bytes.
#[derive(Default)]
pub struct DocumentStore {
    buffers: HashMap<DocumentId, Entity<DocumentBuffer>>,
    path_index: HashMap<PathBuf, DocumentId>,
    refs: HashMap<DocumentId, usize>,
}

impl gpui::Global for DocumentStore {}

impl DocumentStore {
    /// Installs the store as a process global. Called once at startup.
    pub fn init(cx: &mut App) {
        cx.set_global(Self::default());
    }

    /// Registers a buffer for `text`/`path`. When a buffer for `path` is
    /// already registered, returns it — its in-memory content wins.
    pub fn create(text: String, path: Option<PathBuf>, cx: &mut App) -> Entity<DocumentBuffer> {
        if let Some(path) = &path {
            if let Some(existing) = cx.global::<Self>().resolve(path) {
                return existing;
            }
        }
        let buffer = cx.new(|_| DocumentBuffer::new(text, path.clone()));
        let id = buffer.read(cx).id;
        cx.global_mut::<Self>().insert(id, path, buffer.clone());
        buffer
    }

    /// Opens `path`: returns the registered buffer when the document is
    /// already open (dirty or not), otherwise loads the file from disk.
    pub fn open(path: &Path, cx: &mut App) -> Result<Entity<DocumentBuffer>, std::io::Error> {
        if let Some(existing) = cx.global::<Self>().resolve(path) {
            return Ok(existing);
        }
        let bytes = std::fs::read(path)?;
        let text = String::from_utf8_lossy(&bytes).to_string();
        let path_buf = path.to_path_buf();
        let buffer = cx.new(|_| DocumentBuffer::new(text, Some(path_buf.clone())));
        let id = buffer.read(cx).id;
        cx.global_mut::<Self>()
            .insert(id, Some(path_buf), buffer.clone());
        Ok(buffer)
    }

    /// The registered buffer for `path`, if any.
    pub fn resolve(&self, path: &Path) -> Option<Entity<DocumentBuffer>> {
        let id = self.path_index.get(path)?;
        self.buffers.get(id).cloned()
    }

    pub fn get(&self, id: DocumentId) -> Option<Entity<DocumentBuffer>> {
        self.buffers.get(&id).cloned()
    }

    /// Number of registered views of the buffer across the process.
    pub fn view_count(&self, id: DocumentId) -> usize {
        self.refs.get(&id).copied().unwrap_or(0)
    }

    /// Registers one view of the buffer.
    pub fn acquire(&mut self, id: DocumentId) {
        *self.refs.entry(id).or_insert(0) += 1;
    }

    /// Releases one view. When the last view releases, the buffer is removed
    /// unless `keep` (callers keep dirty buffers alive).
    pub fn release(&mut self, id: DocumentId, keep: bool) {
        let Some(count) = self.refs.get_mut(&id) else {
            return;
        };
        *count -= 1;
        if *count == 0 {
            self.refs.remove(&id);
            if !keep {
                self.remove(id);
            }
        }
    }

    /// Force-removes a discarded buffer from the registry.
    pub fn discard(&mut self, id: DocumentId) {
        self.refs.remove(&id);
        self.remove(id);
    }

    /// Repoints every buffer under `from` to `to` after a filesystem rename,
    /// keeping the path index consistent.
    pub fn rename_paths(from: &Path, to: &Path, cx: &mut App) {
        let affected: Vec<(PathBuf, PathBuf, DocumentId)> = {
            let store = cx.global::<DocumentStore>();
            store
                .path_index
                .iter()
                .filter(|(path, _)| path.as_path() == from || path.starts_with(from))
                .map(|(path, id)| {
                    let new_path = if path.as_path() == from {
                        to.to_path_buf()
                    } else {
                        path.strip_prefix(from)
                            .map(|relative| to.join(relative))
                            .unwrap_or_else(|_| path.clone())
                    };
                    (path.clone(), new_path, *id)
                })
                .collect()
        };

        for (_, new_path, id) in &affected {
            let Some(buffer) = cx.global::<DocumentStore>().get(*id) else {
                continue;
            };
            let new_path = new_path.clone();
            buffer.update(cx, |buffer, cx| buffer.set_path(new_path, cx));
        }

        let store = cx.global_mut::<DocumentStore>();
        for (old_path, new_path, id) in affected {
            store.path_index.remove(&old_path);
            store.path_index.insert(new_path, id);
        }
    }

    /// Ids of every dirty buffer, for close-guard aggregation.
    pub fn dirty_buffer_ids(&self, cx: &App) -> Vec<DocumentId> {
        self.buffers
            .values()
            .filter_map(|buffer| {
                let buffer = buffer.read(cx);
                buffer.dirty.then_some(buffer.id)
            })
            .collect()
    }

    /// The display name of the first dirty buffer, if any.
    pub fn first_dirty_name(&self, cx: &App) -> Option<String> {
        self.buffers.values().find_map(|buffer| {
            let buffer = buffer.read(cx);
            if !buffer.dirty {
                return None;
            }
            Some(
                buffer
                    .path
                    .as_ref()
                    .and_then(|path| path.file_name())
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Untitled".to_string()),
            )
        })
    }

    /// Durable snapshot of every registered buffer, for window-state
    /// persistence.
    pub fn persisted_snapshot(cx: &App) -> Vec<PersistedDocument> {
        cx.global::<Self>()
            .buffers
            .values()
            .map(|buffer| {
                let buffer = buffer.read(cx);
                PersistedDocument {
                    id: buffer.id,
                    text: buffer.snapshot().text.to_string(),
                    path: buffer.path.clone(),
                    dirty: buffer.dirty,
                }
            })
            .collect()
    }

    /// Rebuilds buffers from a persisted snapshot, preserving identities.
    pub fn restore(documents: Vec<PersistedDocument>, cx: &mut App) {
        for document in documents {
            let buffer = cx.new(|_| {
                DocumentBuffer::restore(
                    document.id,
                    document.text,
                    document.path.clone(),
                    document.dirty,
                )
            });
            cx.global_mut::<DocumentStore>()
                .insert(document.id, document.path, buffer);
        }
    }

    /// Re-indexes a buffer after its path changed (save-as). The previous
    /// entry is removed only when it still points at this buffer.
    pub fn update_path_index(
        &mut self,
        id: DocumentId,
        old_path: Option<&Path>,
        new_path: Option<PathBuf>,
    ) {
        if let Some(old_path) = old_path {
            if self.path_index.get(old_path) == Some(&id) {
                self.path_index.remove(old_path);
            }
        }
        if let Some(new_path) = new_path {
            self.path_index.insert(new_path, id);
        }
    }

    fn insert(&mut self, id: DocumentId, path: Option<PathBuf>, buffer: Entity<DocumentBuffer>) {
        if let Some(path) = path {
            self.path_index.insert(path, id);
        }
        self.buffers.insert(id, buffer);
    }

    fn remove(&mut self, id: DocumentId) {
        self.buffers.remove(&id);
        self.path_index.retain(|_, buffer_id| *buffer_id != id);
    }
}
