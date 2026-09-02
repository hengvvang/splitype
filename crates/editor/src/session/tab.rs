//! Document tab and pane view state models.

use std::collections::HashMap;

use gpui::{App, Pixels, ScrollHandle, Size};

use crate::session::file::FileState;
use crate::session::{PaneKind, TabKind};
use editor_contracts::{DocumentId, DocumentSnapshot};

/// One document tab: the authoritative raw text and all document-level metadata.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct DocumentTab {
    /// Stable identity shared by every pane projection of this document.
    pub id: DocumentId,
    /// Authoritative Markdown text — the single raw source of truth.
    pub text: String,
    /// Bumped whenever the document text changes.
    pub document_revision: u64,
    pub file: FileState,
    pub kind: TabKind,
    /// Per-pane view states, keyed by pane id (rebuilt from the text on restore).
    #[serde(skip)]
    pub panes: HashMap<editor_contracts::PaneId, PaneState>,
    /// Cached (revision, word_count) to avoid full recounting on every frame.
    pub cached_word_count: Option<(u64, usize)>,
}

/// The independent view state of one pane inside an editor area.
pub struct PaneState {
    pub scroll: ScrollState,
    pub pane: Box<dyn editor_contracts::PaneView>,
}

/// Scroll handle, layout anchoring, and viewport tracking state.
pub struct ScrollState {
    pub handle: ScrollHandle,
    pub last_viewport_size: Option<Size<Pixels>>,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self {
            handle: ScrollHandle::new(),
            last_viewport_size: None,
        }
    }
}

impl DocumentTab {
    pub fn snapshot(&self) -> DocumentSnapshot {
        DocumentSnapshot::new(
            self.id,
            self.document_revision,
            self.text.clone(),
            self.file.path.clone(),
        )
    }

    #[inline]
    pub fn serialized_text(&self, _cx: &App) -> String {
        self.text.clone()
    }

    #[inline]
    pub fn is_transient(&self) -> bool {
        self.kind == TabKind::Transient
    }

    #[inline]
    pub fn persist(&mut self) {
        self.kind = TabKind::Persistent;
    }
}

impl PaneState {
    pub fn new(kind: PaneKind) -> Self {
        Self {
            scroll: ScrollState::default(),
            pane: new_pane_for_kind(kind),
        }
    }

    pub fn kind(&self) -> PaneKind {
        self.pane.kind()
    }

    pub fn ensure_kind(&mut self, kind: PaneKind) {
        if self.kind() == kind {
            return;
        }
        self.pane = new_pane_for_kind(kind);
    }
}

pub fn new_pane_for_kind(kind: PaneKind) -> Box<dyn editor_contracts::PaneView> {
    editor_contracts::PaneRegistry::create_registered(kind.clone())
        .unwrap_or_else(|error| panic!("failed to access pane registry: {error}"))
        .unwrap_or_else(|| panic!("no pane descriptor registered for {kind}"))
}
