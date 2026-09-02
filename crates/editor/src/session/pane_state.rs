//! Pane view state: scroll anchoring and the live pane view instance.

use gpui::{Pixels, ScrollHandle, Size};

use crate::session::PaneKind;

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
