//! Document tab model: a shallow view reference to a shared buffer.

use std::collections::HashMap;
use std::path::PathBuf;

use gpui::{Entity, EntityId};

use crate::document::DocumentBuffer;
use crate::session::{PaneState, TabKind};
use editor_contracts::DocumentId;

/// Transient per-tab view bookkeeping for save/close/drop flows.
///
/// Nothing here is durable: the authoritative document state lives in the
/// tab's [`DocumentBuffer`]; these flags only drive in-flight dialogs and
/// the next-frame window-chrome refresh.
#[derive(Default)]
pub struct TabPendingState {
    pub pending_save: bool,
    pub pending_save_as: bool,
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
    /// Per-pane view states, keyed by pane id.
    pub panes: HashMap<editor_contracts::PaneId, PaneState>,
    pub pending: TabPendingState,
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

/// Durable tab projection: a buffer identity plus the tab kind.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PersistedTab {
    pub buffer: DocumentId,
    pub kind: TabKind,
}
