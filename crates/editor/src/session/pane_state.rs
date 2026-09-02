//! Pane view state: scroll anchoring and the per-kind pane instances.
//!
//! One [`PaneState`] lives per split leaf. It keeps one live pane instance
//! per kind (WYSIWYG / Source Code / Preview / plugins) and switches
//! between them without destroying any: cursors, folds, IME state, search
//! highlights, and block trees all survive kind switches, mirroring how tab
//! switches already preserve pane state. Only the active instance is
//! rendered, receives input, and participates in document sync; inactive
//! instances stay dormant and catch up the moment they are activated.

use std::collections::HashMap;

use gpui::{App, Pixels, Point, ScrollHandle, Size, point, px};

use crate::session::PaneKind;
use editor_contracts::{DocumentSnapshot, PaneView};

/// The independent view state of one pane inside an editor area.
pub struct PaneState {
    pub scroll: ScrollState,
    /// One live pane instance per kind, keyed by kind.
    panes: HashMap<PaneKind, Box<dyn PaneView>>,
    active_kind: PaneKind,
    /// Scroll offset per kind, saved when switching away so each kind's
    /// viewport position is restored when switching back.
    scroll_offsets: HashMap<PaneKind, Point<Pixels>>,
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
        let mut panes = HashMap::new();
        panes.insert(kind.clone(), new_pane_for_kind(kind.clone()));
        Self {
            scroll: ScrollState::default(),
            panes,
            active_kind: kind,
            scroll_offsets: HashMap::new(),
        }
    }

    /// The active pane instance.
    pub fn pane(&self) -> &dyn PaneView {
        &**self
            .panes
            .get(&self.active_kind)
            .expect("the active pane instance must exist")
    }

    /// The active pane instance, mutably.
    pub fn pane_mut(&mut self) -> &mut dyn PaneView {
        &mut **self
            .panes
            .get_mut(&self.active_kind)
            .expect("the active pane instance must exist")
    }

    /// Switches to `kind`: reuses the existing instance or lazily creates
    /// one, anchoring the outgoing kind's scroll offset and restoring the
    /// incoming kind's. Returns whether a switch happened.
    pub fn ensure_kind(&mut self, kind: PaneKind) -> bool {
        if self.active_kind == kind {
            return false;
        }
        self.scroll_offsets
            .insert(self.active_kind.clone(), self.scroll.handle.offset());
        self.panes
            .entry(kind.clone())
            .or_insert_with(|| new_pane_for_kind(kind.clone()));
        self.active_kind = kind;
        let restored = self
            .scroll_offsets
            .get(&self.active_kind)
            .copied()
            .unwrap_or_else(|| point(px(0.0), px(0.0)));
        self.scroll.handle.set_offset(restored);
        true
    }

    /// Synchronizes only the active pane with a document snapshot.
    /// Inactive instances stay dormant and catch up when activated: the
    /// render path syncs the active pane every frame, so activation and
    /// catch-up happen in the same pass.
    pub fn sync_active(&mut self, document: &DocumentSnapshot, cx: &mut App) {
        self.pane_mut().sync_document(document, cx);
    }
}

/// Creates a pane instance for a registered kind.
fn new_pane_for_kind(kind: PaneKind) -> Box<dyn PaneView> {
    editor_contracts::PaneRegistry::create_registered(kind.clone())
        .unwrap_or_else(|error| panic!("failed to access pane registry: {error}"))
        .unwrap_or_else(|| panic!("no pane descriptor registered for {kind}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::Any;

    use editor_contracts::{PaneRenderContext, PaneView};
    use gpui::IntoElement;

    struct StubPane {
        kind: PaneKind,
    }

    impl StubPane {
        fn boxed(kind: &str) -> Box<dyn PaneView> {
            Box::new(Self {
                kind: PaneKind::new(kind),
            })
        }
    }

    impl PaneView for StubPane {
        fn kind(&self) -> PaneKind {
            self.kind.clone()
        }

        fn sync_document(&mut self, _document: &DocumentSnapshot, _cx: &mut App) {}

        fn document_text(&self, _cx: &App) -> Option<String> {
            None
        }

        fn render(
            &mut self,
            _ctx: &PaneRenderContext,
            _window: &mut gpui::Window,
            _cx: &mut App,
        ) -> gpui::AnyElement {
            gpui::div().into_any_element()
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    fn state_with_two_kinds() -> PaneState {
        let kind_a = PaneKind::new("stub.pane.a");
        let kind_b = PaneKind::new("stub.pane.b");
        let mut panes = HashMap::new();
        panes.insert(kind_a.clone(), StubPane::boxed("stub.pane.a"));
        panes.insert(kind_b.clone(), StubPane::boxed("stub.pane.b"));
        PaneState {
            scroll: ScrollState::default(),
            panes,
            active_kind: kind_a,
            scroll_offsets: HashMap::new(),
        }
    }

    #[test]
    fn ensure_kind_reuses_instances() {
        let kind_a = PaneKind::new("stub.pane.a");
        let kind_b = PaneKind::new("stub.pane.b");
        let mut state = state_with_two_kinds();

        assert_eq!(state.pane().kind(), kind_a);

        // Switching activates the existing instance; no new pane is built.
        assert!(state.ensure_kind(kind_b.clone()));
        assert_eq!(state.pane().kind(), kind_b);

        // Same-kind ensure is a no-op.
        assert!(!state.ensure_kind(kind_b.clone()));

        // Switching back reactivates the original instance.
        assert!(state.ensure_kind(kind_a.clone()));
        assert_eq!(state.pane().kind(), kind_a);
    }

    #[test]
    fn ensure_kind_anchors_scroll_per_kind() {
        let kind_a = PaneKind::new("stub.pane.a");
        let kind_b = PaneKind::new("stub.pane.b");
        let mut state = state_with_two_kinds();

        state.scroll.handle.set_offset(point(px(0.0), px(-120.0)));
        state.ensure_kind(kind_b.clone());

        // A never-seen kind starts at the top.
        assert_eq!(state.scroll.handle.offset(), point(px(0.0), px(0.0)));
        state.scroll.handle.set_offset(point(px(0.0), px(-240.0)));

        // Switching back restores the outgoing kind's offset.
        state.ensure_kind(kind_a.clone());
        assert_eq!(state.scroll.handle.offset(), point(px(0.0), px(-120.0)));
        state.ensure_kind(kind_b);
        assert_eq!(state.scroll.handle.offset(), point(px(0.0), px(-240.0)));
    }
}
