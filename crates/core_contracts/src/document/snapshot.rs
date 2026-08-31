use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Stable identity of a document during its lifetime in the host.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocumentId(Uuid);

impl DocumentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub const fn nil() -> Self {
        Self(Uuid::nil())
    }
}

impl Default for DocumentId {
    fn default() -> Self {
        Self::new()
    }
}

/// Immutable document state shared with every pane implementation.
///
/// The editor owns the mutable source of truth. Panes consume this snapshot
/// and submit edits through their host; they must not independently infer file
/// or resource context from process globals.
#[derive(Clone, Debug)]
pub struct DocumentSnapshot {
    pub id: DocumentId,
    pub revision: u64,
    pub text: Arc<str>,
    pub path: Option<Arc<Path>>,
    pub base_dir: Option<Arc<Path>>,
}

impl DocumentSnapshot {
    pub fn new(
        id: DocumentId,
        revision: u64,
        text: impl Into<Arc<str>>,
        path: Option<PathBuf>,
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
            path,
            base_dir,
        }
    }

    pub fn empty() -> Self {
        Self::new(DocumentId::nil(), 0, Arc::<str>::from(""), None)
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
            "![image](asset.png)",
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
    }
}
