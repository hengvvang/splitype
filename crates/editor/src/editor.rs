//! Top-level editor aggregate root.
//!
//! [`Editor`] aggregates the editor's own view state: the document tab views,
//! view mode, scroll state, focus management, and the editor's pane views
//! (WYSIWYG, Source Code, Preview, and custom plugins).
//!
//! The authoritative raw text lives in the process-level
//! [`crate::document::DocumentBuffer`]; every editor tab is a shallow view
//! reference to a shared buffer. `Editor` observes the buffers it shows and
//! pushes fresh snapshots down to its panes. It does zero AST parsing or
//! serialization itself — all syntax trees and viewport specifics live
//! strictly inside each pane plugin implementation.

pub mod export;
pub mod pane_host;
pub mod search_host;

use std::collections::HashMap;
use std::sync::Arc;

use gpui::*;

use editor_contracts::{
    DocumentHost, DocumentSnapshot, EditTransaction, OutlineHudState, PaneId, PaneKind, TabKind,
};
use platform_contracts::PanelId;

use crate::document::{DocumentBuffer, DocumentStore};
use crate::editor::pane_host::EditorPaneHost;
use crate::session::{DocumentTab, EditorSession, EditorTabList, PaneState, ScrollState};

/// The Editor aggregate root entity.
pub struct Editor {
    pub panel_id: PanelId,
    pub entity_id: EntityId,
    pub host: Option<Arc<dyn DocumentHost>>,
    pub pane_host: Arc<dyn editor_contracts::PaneHost>,
    pub session: EditorSession,
    pub panel_rect: Option<Bounds<Pixels>>,
    pub is_active_panel: bool,
    pub is_maximized: bool,
    pub leaf_count: usize,
    pub outline: OutlineHudState,
    /// Outline open/closed state per pane kind within this editor.
    /// Panes of the same kind within this editor share this setting;
    /// different kinds are independent; different editors are independent.
    pub outline_enabled_by_kind: HashMap<PaneKind, bool>,
    pub focused_pane_id: Option<PaneId>,
    pub search: editor_contracts::SearchPanelState,
    /// Observer subscriptions per shared buffer, keyed by buffer id.
    pub buffer_subscriptions: HashMap<editor_contracts::DocumentId, Subscription>,
    /// Set once this panel released all of its document views.
    pub documents_released: bool,
    /// Last known render width per pane id. Preserves pane layout width across tab switches
    /// so newly mounted tabs do not suffer from a 0-width initial frame offset shift.
    pub last_pane_widths: HashMap<PaneId, f32>,
}

impl Editor {
    /// Creates an Editor initialized with an existing session (e.g. restored
    /// or suspended), observing every buffer the session references.
    pub fn with_session(panel_id: PanelId, session: EditorSession, cx: &mut Context<Self>) -> Self {
        let mut editor = Self {
            panel_id,
            entity_id: cx.entity().entity_id(),
            host: None,
            pane_host: EditorPaneHost::new(cx.weak_entity()),
            session,
            panel_rect: None,
            is_active_panel: false,
            is_maximized: false,
            leaf_count: 1,
            outline: OutlineHudState::default(),
            outline_enabled_by_kind: HashMap::new(),
            focused_pane_id: None,
            search: editor_contracts::SearchPanelState::new(cx),
            buffer_subscriptions: HashMap::new(),
            documents_released: false,
            last_pane_widths: HashMap::new(),
        };
        let buffers: Vec<Entity<DocumentBuffer>> = editor
            .session
            .tabs()
            .map(|tab| tab.buffer.clone())
            .collect();
        for buffer in buffers {
            editor.observe_buffer(buffer, cx);
        }
        editor
    }

    // ------------------------------------------------------------------
    // Buffer observation and view registration
    // ------------------------------------------------------------------

    /// Subscribes to a buffer once; every change re-syncs this editor's
    /// panes and refreshes its window chrome.
    pub(crate) fn observe_buffer(
        &mut self,
        buffer: Entity<DocumentBuffer>,
        cx: &mut Context<Self>,
    ) {
        let id = buffer.read(cx).id;
        if self.buffer_subscriptions.contains_key(&id) {
            return;
        }
        let subscription = cx.observe(&buffer, Self::on_buffer_changed);
        self.buffer_subscriptions.insert(id, subscription);
    }

    /// Registers a new view of the buffer and subscribes to it.
    fn acquire_and_observe(&mut self, buffer: Entity<DocumentBuffer>, cx: &mut Context<Self>) {
        let id = buffer.read(cx).id;
        cx.global_mut::<DocumentStore>().acquire(id);
        self.observe_buffer(buffer, cx);
    }

    /// Pushes a new tab into the session, registering its buffer view.
    pub(crate) fn attach_tab(&mut self, mut tab: DocumentTab, cx: &mut Context<Self>) {
        let buffer = tab.buffer.clone();
        let snapshot = buffer.read(cx).snapshot();
        tab.is_markdown = snapshot.is_markdown();
        tab.snapshot = snapshot;
        self.acquire_and_observe(buffer, cx);
        self.session.push_tab(tab);
    }

    /// Releases the view registration of a removed tab and drops its buffer
    /// subscription when no other tab of this editor references it.
    fn detach_tab(&mut self, tab: &DocumentTab, cx: &mut Context<Self>) {
        let buffer = tab.buffer.clone();
        let (id, keep) = {
            let buffer = buffer.read(cx);
            (buffer.id, buffer.dirty)
        };
        cx.global_mut::<DocumentStore>().release(id, keep);
        if !self.session.tabs().any(|other| other.buffer == buffer) {
            self.buffer_subscriptions.remove(&id);
        }
    }

    /// Buffer change broadcast: syncs every pane of every tab referencing
    /// the buffer and refreshes window chrome. Pane-level revision guards
    /// make re-syncing the originating pane a harmless no-op.
    fn on_buffer_changed(&mut self, buffer: Entity<DocumentBuffer>, cx: &mut Context<Self>) {
        if buffer.read(cx).discarded {
            let indices: Vec<usize> = self
                .session
                .tabs()
                .enumerate()
                .filter(|(_, tab)| tab.buffer == buffer)
                .map(|(index, _)| index)
                .collect();
            for index in indices.into_iter().rev() {
                self.close_tab(index, cx);
            }
            cx.notify();
            return;
        }
        let is_markdown = buffer.read(cx).is_markdown();
        let document = buffer.read(cx).snapshot();
        let mut leaf_ids = Vec::new();
        self.session.root.tree.leaf_ids(&mut leaf_ids);
        let leaf_kinds: Vec<(PaneId, PaneKind)> = leaf_ids
            .into_iter()
            .filter_map(|id| {
                self.session
                    .root
                    .tree
                    .find_leaf_kind(id)
                    .map(|kind| (PaneId::from(id), kind))
            })
            .collect();

        for tab in self.session.tabs_mut() {
            if tab.buffer == buffer {
                tab.is_markdown = is_markdown;
                tab.snapshot = document.clone();
                tab.pending.window_title_refresh = true;
                tab.pending.window_edited = true;
                for (pane_id, configured) in &leaf_kinds {
                    let effective = Self::resolve_effective_pane_kind(configured, Some(&document));
                    if let Some(state) = tab.panes.get_mut(pane_id) {
                        state.ensure_kind(effective);
                    }
                }
                for state in tab.panes.values_mut() {
                    state.sync_active(&document, cx);
                }
            }
        }
        cx.notify();
    }

    // ------------------------------------------------------------------
    // Document text access and commits
    // ------------------------------------------------------------------

    /// Synchronizes all panes of the active tab with the current buffer.
    pub fn sync_panes_with_active_tab(&mut self, cx: &mut Context<Self>) {
        let Some(buffer) = self.session.active_tab().map(|tab| tab.buffer.clone()) else {
            return;
        };
        let is_markdown = buffer.read(cx).is_markdown();
        let document = buffer.read(cx).snapshot();
        let mut leaf_ids = Vec::new();
        self.session.root.tree.leaf_ids(&mut leaf_ids);
        let leaf_kinds: Vec<(PaneId, PaneKind)> = leaf_ids
            .into_iter()
            .filter_map(|id| {
                self.session
                    .root
                    .tree
                    .find_leaf_kind(id)
                    .map(|kind| (PaneId::from(id), kind))
            })
            .collect();

        if let Some(tab_mut) = self.session.active_tab_mut() {
            tab_mut.is_markdown = is_markdown;
            tab_mut.snapshot = document.clone();
            for (pane_id, configured) in leaf_kinds {
                let effective = Self::resolve_effective_pane_kind(&configured, Some(&document));
                if let Some(state) = tab_mut.panes.get_mut(&pane_id) {
                    state.ensure_kind(effective);
                }
            }
            for state in tab_mut.panes.values_mut() {
                state.sync_active(&document, cx);
            }
        }
    }

    /// Commits a pane-produced edit into the shared buffer; observers
    /// (including this editor) broadcast the new snapshot to every pane.
    pub fn commit_document_edit(&mut self, edit: EditTransaction, cx: &mut Context<Self>) {
        if !self.session.has_tabs() {
            // No open tab: there is nothing to edit — ignore pane-driven
            // input instead of implicitly creating a tab.
            return;
        }
        let Some(buffer) = self.session.active_tab().map(|tab| tab.buffer.clone()) else {
            return;
        };
        buffer.update(cx, |buffer, cx| buffer.apply_edit(edit, cx));
        cx.notify();
    }

    /// Activates the tab at `index`, restoring its focus and window chrome.
    pub fn activate_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.session.tab_count() {
            return;
        }
        self.session.set_active_tab(index);
        if let Some(tab) = self.session.tab_mut(index) {
            tab.pending.window_title_refresh = true;
            tab.pending.window_edited = true;
        }
        self.sync_panes_with_active_tab(cx);
        if self.search.visible {
            self.execute_search(cx);
        }
        cx.notify();
    }

    /// Opens a file in this editor's tab list: activates its tab if the
    /// shared buffer is already shown here, otherwise opens the document
    /// through the store (reusing the in-memory buffer when it exists).
    pub fn open_file_in_panel(
        &mut self,
        path: &std::path::Path,
        kind: TabKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let buffer = match DocumentStore::open(path, cx) {
            Ok(buffer) => buffer,
            Err(err) => {
                self.show_drop_open_failed_prompt(
                    format!("failed to read '{}': {err}", path.display()),
                    window,
                    cx,
                );
                return;
            }
        };
        if let Some(index) = self
            .session
            .tab_list
            .iter()
            .position(|tab| tab.buffer == buffer)
        {
            if kind == TabKind::Persistent {
                if let Some(tab) = self.session.tab_mut(index) {
                    tab.persist();
                }
            }
            self.activate_tab(index, cx);
            return;
        }

        let snapshot = buffer.read(cx).snapshot();
        let tab = DocumentTab::new(buffer.clone(), kind).with_snapshot(snapshot);
        if kind == TabKind::Transient {
            let clean_transient_idx = self
                .session
                .tab_list
                .iter()
                .position(|tab| tab.is_transient() && !tab.buffer.read(cx).dirty);
            if let Some(index) = clean_transient_idx {
                self.acquire_and_observe(buffer, cx);
                let old = self
                    .session
                    .tab_list
                    .replace(index, tab)
                    .expect("just checked");
                self.detach_tab(&old, cx);
                self.activate_tab(index, cx);
                self.record_recent_file(path, cx);
                return;
            }
        }

        self.attach_tab(tab, cx);
        self.activate_tab(self.session.tab_count() - 1, cx);
        self.record_recent_file(path, cx);
    }

    fn record_recent_file(&self, path: &std::path::Path, cx: &mut Context<Self>) {
        if let Some(host) = &self.host {
            host.record_recent_file(path, cx);
        }
    }

    pub fn new_untitled_tab(&mut self, cx: &mut Context<Self>) {
        let buffer = DocumentStore::create(String::new(), None, cx);
        self.attach_tab(DocumentTab::new(buffer, TabKind::Persistent), cx);
        self.activate_tab(self.session.tab_count() - 1, cx);
    }

    pub fn request_close_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(tab) = self.session.tab(index) else {
            return;
        };
        let buffer = tab.buffer.clone();
        let (id, dirty) = {
            let buffer = buffer.read(cx);
            (buffer.id, buffer.dirty)
        };
        if dirty && cx.global::<DocumentStore>().view_count(id) == 1 {
            let panel_id = self.panel_id;
            self.activate_tab(index, cx);
            self.defer_host_action(cx, move |host, cx| {
                host.prompt_close_tab(panel_id, index, cx);
            });
            return;
        }
        self.close_tab(index, cx);
    }

    pub fn close_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(tab) = self.session.close_tab(index) else {
            return;
        };
        self.detach_tab(&tab, cx);
        if !self.session.has_tabs() {
            self.clear_search_highlights_from_document(cx);
            self.search.matches.clear();
            self.search.active_match_index = None;
            cx.notify();
            return;
        }
        if let Some(tab) = self.session.active_tab_mut() {
            tab.pending.window_title_refresh = true;
            tab.pending.window_edited = true;
        }
        if self.search.visible {
            self.execute_search(cx);
        }
        cx.notify();
    }

    /// Closes every tab view, releasing each buffer registration.
    pub fn clear_tabs(&mut self, cx: &mut Context<Self>) {
        for tab in self.session.tabs() {
            let buffer = tab.buffer.clone();
            let (id, keep) = {
                let buffer = buffer.read(cx);
                (buffer.id, buffer.dirty)
            };
            cx.global_mut::<DocumentStore>().release(id, keep);
        }
        self.buffer_subscriptions.clear();
        self.session.clear_tabs();
        cx.notify();
    }

    /// Discards the tab at `index` and closes it. The shared buffer is
    /// destroyed when this tab was its last view; otherwise only this
    /// panel's view is released.
    pub fn discard_tab_at(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(tab) = self.session.tab(index) {
            let buffer = tab.buffer.clone();
            let (id, dirty) = {
                let buffer = buffer.read(cx);
                (buffer.id, buffer.dirty)
            };
            if dirty && cx.global::<DocumentStore>().view_count(id) == 1 {
                buffer.update(cx, |buffer, cx| buffer.mark_discarded(cx));
                cx.global_mut::<DocumentStore>().discard(id);
            }
        }
        self.close_tab(index, cx);
    }

    /// Display name of the first dirty buffer in this panel, if any.
    pub fn first_dirty_title(&self, cx: &App) -> Option<String> {
        self.session.tabs().find_map(|tab| {
            let buffer = tab.buffer.read(cx);
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

    pub fn active_pane_id(&self) -> PaneId {
        if let Some(pane_id) = self.focused_pane_id {
            return pane_id;
        }
        self.session
            .root
            .tree
            .first_leaf_id()
            .map(PaneId::from)
            .unwrap_or_default()
    }

    pub fn active_pane_scroll(&mut self) -> &ScrollState {
        let active_id = self.active_pane_id();
        &self.pane_state(active_id).scroll
    }

    #[inline]
    pub fn default_pane_kind(&self) -> PaneKind {
        Self::registered_default_pane_kind()
    }

    #[inline]
    pub fn registered_default_pane_kind() -> PaneKind {
        editor_contracts::PaneRegistry::registered_default_kind()
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    #[inline]
    pub fn resolve_effective_pane(
        configured: &PaneKind,
        doc: Option<&DocumentSnapshot>,
    ) -> (PaneKind, bool) {
        if let Some(doc) = doc {
            if let Ok(Some(takeover)) =
                editor_contracts::PaneRegistry::resolve_takeover_global(configured, doc)
            {
                return (takeover.target_kind, takeover.read_only);
            }
        }
        (configured.clone(), false)
    }

    #[inline]
    pub fn resolve_effective_pane_kind(
        configured: &PaneKind,
        doc: Option<&DocumentSnapshot>,
    ) -> PaneKind {
        Self::resolve_effective_pane(configured, doc).0
    }

    #[inline]
    pub fn effective_pane_kind(
        &self,
        configured: &PaneKind,
        doc: Option<&DocumentSnapshot>,
    ) -> PaneKind {
        Self::resolve_effective_pane_kind(configured, doc)
    }

    pub fn pane_state(&mut self, pane_id: PaneId) -> &mut PaneState {
        let configured = self
            .pane_kind(pane_id)
            .unwrap_or_else(|| self.default_pane_kind());
        let doc = self.active_tab().map(|t| &t.snapshot);
        let effective = Self::resolve_effective_pane_kind(&configured, doc);
        let tab = self
            .session
            .active_tab_mut()
            .expect("pane state requires an open tab");
        let state = tab
            .panes
            .entry(pane_id)
            .or_insert_with(|| PaneState::new(effective.clone()));
        state.ensure_kind(effective);
        state
    }

    pub fn pane_state_mut(&mut self, pane_id: PaneId) -> Option<&mut PaneState> {
        let configured = self
            .pane_kind(pane_id)
            .unwrap_or_else(|| self.default_pane_kind());
        let doc = self.active_tab().map(|t| &t.snapshot);
        let effective = Self::resolve_effective_pane_kind(&configured, doc);
        let tab = self.session.active_tab_mut()?;
        let state = tab
            .panes
            .entry(pane_id)
            .or_insert_with(|| PaneState::new(effective.clone()));
        state.ensure_kind(effective);
        Some(state)
    }

    pub fn pane_state_ref(&self, pane_id: PaneId) -> Option<&PaneState> {
        let tab = self.active_tab()?;
        tab.panes.get(&pane_id)
    }

    pub fn defer_host_action(
        &self,
        cx: &mut Context<Self>,
        action: impl FnOnce(&dyn DocumentHost, &mut App) + 'static,
    ) {
        if let Some(host) = self.host.clone() {
            cx.defer(move |cx| {
                action(host.as_ref(), cx);
            });
        }
    }

    pub fn focus_pane(
        &mut self,
        pane_id: impl Into<PaneId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pane_id = pane_id.into();
        self.focused_pane_id = Some(pane_id);
        self.session.root.activate_leaf(pane_id.leaf_id());
        self.session.root.interaction.clear_dropdowns();
        let panel_id = self.panel_id;
        self.defer_host_action(cx, move |host, cx| host.activate_panel(panel_id, cx));
        if !self.has_tabs() {
            cx.notify();
            return;
        }
        if let Some(state) = self.pane_state_mut(pane_id) {
            if let Some(handle) = state.pane().focus_handle(cx) {
                handle.focus(window, cx);
            }
        }
        cx.notify();
    }

    #[inline]
    pub fn pane_kind(&self, pane_id: PaneId) -> Option<PaneKind> {
        self.session
            .root
            .tree
            .find_leaf(pane_id.0)
            .map(|l| l.kind.clone())
    }

    #[inline]
    pub fn active_pane_kind(&self) -> PaneKind {
        self.pane_kind(self.active_pane_id())
            .unwrap_or_else(|| self.default_pane_kind())
    }

    #[inline]
    pub fn active_tab(&self) -> Option<&DocumentTab> {
        self.session.active_tab()
    }

    #[inline]
    pub fn active_tab_mut(&mut self) -> Option<&mut DocumentTab> {
        self.session.active_tab_mut()
    }

    #[inline]
    pub fn has_tabs(&self) -> bool {
        self.session.has_tabs()
    }

    pub fn tab_list_mut(&mut self) -> &mut EditorTabList<DocumentTab> {
        &mut self.session.tab_list
    }

    #[inline]
    pub fn set_panel_id(&mut self, id: PanelId) {
        self.panel_id = id;
    }

    #[inline]
    pub fn set_leaf_count(&mut self, count: usize) {
        self.leaf_count = count;
    }

    #[inline]
    pub fn set_maximized(&mut self, is_maximized: bool) {
        self.is_maximized = is_maximized;
    }

    /// Returns whether outline is enabled for `kind` in this editor.
    /// Defaults to `true` (mode-enabled initial state).
    #[inline]
    pub fn is_outline_enabled_for_kind(&self, kind: &PaneKind) -> bool {
        self.outline_enabled_by_kind
            .get(kind)
            .copied()
            .unwrap_or(true)
    }

    /// Toggles the outline state for `kind` in this editor.
    /// All panes of this kind in this editor share the new state.
    pub fn toggle_outline_for_kind(&mut self, kind: PaneKind) -> bool {
        let new_state = !self.is_outline_enabled_for_kind(&kind);
        self.outline_enabled_by_kind.insert(kind, new_state);
        new_state
    }

    /// Toggles the outline state for the kind of the pane identified by `pane_id`.
    /// All panes of that kind in this editor will update their outline state.
    pub fn toggle_outline_for_pane(&mut self, pane_id: PaneId) -> bool {
        let kind = self
            .session
            .root
            .tree
            .find_leaf_kind(pane_id.0)
            .or_else(|| self.pane_state_ref(pane_id).map(|s| s.pane().kind()));
        if let Some(kind) = kind {
            self.toggle_outline_for_kind(kind)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test;

    #[test]
    fn test_outline_enabled_by_kind_isolation_and_sharing() {
        let mut editor1_map: HashMap<PaneKind, bool> = HashMap::new();
        let editor2_map: HashMap<PaneKind, bool> = HashMap::new();

        let wysiwyg = PaneKind::new("splitype.wysiwyg");
        let source_code = PaneKind::new("splitype.source-code");

        // 1. Initial state is mode-enabled ("模式开启的")
        let ed1_wysiwyg_initial = editor1_map.get(&wysiwyg).copied().unwrap_or(true);
        let ed1_source_initial = editor1_map.get(&source_code).copied().unwrap_or(true);
        assert!(ed1_wysiwyg_initial);
        assert!(ed1_source_initial);

        // 2. Toggle WYSIWYG off in Editor 1
        editor1_map.insert(wysiwyg.clone(), false);

        // All WYSIWYG panes in Editor 1 now observe false
        assert!(!editor1_map.get(&wysiwyg).copied().unwrap_or(true));

        // Source code panes in Editor 1 remain true (independent)
        assert!(editor1_map.get(&source_code).copied().unwrap_or(true));

        // Editor 2 WYSIWYG panes remain true (Editor 2 is independent from Editor 1)
        assert!(editor2_map.get(&wysiwyg).copied().unwrap_or(true));
    }

    #[test]
    fn test_effective_pane_delegation() {
        use std::path::PathBuf;

        let universal_kind = PaneKind::from_static("test.universal");
        let takeover_target = PaneKind::from_static("test.takeover_target");
        let _ = editor_contracts::PaneRegistry::register_takeover_global(
            universal_kind.clone(),
            takeover_target.clone(),
            true,
            Arc::new(|doc| !doc.is_markdown()),
        );

        let md_doc = DocumentSnapshot::empty().with_path(PathBuf::from("readme.md"));
        let rs_doc = DocumentSnapshot::empty().with_path(PathBuf::from("main.rs"));

        let (eff_univ_md, ro_univ_md) =
            Editor::resolve_effective_pane(&universal_kind, Some(&md_doc));
        assert_eq!(eff_univ_md, universal_kind);
        assert!(!ro_univ_md);

        let (eff_univ_rs, ro_univ_rs) =
            Editor::resolve_effective_pane(&universal_kind, Some(&rs_doc));
        assert_eq!(eff_univ_rs, takeover_target);
        assert!(ro_univ_rs);
    }
}
