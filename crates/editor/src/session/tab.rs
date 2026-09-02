//! Document tab and pane view state models.

use std::collections::HashMap;
use std::path::PathBuf;

use gpui::{Entity, EntityId, Pixels, ScrollHandle, Size};

use crate::document::DocumentBuffer;
use crate::session::{PaneKind, TabKind};
use editor_contracts::DocumentId;

/// A link-navigation request deferred until a `Window` is available.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PendingOpenLink {
    pub prompt_target: String,
    pub open_target: String,
}

/// Transient per-tab view bookkeeping for save/close/drop flows.
///
/// Nothing here is durable: the authoritative document state lives in the
/// tab's [`DocumentBuffer`]; these flags only drive in-flight dialogs and
/// the next-frame window-chrome refresh.
#[derive(Default)]
pub struct TabPendingState {
    pub pending_save: bool,
    pub pending_save_as: bool,
    pub pending_open_link: Option<PendingOpenLink>,
    pub window_edited: bool,
    pub window_title_refresh: bool,
    pub show_unsaved_changes_dialog: bool,
    pub pending_close_after_save: bool,
    pub close_dialog_restore_focus: Option<EntityId>,
    pub drop_replace_path: Option<PathBuf>,
    pub show_drop_replace_dialog: bool,
    pub drop_replace_after_save: bool,
    pub drop_replace_restore_focus: Option<EntityId>,
}

/// One document tab: a shallow view reference to a shared buffer plus the
/// pane projections and transient UI state of this editor view.
pub struct DocumentTab {
    /// The shared document source of truth.
    pub buffer: Entity<DocumentBuffer>,
    pub kind: TabKind,
    /// Per-pane view states, keyed by pane id (rebuilt from the text on restore).
    pub panes: HashMap<editor_contracts::PaneId, PaneState>,
    pub pending: TabPendingState,
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
    pub fn new(buffer: Entity<DocumentBuffer>, kind: TabKind) -> Self {
        Self {
            buffer,
            kind,
            panes: HashMap::new(),
            pending: TabPendingState::default(),
        }
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

/// Durable tab projection: a buffer identity plus the tab kind.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PersistedTab {
    pub buffer: DocumentId,
    pub kind: TabKind,
}
