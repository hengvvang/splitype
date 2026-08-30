//! Top-level editor aggregate root.
//!
//! [`Editor`] aggregates the editor's own state: the raw text session tabs,
//! view mode, scroll state, focus management, and the editor's pane views
//! (WYSIWYG, Source Code, Preview, and custom plugins).
//!
//! `Editor` holds the single authoritative raw text source of truth and does
//! zero AST parsing or serialization itself. All syntax trees and viewport
//! specifics live strictly inside each pane plugin implementation.

pub mod export;
pub mod host_bridge;

pub use std::time::{Duration, Instant};

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub use gpui::*;

pub use editor_model::{AutoscrollStrategy, EditorHost, PaneId};
pub use editor_outline::OutlineHudState;
pub use splitter::root::SplitterRoot;
pub use workspace::{PanelId, WindowPanelKind, DEFAULT_EDITOR_PANEL_ID};

pub use crate::session::{
    DocumentTab, EditorPaneKind, EditorSession, EditorTabList, FileState, OpenFileMode, PaneKindId,
    PaneState, PendingOpenLink, ScrollState, ScrollbarDragSession, TabKind,
};
pub use export::ExportFormat;
pub use host_bridge::{EditorPaneHost, EditorSearchIme, EditorSearchView};

/// The Editor aggregate root entity.
pub struct Editor {
    pub panel_id: PanelId,
    pub entity_id: EntityId,
    pub self_weak: WeakEntity<Self>,
    pub host: Option<Arc<dyn EditorHost>>,
    pub pane_host: Arc<dyn editor_model::PaneHost>,
    pub search_view: Arc<dyn editor_search::SearchStateView>,
    pub search_ime: Arc<dyn editor_search::SearchIme>,
    pub session: EditorSession,
    pub panel_rect: Option<Bounds<Pixels>>,
    pub is_active_panel: bool,
    pub is_maximized: bool,
    pub leaf_count: usize,
    pub outline: OutlineHudState,
    pub welcome_last_click: Option<Instant>,
    pub focused_pane_id: Option<PaneId>,
    pub search: crate::search::SearchPanelState,
}

impl Editor {
    /// Creates a fresh Editor entity with an untitled tab from markdown content.
    pub fn new(markdown: String, file_path: Option<PathBuf>, cx: &mut Context<Self>) -> Self {
        let tab = Self::new_tab_from_markdown(markdown, file_path);
        let mut editor = Self {
            panel_id: PanelId(DEFAULT_EDITOR_PANEL_ID),
            entity_id: cx.entity().entity_id(),
            self_weak: cx.weak_entity(),
            host: None,
            pane_host: EditorPaneHost::new(cx.weak_entity()),
            search_view: EditorSearchView::new(cx.weak_entity()),
            search_ime: EditorSearchIme::new(cx.weak_entity()),
            session: EditorSession::welcome(),
            panel_rect: None,
            is_active_panel: false,
            is_maximized: false,
            leaf_count: 1,
            outline: OutlineHudState::default(),
            welcome_last_click: None,
            focused_pane_id: None,
            search: crate::search::SearchPanelState::new(cx),
        };
        editor.session.push_tab(tab);
        editor
    }

    /// Creates an Editor initialized with an existing session (e.g. restored or cloned).
    pub fn with_session(
        panel_id: PanelId,
        session: EditorSession,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            panel_id,
            entity_id: cx.entity().entity_id(),
            self_weak: cx.weak_entity(),
            host: None,
            pane_host: EditorPaneHost::new(cx.weak_entity()),
            search_view: EditorSearchView::new(cx.weak_entity()),
            search_ime: EditorSearchIme::new(cx.weak_entity()),
            session,
            panel_rect: None,
            is_active_panel: false,
            is_maximized: false,
            leaf_count: 1,
            outline: OutlineHudState::default(),
            welcome_last_click: None,
            focused_pane_id: None,
            search: crate::search::SearchPanelState::new(cx),
        }
    }

    /// Builds a document tab from raw Markdown and an optional file path.
    pub fn new_tab_from_markdown(
        markdown: String,
        file_path: Option<PathBuf>,
    ) -> DocumentTab {
        let normalized = markdown.replace("\r\n", "\n").replace('\r', "\n");
        DocumentTab {
            text: normalized,
            document_revision: 1,
            file: FileState {
                path: file_path,
                ..FileState::default()
            },
            kind: TabKind::Persistent,
            panes: HashMap::new(),
            cached_word_count: None,
        }
    }

    pub fn image_base_dir(&self) -> Option<PathBuf> {
        self.tab().file.path.as_ref().and_then(|p| p.parent()).map(|p| p.to_path_buf())
    }

    /// Synchronizes all panes of the active tab with the current raw text.
    pub fn sync_panes_with_active_tab(&mut self, cx: &mut Context<Self>) {
        let Some(tab) = self.session.active_tab() else {
            return;
        };
        let text = tab.text.clone();
        let revision = tab.document_revision;
        if let Some(tab_mut) = self.session.active_tab_mut() {
            for state in tab_mut.panes.values_mut() {
                state.pane.sync_document_text(&text, revision, cx);
            }
        }
    }

    /// Rebuilds document text from a markdown string (e.g. from an editing pane).
    pub fn rebuild_document_from_markdown(&mut self, text: &str, cx: &mut Context<Self>) {
        let active_pane = self.active_pane_id();
        self.update_raw_document_text(text.to_string(), active_pane, cx);
    }

    /// Updates raw text source of truth and syncs other panes.
    pub fn update_raw_document_text(
        &mut self,
        new_text: String,
        origin_pane: PaneId,
        cx: &mut Context<Self>,
    ) {
        if let Some(tab) = self.session.active_tab_mut() {
            tab.text = new_text;
            tab.document_revision = tab.document_revision.wrapping_add(1);
            tab.file.dirty = true;
            tab.file.pending_window_edited = true;
            tab.cached_word_count = None;

            let text = tab.text.clone();
            let revision = tab.document_revision;
            for (&pane_id, state) in tab.panes.iter_mut() {
                if pane_id != origin_pane {
                    state.pane.sync_document_text(&text, revision, cx);
                }
            }
        }
        cx.notify();
    }

    /// Activates the tab at `index`, restoring its focus and window chrome.
    pub fn activate_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.session.tab_count() {
            return;
        }
        self.session.set_active_tab(index);
        if let Some(tab) = self.session.tab_mut(index) {
            tab.file.pending_window_title_refresh = true;
            tab.file.pending_window_edited = true;
        }
        self.sync_panes_with_active_tab(cx);
        if self.search.visible {
            self.execute_search(cx);
        }
        cx.notify();
    }

    /// Opens a file in this editor's tab list: activates its tab if
    /// already open, otherwise loads a new tab from disk.
    pub fn open_file_in_panel(
        &mut self,
        path: &std::path::Path,
        mode: OpenFileMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let already_open = self
            .session
            .tab_list
            .iter()
            .position(|t| t.file.path.as_deref() == Some(path));
        if let Some(index) = already_open {
            if mode == OpenFileMode::Persistent {
                if let Some(tab) = self.session.tab_mut(index) {
                    tab.persist();
                }
            }
            self.activate_tab(index, cx);
            return;
        }

        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(_) => {
                self.show_drop_open_failed_prompt(
                    format!("failed to read '{}'", path.display()),
                    window,
                    cx,
                );
                return;
            }
        };
        let markdown = String::from_utf8_lossy(&bytes).to_string();
        let mut tab = Self::new_tab_from_markdown(markdown, Some(path.to_path_buf()));
        tab.kind = match mode {
            OpenFileMode::Transient => TabKind::Transient,
            OpenFileMode::Persistent => TabKind::Persistent,
        };

        if mode == OpenFileMode::Transient {
            let clean_transient_idx = self
                .session
                .tab_list
                .iter()
                .position(|t| t.is_transient() && !t.file.dirty);
            if let Some(idx) = clean_transient_idx {
                self.session.tab_list.replace(idx, tab);
                self.activate_tab(idx, cx);
                self.record_recent_file(path, cx);
                return;
            }
        }

        let last = self.session.push_tab(tab);
        self.activate_tab(last, cx);
        self.record_recent_file(path, cx);
    }

    fn record_recent_file(&self, path: &std::path::Path, cx: &mut Context<Self>) {
        if let Some(host) = &self.host {
            host.record_recent_file(path, cx);
        }
    }

    pub fn open_file_in_active_editor(
        &mut self,
        path: &std::path::Path,
        mode: OpenFileMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(host) = self.host.clone() else {
            return false;
        };
        host.open_file_in_active_editor(path, mode, window, cx)
    }

    pub fn new_untitled_tab(&mut self, cx: &mut Context<Self>) {
        let last = self
            .session
            .tab_list
            .push(Self::new_tab_from_markdown(String::new(), None));
        self.activate_tab(last, cx);
    }

    pub fn request_close_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(tab) = self.session.tab(index) else {
            return;
        };
        if tab.file.dirty {
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
        if self.session.close_tab(index).is_none() {
            return;
        }
        if !self.session.has_tabs() {
            self.clear_search_highlights_from_document(cx);
            self.search.matches.clear();
            self.search.active_match_index = None;
            cx.notify();
            return;
        }
        if let Some(tab) = self.session.active_tab_mut() {
            tab.file.pending_window_title_refresh = true;
            tab.file.pending_window_edited = true;
        }
        if self.search.visible {
            self.execute_search(cx);
        }
        cx.notify();
    }

    pub fn active_pane_id(&self) -> PaneId {
        if let Some(pane_id) = self.focused_pane_id {
            return pane_id;
        }
        PaneId(self.session.root.tree.first_leaf_id().unwrap_or(0))
    }

    pub fn active_pane_state(&mut self) -> &mut PaneState {
        let pane_id = self.active_pane_id();
        self.pane_state(pane_id)
    }

    pub fn pane_state(&mut self, pane_id: PaneId) -> &mut PaneState {
        let kind = self.pane_kind(pane_id).unwrap_or(PaneKindId::WYSIWYG);
        let tab = self.tab_mut();
        let state = tab.panes.entry(pane_id).or_insert_with(|| PaneState::new(kind));
        state.ensure_kind(kind);
        state
    }

    pub fn pane_state_mut(&mut self, pane_id: PaneId) -> Option<&mut PaneState> {
        let kind = self.pane_kind(pane_id).unwrap_or(PaneKindId::WYSIWYG);
        let tab = self.session.active_tab_mut()?;
        let state = tab.panes.entry(pane_id).or_insert_with(|| PaneState::new(kind));
        state.ensure_kind(kind);
        Some(state)
    }

    pub fn pane_state_ref(&self, pane_id: PaneId) -> Option<&PaneState> {
        let tab = self.active_tab()?;
        tab.panes.get(&pane_id)
    }

    pub fn active_pane_scroll(&self) -> &ScrollState {
        &self
            .pane_state_ref(self.active_pane_id())
            .or_else(|| self.tab().panes.values().next())
            .expect("tab always has at least one pane state")
            .scroll
    }

    pub fn defer_host_action(
        &self,
        cx: &mut Context<Self>,
        action: impl FnOnce(&dyn EditorHost, &mut App) + 'static,
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
        self.session.root.activate_leaf(pane_id.0);
        self.session.root.clear_dropdowns();
        let panel_id = self.panel_id;
        self.defer_host_action(cx, move |host, cx| host.activate_panel(panel_id, cx));
        if !self.has_tabs() {
            cx.notify();
            return;
        }
        if let Some(state) = self.pane_state_mut(pane_id) {
            if let Some(handle) = state.pane.focus_handle(cx) {
                handle.focus(window, cx);
            }
        }
        cx.notify();
    }

    #[inline]
    pub fn pane_kind(&self, pane_id: PaneId) -> Option<PaneKindId> {
        self.session.root.tree.find_leaf(pane_id.0).map(|l| l.kind)
    }

    #[inline]
    pub fn active_pane_kind(&self) -> PaneKindId {
        self.pane_kind(self.active_pane_id())
            .unwrap_or(PaneKindId::WYSIWYG)
    }

    #[inline]
    pub fn is_wysiwyg(&self) -> bool {
        self.active_pane_kind() == PaneKindId::WYSIWYG
    }

    #[inline]
    pub fn is_source_code(&self) -> bool {
        self.active_pane_kind() == PaneKindId::SOURCE_CODE
    }

    #[inline]
    pub fn is_preview(&self) -> bool {
        self.active_pane_kind() == PaneKindId::PREVIEW
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
    pub fn tab(&self) -> &DocumentTab {
        self.session
            .active_tab()
            .expect("active tab requested on empty editor")
    }

    #[inline]
    pub fn tab_mut(&mut self) -> &mut DocumentTab {
        self.session
            .active_tab_mut()
            .expect("active tab mut requested on empty editor")
    }

    #[inline]
    pub fn has_tabs(&self) -> bool {
        self.session.has_tabs()
    }

    pub fn tab_list_mut(&mut self) -> &mut EditorTabList<DocumentTab> {
        &mut self.session.tab_list
    }

    pub fn clone_session(&self, cx: &mut Context<Self>) -> EditorSession {
        let mut root = SplitterRoot::single_leaf(1, PaneKindId::SOURCE_CODE);
        let mut next_id = 1;
        root.tree = self.session.root.tree.clone_with_new_ids(&mut next_id);
        root.next_node_id = next_id;

        let mut list = EditorTabList::new();
        for tab in self.session.tabs() {
            let text = tab.serialized_text(cx);
            let mut copy = Self::new_tab_from_markdown(text, tab.file.path.clone());
            copy.file.dirty = tab.file.dirty;
            copy.kind = tab.kind;
            list.push(copy);
        }
        list.set_active_tab(self.session.active_tab_index());
        EditorSession {
            tab_list: list,
            root,
        }
    }

    pub fn first_dirty_tab(&self) -> Option<(PanelId, usize)> {
        for (index, tab) in self.session.tabs().enumerate() {
            if tab.file.dirty {
                return Some((self.panel_id, index));
            }
        }
        None
    }
}
