use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::edit::CursorHint;
use crate::highlight::HighlightSnapshot;
use crate::text::Rope;

pub use platform_contracts::DocumentId;

/// Immutable document state shared with every pane implementation.
///
/// The editor owns the mutable source of truth. Panes consume this snapshot
/// and submit edits through their host; they must not independently infer file
/// or resource context from process globals.
///
/// `rope` is the structured text editors edit against (shared per revision,
/// O(1) clone); `text` is the lazily materialized full-text mirror for
/// full-text consumers (wysiwyg re-serialization, preview, search).
#[derive(Clone, Debug)]
pub struct DocumentSnapshot {
    pub id: DocumentId,
    pub revision: u64,
    pub text: Arc<str>,
    pub rope: Arc<Rope>,
    pub path: Option<Arc<Path>>,
    pub base_dir: Option<Arc<Path>>,
    /// Set when this revision was produced by undo/redo: the caret the pane
    /// should move to while syncing. Regular edits carry `None`.
    pub restore_cursor: Option<CursorHint>,
    /// Latest computed highlights; may lag a few revisions while the
    /// background refresh runs (stale-while-revalidate).
    pub highlights: Option<Arc<HighlightSnapshot>>,
}

impl DocumentSnapshot {
    pub fn new(
        id: DocumentId,
        revision: u64,
        rope: impl Into<Arc<Rope>>,
        path: Option<PathBuf>,
    ) -> Self {
        let rope: Arc<Rope> = rope.into();
        let text: Arc<str> = Arc::from(rope.materialize());
        Self::with_restore_cursor(id, revision, rope, text, path, None)
    }

    pub fn with_restore_cursor(
        id: DocumentId,
        revision: u64,
        rope: impl Into<Arc<Rope>>,
        text: impl Into<Arc<str>>,
        path: Option<PathBuf>,
        restore_cursor: Option<CursorHint>,
    ) -> Self {
        Self::with_all(id, revision, rope, text, path, restore_cursor, None)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_all(
        id: DocumentId,
        revision: u64,
        rope: impl Into<Arc<Rope>>,
        text: impl Into<Arc<str>>,
        path: Option<PathBuf>,
        restore_cursor: Option<CursorHint>,
        highlights: Option<Arc<HighlightSnapshot>>,
    ) -> Self {
        let path = path.map(Arc::<Path>::from);
        let base_dir = path
            .as_deref()
            .and_then(Path::parent)
            .map(Arc::<Path>::from);
        Self {
            id,
            revision,
            text: text.into(),
            rope: rope.into(),
            path,
            base_dir,
            restore_cursor,
            highlights,
        }
    }

    pub fn empty() -> Self {
        Self::new(DocumentId::nil(), 0, Rope::new(""), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_resource_base_from_document_path() {
        let snapshot = DocumentSnapshot::new(
            DocumentId::new(),
            7,
            Rope::new("![image](asset.png)"),
            Some(PathBuf::from("workspace/docs/readme.md")),
        );

        assert_eq!(
            snapshot.base_dir.as_deref(),
            Some(Path::new("workspace/docs"))
        );
        assert_eq!(snapshot.revision, 7);
    }

    #[test]
    fn empty_snapshot_has_stable_identity() {
        assert_eq!(DocumentSnapshot::empty().id, DocumentSnapshot::empty().id);
        assert_eq!(DocumentSnapshot::empty().rope.len(), 0);
    }
}
