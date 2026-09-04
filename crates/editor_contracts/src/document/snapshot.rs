use std::path::{Path, PathBuf};
use std::sync::Arc;

use markdown_parser::parse::BlockData;

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
/// O(1) clone); `text` is the materialized full-text mirror for full-text
/// consumers (search, export).
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
    /// Document-level block projection for this revision, shared by every
    /// structured (wysiwyg) pane. The buffer maintains it incrementally on
    /// each edit, so it never lags behind the text.
    pub blocks: Arc<Vec<BlockData>>,
}

impl DocumentSnapshot {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: DocumentId,
        revision: u64,
        rope: impl Into<Arc<Rope>>,
        text: impl Into<Arc<str>>,
        path: Option<PathBuf>,
        restore_cursor: Option<CursorHint>,
        highlights: Option<Arc<HighlightSnapshot>>,
        blocks: Arc<Vec<BlockData>>,
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
            blocks,
        }
    }

    pub fn empty() -> Self {
        Self::new(
            DocumentId::nil(),
            0,
            Rope::new(""),
            "",
            None,
            None,
            None,
            Arc::new(Vec::new()),
        )
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
            "![image](asset.png)",
            Some(PathBuf::from("workspace/docs/readme.md")),
            None,
            None,
            Arc::new(Vec::new()),
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
