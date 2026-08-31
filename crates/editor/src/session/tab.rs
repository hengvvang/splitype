//! Document tab and pane view state models.

use std::collections::HashMap;
use std::time::Instant;

use gpui::{App, Pixels, ScrollHandle, Size, Task};

use crate::session::file::FileState;
use crate::session::{PaneKind, TabKind};
use core_contracts::{AutoscrollStrategy, DocumentId, DocumentSnapshot};

/// One document tab: the authoritative raw text and all document-level metadata.
pub struct DocumentTab {
    /// Stable identity shared by every pane projection of this document.
    pub id: DocumentId,
    /// Authoritative Markdown text — the single raw source of truth.
    pub text: String,
    /// Bumped whenever the document text changes.
    pub document_revision: u64,
    pub file: FileState,
    pub kind: TabKind,
    /// Per-pane view states, keyed by pane id.
    pub panes: HashMap<core_contracts::PaneId, PaneState>,
    /// Cached (revision, word_count) to avoid full recounting on every frame.
    pub cached_word_count: Option<(u64, usize)>,
}

/// The independent view state of one pane inside an editor area.
pub struct PaneState {
    pub scroll: ScrollState,
    pub pane: Box<dyn core_contracts::PaneView>,
}

/// Scroll handle, layout anchoring, and autoscroll interaction state.
pub struct ScrollState {
    pub handle: ScrollHandle,
    pub pending_autoscroll: Option<AutoscrollStrategy>,
    pub last_viewport_size: Option<Size<Pixels>>,
    pub scrollbar_hovered: bool,
    pub scrollbar_visible_until: Instant,
    pub scrollbar_fade_task: Option<Task<()>>,
    pub scrollbar_drag: Option<ScrollbarDragSession>,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self {
            handle: ScrollHandle::new(),
            pending_autoscroll: None,
            last_viewport_size: None,
            scrollbar_hovered: false,
            scrollbar_visible_until: Instant::now(),
            scrollbar_fade_task: None,
            scrollbar_drag: None,
        }
    }
}

/// Scrollbar drag session tracking.
#[derive(Clone, Copy, Debug)]
pub struct ScrollbarDragSession {
    pub pointer_offset_y: f32,
    pub track_height: f32,
    pub thumb_height: f32,
    pub max_scroll_y: f32,
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

pub fn new_pane_for_kind(kind: PaneKind) -> Box<dyn core_contracts::PaneView> {
    core_contracts::PaneRegistry::create_registered(kind.clone())
        .unwrap_or_else(|error| panic!("failed to access pane registry: {error}"))
        .unwrap_or_else(|| panic!("no pane descriptor registered for {kind}"))
}
