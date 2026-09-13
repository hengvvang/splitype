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
pub mod links_host;
pub mod pane_host;
pub mod search_host;

use std::collections::{HashMap, HashSet};
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
    /// Track open outline panels per individual pane id.
    pub outline_open_by_pane: HashSet<PaneId>,
    pub focused_pane_id: Option<PaneId>,
    pub search: editor_contracts::SearchPanelState,
    /// Observer subscriptions per shared buffer, keyed by buffer id.
    pub buffer_subscriptions: HashMap<editor_contracts::DocumentId, Subscription>,
    /// Set once this panel released all of its document views.
    pub documents_released: bool,
    /// Last known render width per pane id. Preserves pane layout width across tab switches
    /// so newly mounted tabs do not suffer from a 0-width initial frame offset shift.
    pub last_pane_widths: HashMap<PaneId, f32>,
    /// Active drag-hover state when a tab is dragged over this editor.
    pub tab_drag_hover: Option<crate::layout::tab_drag::TabDragHoverState>,
    /// Active tab reorder target index when a tab is dragged over this editor's tab bar.
    pub tab_reorder_target: Option<usize>,
    /// Navigation history stack tracking jump locations across documents and anchors.
    pub jump_stack: crate::session::EditorJumpStack,
    /// Active link relationships widget state (when opened).
    pub links_widget: Option<crate::links::LinksWidgetState>,
    pub links_host: Arc<dyn crate::links::LinksWidgetHost>,
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
            outline_open_by_pane: HashSet::new(),
            focused_pane_id: None,
            search: editor_contracts::SearchPanelState::new(cx),
            buffer_subscriptions: HashMap::new(),
            documents_released: false,
            last_pane_widths: HashMap::new(),
            tab_drag_hover: None,
            tab_reorder_target: None,
            jump_stack: crate::session::EditorJumpStack::new(),
            links_widget: None,
            links_host: crate::editor::links_host::EditorLinksHost::new(cx.weak_entity()),
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

    /// Reorders the tab from `from` index to `to` index.
    pub fn reorder_tab(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        self.session.reorder_tab(from, to);
        self.sync_panes_with_active_tab(cx);
        cx.notify();
    }

    /// Opens a file in this editor's tab list without splitting the window.
    pub fn open_file(
        &mut self,
        path: &std::path::Path,
        kind: TabKind,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let buffer = match DocumentStore::open(path, cx) {
            Ok(buffer) => buffer,
            Err(err) => return Err(format!("failed to read '{}': {err}", path.display())),
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
            return Ok(());
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
                return Ok(());
            }
        }

        self.attach_tab(tab, cx);
        self.activate_tab(self.session.tab_count() - 1, cx);
        self.record_recent_file(path, cx);
        Ok(())
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
        if let Err(err) = self.open_file(path, kind, cx) {
            self.show_drop_open_failed_prompt(err, window, cx);
        }
    }

    /// Captures the current jump location for back/forward navigation.
    pub fn current_jump_point(&self, cx: &App) -> Option<crate::session::JumpPoint> {
        let tab = self.session.active_tab()?;
        let buffer = tab.buffer.read(cx);
        let pane_id = self.active_pane_id();
        let scroll_y = self
            .pane_state_ref(pane_id)
            .map(|s| f32::from(-s.scroll.handle.offset().y))
            .unwrap_or(0.0);
        Some(crate::session::JumpPoint {
            document_id: buffer.id,
            file_path: buffer.path.clone(),
            scroll_y,
        })
    }

    /// Navigates back to the previous jump point (Alt+Left).
    pub fn navigate_back(&mut self, cx: &mut Context<Self>) {
        let Some(current) = self.current_jump_point(cx) else {
            return;
        };
        let Some(target) = self.jump_stack.pop_back(current) else {
            return;
        };
        self.restore_jump_point(target, cx);
    }

    /// Navigates forward to the next jump point (Alt+Right).
    pub fn navigate_forward(&mut self, cx: &mut Context<Self>) {
        let Some(current) = self.current_jump_point(cx) else {
            return;
        };
        let Some(target) = self.jump_stack.pop_forward(current) else {
            return;
        };
        self.restore_jump_point(target, cx);
    }

    fn restore_jump_point(&mut self, point: crate::session::JumpPoint, cx: &mut Context<Self>) {
        let found_idx = self.session.tabs().position(|t| {
            let b = t.buffer.read(cx);
            b.id == point.document_id || (point.file_path.is_some() && b.path == point.file_path)
        });
        if let Some(idx) = found_idx {
            self.activate_tab(idx, cx);
        } else if let Some(path) = &point.file_path {
            if path.exists() {
                let _ = self.open_file(path, TabKind::Persistent, cx);
            }
        }
        let pane_id = self.active_pane_id();
        self.scroll_pane_to_y(pane_id, point.scroll_y, cx);
    }

    /// Navigates to a target link (external URL, wikilink, or markdown file/anchor).
    pub fn navigate_to_link(&mut self, target: &str, cx: &mut Context<Self>) {
        let target = target.trim();
        if target.is_empty() {
            return;
        }

        if target.starts_with("http://")
            || target.starts_with("https://")
            || target.starts_with("mailto:")
        {
            cx.open_url(target);
            return;
        }

        let raw_target = target.strip_prefix("wikilink:").unwrap_or(target);
        let (file_target, anchor) = if let Some((f, a)) = raw_target.split_once('#') {
            (f.trim(), Some(a.trim()))
        } else {
            (raw_target.trim(), None)
        };

        if let Some(current) = self.current_jump_point(cx) {
            self.jump_stack.push(current);
        }

        if file_target.is_empty() {
            if let Some(anchor) = anchor {
                self.jump_to_anchor(anchor, cx);
            }
            return;
        }

        let resolved_path = self.resolve_link_target_path(file_target, cx);
        if let Some(path) = resolved_path {
            if self.open_file(&path, TabKind::Persistent, cx).is_ok() {
                if let Some(anchor) = anchor {
                    self.jump_to_anchor(anchor, cx);
                }
            }
        }
    }

    /// Resolves a link or wikilink note name into a physical filesystem path.
    pub fn resolve_link_target_path(&self, target: &str, cx: &App) -> Option<std::path::PathBuf> {
        let target_path = std::path::Path::new(target);
        if target_path.is_absolute() && target_path.exists() {
            return Some(target_path.to_path_buf());
        }

        let current_dir = self
            .session
            .active_tab()
            .and_then(|t| t.buffer.read(cx).path.as_ref())
            .and_then(|p| p.parent().map(|p| p.to_path_buf()));

        let candidates = [
            target.to_string(),
            if !target.ends_with(".md") && !target.ends_with(".markdown") {
                format!("{target}.md")
            } else {
                target.to_string()
            },
        ];

        if let Some(dir) = &current_dir {
            for c in &candidates {
                let candidate_path = dir.join(c);
                if candidate_path.exists() {
                    return Some(candidate_path);
                }
            }
        }

        for tab in self.session.tabs() {
            if let Some(p) = tab.buffer.read(cx).path.as_ref() {
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    for c in &candidates {
                        if name.eq_ignore_ascii_case(c) {
                            return Some(p.clone());
                        }
                    }
                }
                if let Some(stem) = p.file_stem().and_then(|n| n.to_str()) {
                    if stem.eq_ignore_ascii_case(target) {
                        return Some(p.clone());
                    }
                }
            }
        }

        if let Some(dir) = current_dir {
            let file_name = if target.ends_with(".md") {
                target.to_string()
            } else {
                format!("{target}.md")
            };
            let new_path = dir.join(file_name);
            let _ = std::fs::write(&new_path, "");
            return Some(new_path);
        }

        None
    }

    /// Jumps to an anchor (heading or block ^id) in the active pane.
    pub fn jump_to_anchor(&mut self, anchor: &str, cx: &mut Context<Self>) {
        let pane_id = self.active_pane_id();
        let theme = cx.global::<theme::ThemeManager>().current_arc();
        if let Some(state) = self.pane_state_mut(pane_id) {
            if let Some(target_y) = state.pane_mut().navigate_to_anchor(anchor, &theme, cx) {
                state
                    .scroll
                    .handle
                    .set_offset(point(px(0.0), px(-target_y.max(0.0))));
                cx.notify();
            }
        }
    }

    /// Provides a preview snippet for hovering over a link or anchor.
    pub fn peek_link(&self, target: &str, cx: &App) -> Option<String> {
        let target = target.trim();
        if target.is_empty() {
            return None;
        }

        if target.starts_with("http://")
            || target.starts_with("https://")
            || target.starts_with("mailto:")
        {
            return Some(target.to_string());
        }

        let raw_target = target.strip_prefix("wikilink:").unwrap_or(target);
        let (file_target, anchor) = if let Some((f, a)) = raw_target.split_once('#') {
            (f.trim(), Some(a.trim()))
        } else {
            (raw_target.trim(), None)
        };

        let content = if file_target.is_empty() {
            self.session.active_tab().map(|t| t.buffer.read(cx).snapshot().text.to_string())
        } else {
            let path = self.resolve_link_target_path(file_target, cx)?;
            if let Some(tab) = self.session.tabs().find(|t| t.buffer.read(cx).path.as_ref() == Some(&path)) {
                Some(tab.buffer.read(cx).snapshot().text.to_string())
            } else {
                std::fs::read_to_string(&path).ok()
            }
        }?;

        if let Some(anchor) = anchor {
            if let Some(block_id) = anchor.strip_prefix('^') {
                let token = format!("^{block_id}");
                for line in content.lines() {
                    if line.contains(&token) {
                        return Some(format!("{line}\n\n(Block: ^{block_id})"));
                    }
                }
            } else {
                let mut found_heading = false;
                let mut snippet = Vec::new();
                let normalize = |s: &str| -> String {
                    s.to_lowercase()
                        .replace(['-', '_', '#'], " ")
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                let target_norm = normalize(anchor);
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with('#') {
                        let heading_title = trimmed.trim_start_matches('#').trim();
                        if normalize(heading_title) == target_norm || heading_title.eq_ignore_ascii_case(anchor) {
                            found_heading = true;
                            snippet.push(line);
                            continue;
                        } else if found_heading {
                            break;
                        }
                    }
                    if found_heading {
                        snippet.push(line);
                        if snippet.len() >= 8 {
                            snippet.push("...");
                            break;
                        }
                    }
                }
                if !snippet.is_empty() {
                    return Some(snippet.join("\n"));
                }
            }
        }

        let preview_lines: Vec<&str> = content
            .lines()
            .take(10)
            .collect();
        if !preview_lines.is_empty() {
            let mut summary = preview_lines.join("\n");
            if content.lines().count() > 10 {
                summary.push_str("\n...");
            }
            Some(summary)
        } else {
            Some(format!("(Empty note: {raw_target})"))
        }
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

    /// Returns whether the outline panel is open for the specific pane identified by `pane_id`.
    #[inline]
    pub fn is_outline_open_for_pane(&self, pane_id: PaneId) -> bool {
        self.outline_open_by_pane.contains(&pane_id)
    }

    /// Toggles the outline panel overlay for the pane identified by `pane_id`.
    /// Scoped strictly to this pane, closing conflicting panels on this pane.
    pub fn toggle_outline_for_pane(&mut self, pane_id: PaneId) -> bool {
        if self.outline_open_by_pane.contains(&pane_id) {
            self.outline_open_by_pane.remove(&pane_id);
            false
        } else {
            if let Some(ref p) = self.links_widget {
                if p.target_pane_id == Some(pane_id) {
                    self.links_widget = None;
                }
            }
            if self.search.target_pane_id == Some(pane_id) {
                self.search.visible = false;
            }
            self.outline_open_by_pane.insert(pane_id);
            true
        }
    }

    /// Closes the outline widget for the given pane.
    pub fn close_outline_for_pane(&mut self, pane_id: PaneId) {
        self.outline_open_by_pane.remove(&pane_id);
    }

    /// Toggles the link relationships widget for the active document.
    pub fn toggle_links_widget(&mut self, pane_id: PaneId, cx: &mut Context<Self>) {
        let active_path = self
            .session
            .active_tab()
            .and_then(|tab| tab.buffer.read(cx).path.clone());

        if let Some(ref current_widget) = self.links_widget {
            if current_widget.target_pane_id == Some(pane_id) {
                self.links_widget = None;
                cx.notify();
                return;
            }
        }

        // Opening links widget on this pane: close search and outline on this pane
        self.outline_open_by_pane.remove(&pane_id);
        if self.search.target_pane_id == Some(pane_id) {
            self.search.visible = false;
        }

        let mut widget_state = crate::links::LinksWidgetState::new(active_path.clone(), Some(pane_id));

        if let Some(active_tab) = self.session.active_tab() {
            let buffer_entity = active_tab.buffer.clone();
            let snapshot = buffer_entity.read(cx).snapshot();
            let outgoing = crate::links::extract_outgoing_links(&snapshot.text);
            widget_state.report = Some(crate::links::LinkRelationReport {
                outgoing,
                backlinks: Vec::new(),
            });
        }

        self.links_widget = Some(widget_state);
        cx.notify();

        if let Some(target_file) = active_path {
            let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            let weak_editor = cx.entity().downgrade();
            cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
                let backlinks = cx
                    .background_executor()
                    .spawn(async move {
                        crate::links::scan_workspace_backlinks(&root, &target_file)
                    })
                  .await;

                let _ = weak_editor.update(cx, |editor, cx| {
                    if let Some(ref mut widget) = editor.links_widget {
                        if let Some(ref mut report) = widget.report {
                            report.backlinks = backlinks;
                        }
                        widget.is_loading = false;
                        cx.notify();
                    }
                });
            })
            .detach();
        } else if let Some(ref mut widget) = self.links_widget {
            widget.is_loading = false;
        }
    }

    /// Backwards-compatible alias for toggle_links_widget.
    #[inline]
    pub fn toggle_links_panel(&mut self, pane_id: PaneId, cx: &mut Context<Self>) {
        self.toggle_links_widget(pane_id, cx);
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

    #[test]
    fn test_breadcrumb_actions_pane_isolation() {
        let mut outline_open_by_pane: HashSet<PaneId> = HashSet::new();
        let pane1 = PaneId::from(1);
        let pane2 = PaneId::from(2);

        // Initially both closed
        assert!(!outline_open_by_pane.contains(&pane1));
        assert!(!outline_open_by_pane.contains(&pane2));

        // Toggle on pane 1
        outline_open_by_pane.insert(pane1);
        assert!(outline_open_by_pane.contains(&pane1));
        assert!(!outline_open_by_pane.contains(&pane2));

        // Toggle on pane 2
        outline_open_by_pane.insert(pane2);
        assert!(outline_open_by_pane.contains(&pane1));
        assert!(outline_open_by_pane.contains(&pane2));

        // Toggle off pane 1
        outline_open_by_pane.remove(&pane1);
        assert!(!outline_open_by_pane.contains(&pane1));
        assert!(outline_open_by_pane.contains(&pane2));
    }
}
