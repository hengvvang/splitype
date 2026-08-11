//! The window shell — the OS window's root entity.
//!
//! Owns the mapping from layout areas to content entities (`AreaContent`),
//! the window-level chrome state (the in-window menu bar), and renders the
//! window chrome (custom titlebar + menu bar) above the primary editor's
//! content. Window-level state that currently lives on the Editor (panel
//! layout, overlays) migrates here incrementally; the Editor keeps only
//! its own editing state.

use std::collections::HashMap;

use gpui::*;

use crate::app::window_chrome::MenuBarState;
use crate::editor::controller::Editor;
use crate::editor::session::EditorSession;
use crate::infra::theme::ThemeManager;
use crate::splitter::NodeId;

/// The content of one area in the outer layout tree.
pub enum AreaContent {
    /// An editor with its own tab list and inner panel layout.
    Editor(Entity<Editor>),
}

/// The OS window's root entity: content areas + window lifecycle.
pub struct Shell {
    /// Content entity per outer area id. An area holds a content entity
    /// only while its kind is `Editor`; switching kinds moves the entity
    /// away (its session is retained while it still holds tabs).
    pub(crate) areas: HashMap<NodeId, AreaContent>,
    /// Sessions of Editor areas that left the Editor kind with tabs
    /// (background editing): switching back recreates the entity with the
    /// retained session.
    pub(crate) retained_editor_sessions: HashMap<NodeId, EditorSession>,
    /// Open/hover state for the in-window titlebar menu bar.
    pub(crate) menu_bar: MenuBarState,
}

impl Shell {
    /// The window's primary (first) editor area content, if any.
    pub(crate) fn primary_editor(&self) -> Option<&Entity<Editor>> {
        self.areas.values().find_map(|area| match area {
            AreaContent::Editor(editor) => Some(editor),
        })
    }

    /// Creates a fresh Editor entity serving `area_id` and registers it in
    /// the areas map. The new entity is wired to this Shell and its runtime
    /// registries are rebuilt for the (possibly cloned) document tabs.
    pub(crate) fn add_editor_area(
        &mut self,
        area_id: NodeId,
        session: EditorSession,
        cx: &mut Context<Self>,
    ) -> Entity<Editor> {
        let shell = cx.entity().downgrade();
        let editor = cx.new(|cx| Editor::with_session(area_id, session, cx));
        editor.update(cx, |editor, cx| {
            editor.shell = Some(shell);
            if !editor.session.tab_list.tabs.is_empty() {
                editor.rebuild_table_runtimes(cx);
                editor.rebuild_image_runtimes(cx);
                editor.refresh_preview_blocks(cx);
                editor.refresh_stable_document_snapshot(cx);
            }
        });
        self.areas
            .insert(area_id, AreaContent::Editor(editor.clone()));
        editor
    }

    /// Removes the content entity of `area_id` (if any) and returns its
    /// session so the caller can retain or discard it.
    pub(crate) fn remove_editor_area(
        &mut self,
        area_id: NodeId,
        cx: &mut Context<Self>,
    ) -> Option<EditorSession> {
        let AreaContent::Editor(entity) = self.areas.remove(&area_id)?;
        Some(entity.update(cx, |editor, _cx| {
            std::mem::replace(&mut editor.session, EditorSession::welcome())
        }))
    }

    /// Swaps the content entities of two areas so they follow the swapped
    /// area kinds; retained sessions swap along with them.
    pub(crate) fn swap_area_contents(&mut self, a: NodeId, b: NodeId, cx: &mut Context<Self>) {
        let content_a = self.areas.remove(&a);
        let content_b = self.areas.remove(&b);
        if let Some(AreaContent::Editor(entity)) = content_a {
            entity.update(cx, |editor, _cx| editor.area_id = b);
            self.areas.insert(b, AreaContent::Editor(entity));
        }
        if let Some(AreaContent::Editor(entity)) = content_b {
            entity.update(cx, |editor, _cx| editor.area_id = a);
            self.areas.insert(a, AreaContent::Editor(entity));
        }
        let retained_a = self.retained_editor_sessions.remove(&a);
        let retained_b = self.retained_editor_sessions.remove(&b);
        if let Some(session) = retained_a {
            self.retained_editor_sessions.insert(b, session);
        }
        if let Some(session) = retained_b {
            self.retained_editor_sessions.insert(a, session);
        }
    }

    /// Aligns the areas map with an area-kind change: entering Editor
    /// recreates the entity (restoring a retained session when one exists);
    /// leaving Editor removes the entity, retaining its session while it
    /// still holds tabs (background editing).
    pub(crate) fn sync_area_kind(
        &mut self,
        area_id: NodeId,
        is_editor: bool,
        cx: &mut Context<Self>,
    ) {
        if is_editor {
            if self.areas.contains_key(&area_id) {
                return;
            }
            let session = self
                .retained_editor_sessions
                .remove(&area_id)
                .unwrap_or_else(EditorSession::welcome);
            self.add_editor_area(area_id, session, cx);
        } else if let Some(session) = self.remove_editor_area(area_id, cx) {
            if !session.tab_list.tabs.is_empty() {
                self.retained_editor_sessions.insert(area_id, session);
            }
        }
    }

    /// First dirty tab across every editor area (foreground entities and
    /// retained background sessions), if any. Consumed by the window-close
    /// flow once it moves to the Shell.
    #[allow(dead_code)]
    pub(crate) fn first_dirty_tab(&mut self, cx: &mut Context<Self>) -> Option<(NodeId, usize)> {
        for (area_id, session) in &self.retained_editor_sessions {
            for (index, tab) in session.tab_list.tabs.iter().enumerate() {
                if tab.file.dirty {
                    return Some((*area_id, index));
                }
            }
        }
        let ids: Vec<NodeId> = self
            .areas
            .iter()
            .filter_map(|(id, area)| match area {
                AreaContent::Editor(_) => Some(*id),
            })
            .collect();
        for area_id in ids {
            let dirty = match &self.areas[&area_id] {
                AreaContent::Editor(entity) => entity.read(cx).first_dirty_tab(),
            };
            if let Some(dirty) = dirty {
                return Some(dirty);
            }
        }
        None
    }

    /// Shift-drag default behavior: open the DRAGGED panel in a new
    /// independent window. The engine hands over a fresh single-leaf tree
    /// of the panel's kind; every Editor area's session is deep-copied
    /// (inner panel layout + tab list) and the explorer state is cloned.
    pub(crate) fn clone_container_into_new_window(
        &mut self,
        cloned: crate::splitter::policy::ClonedContainer<crate::app::window_area::WindowAreaKind>,
        cx: &mut Context<Self>,
    ) {
        let mut sessions = HashMap::new();
        let mut cloned_explorer = None;
        for (old_id, new_id) in &cloned.id_map {
            match cloned.tree.find_leaf_kind(*new_id) {
                Some(crate::app::window_area::WindowAreaKind::Editor) => {
                    if let Some(editor) = self.editor_for(*old_id) {
                        let session = editor.update(cx, |editor, cx| editor.clone_session(cx));
                        sessions.insert(*new_id, session);
                    }
                }
                Some(crate::app::window_area::WindowAreaKind::Explorer) => {
                    // The explorer model is window-global: deep-copy it so
                    // the new window shows the same file tree.
                    if let Some(editor) = self.primary_editor() {
                        cloned_explorer =
                            Some(editor.read(cx).panels.explorer.clone_for_new_window());
                    }
                }
                Some(crate::app::window_area::WindowAreaKind::Settings) | None => {}
            }
        }
        crate::app::window::open_cloned_window(
            cloned.tree,
            cloned.next_node_id,
            sessions,
            cloned_explorer,
            cx,
        );
    }

    /// The editor content entity of `area_id`, if the area holds one.
    pub(crate) fn editor_for(&self, area_id: NodeId) -> Option<&Entity<Editor>> {
        match self.areas.get(&area_id) {
            Some(AreaContent::Editor(editor)) => Some(editor),
            _ => None,
        }
    }

    /// Close-button routing for the custom titlebar: delegate to the
    /// primary editor's unsaved-changes-aware close flow.
    pub(crate) fn on_titlebar_close(
        &mut self,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !event.standard_click() {
            return;
        }
        let Some(editor) = self.primary_editor().cloned() else {
            return;
        };
        let _ = editor.update(cx, |editor, cx| {
            editor.request_close_current_window(window, cx);
        });
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeManager>().current_arc();
        let (titlebar, menu_panel, titlebar_height) = self.render_window_chrome(&theme, window, cx);

        let mut base = div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .relative()
            .bg(theme.colors.editor_background)
            // A mouse-down anywhere in the window body closes an open
            // menu; titlebar and menu panels are siblings of the body
            // container, so their clicks never reach this listener.
            .on_any_mouse_down(cx.listener(Self::on_body_mouse_down));

        if let Some(titlebar) = titlebar {
            base = base.child(titlebar);
        }

        // The custom titlebar is absolutely positioned over the window top;
        // offset the body by its height so content starts below it.
        let body = div()
            .w_full()
            .flex_1()
            .min_h(px(0.0))
            .pt(px(titlebar_height))
            .flex()
            .min_w(px(0.0));
        let body = match self.primary_editor() {
            Some(editor) => body.child(editor.clone().into_any_element()),
            None => body,
        };
        base = base.child(body);

        if let Some(menu_panel) = menu_panel {
            base = base.child(menu_panel);
        }

        base.into_any_element()
    }
}
