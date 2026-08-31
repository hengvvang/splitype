use crate::editor::Editor;
use crate::session::EditorSession;
use core_contracts::{DocumentHost, DocumentPanel, PanelCapabilities, PanelKind, TabKind};
use core_contracts::{PanelDescriptor, PanelHost, PanelId, PanelRenderContext, PanelView};
use gpui::*;
use std::any::Any;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Stable kind identifier of the editor panel plugin.
pub const PANEL_KIND: &str = "splitype.panel.editor";

/// Stable plugin identifier of the editor plugin.
pub const PLUGIN_ID: &str = "splitype.editor";

/// Asset directory holding the editor panel's topbar chrome icons.
pub const TOPBAR_ICON_PREFIX: &str = "icons/editor";

/// View wrapper implementing [`PanelView`] for an Editor container panel.
pub struct EditorPanelView {
    pub editor: Entity<Editor>,
}

impl EditorPanelView {
    pub fn new(editor: Entity<Editor>) -> Self {
        Self { editor }
    }
}

impl PanelView for EditorPanelView {
    fn kind(&self) -> PanelKind {
        PanelKind::from_static(PANEL_KIND)
    }

    fn capabilities(&self) -> PanelCapabilities {
        PanelCapabilities::DOCUMENTS
    }

    fn as_document_panel(&self) -> Option<&dyn DocumentPanel> {
        Some(self)
    }

    fn as_document_panel_mut(&mut self) -> Option<&mut dyn DocumentPanel> {
        Some(self)
    }

    fn display_name(&self) -> SharedString {
        "Editor".into()
    }

    fn icon(&self) -> Option<&'static str> {
        Some("plugin://splitype.editor/panel.svg")
    }

    fn is_dirty(&self, cx: &App) -> bool {
        self.editor.read(cx).session.has_dirty_tabs()
    }

    fn first_dirty_title(&self, cx: &App) -> Option<String> {
        let editor = self.editor.read(cx);
        editor.session.tabs().find(|t| t.file.dirty).map(|t| {
            t.file
                .path
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Untitled".to_string())
        })
    }

    fn save(&mut self, window: &mut Window, cx: &mut App) -> Result<(), String> {
        self.editor.update(cx, |editor, cx| {
            editor.save_document(window, cx);
        });
        Ok(())
    }

    fn save_as(&mut self, window: &mut Window, cx: &mut App) {
        self.editor.update(cx, |editor, cx| {
            editor.save_document_as(window, cx);
        });
    }

    fn on_active_changed(&mut self, _is_active: bool, cx: &mut App) {
        cx.notify(self.editor.entity_id());
    }

    fn on_fs_change(&mut self, target_path: Option<&Path>, cx: &mut App) {
        if let Some(path) = target_path {
            self.editor.update(cx, |editor, _cx| {
                for tab in editor.session.tabs_mut() {
                    if let Some(p) = &tab.file.path {
                        if p == path || p.starts_with(path) {
                            tab.file.pending_window_title_refresh = true;
                        }
                    }
                }
            });
        }
    }

    fn on_fs_path_renamed(&mut self, from: &Path, to: &Path, cx: &mut App) {
        self.editor.update(cx, |editor, _cx| {
            editor.update_tab_path(from, to);
        });
    }

    fn handle_inner_mouse_move(
        &mut self,
        position: gpui::Point<gpui::Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        self.editor
            .update(cx, |editor, _cx| editor.update_inner_drag(position, window))
    }

    fn finish_inner_gestures(&mut self, window: &mut Window, cx: &mut App) {
        self.editor
            .update(cx, |editor, cx| editor.finish_inner_drag(window, cx));
    }

    fn suspend_state(&mut self, cx: &mut App) -> Option<Box<dyn Any>> {
        let editor = self.editor.clone();
        Some(Box::new(editor.update(cx, |editor, cx| {
            editor.clear_search_highlights_from_document(cx);
            editor.search.visible = false;
            editor.search.matches.clear();
            std::mem::replace(&mut editor.session, EditorSession::empty())
        })))
    }

    fn clone_state(&self, cx: &mut App) -> Option<Box<dyn Any>> {
        let editor = self.editor.clone();
        Some(Box::new(
            editor.update(cx, |editor, cx| editor.clone_session(cx)),
        ))
    }

    fn render(
        &mut self,
        ctx: &PanelRenderContext,
        _window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        self.editor.update(cx, |editor, _cx| {
            editor.set_panel_id(ctx.panel_id);
            editor.set_leaf_count(ctx.leaf_count);
            editor.set_maximized(ctx.is_maximized);
            editor.panel_rect = ctx.bounds;
            editor.is_active_panel = ctx.is_active;
        });
        self.editor.clone().into_any_element()
    }

    fn set_panel_id(&mut self, id: PanelId, cx: &mut App) {
        self.editor.update(cx, |editor, _cx| {
            editor.set_panel_id(id);
        });
    }

    fn discard_changes(&mut self, cx: &mut App) {
        self.editor.update(cx, |editor, cx| {
            for tab in editor.session.tabs_mut() {
                tab.file.dirty = false;
            }
            cx.notify();
        });
    }

    fn save_all(&mut self, window: &mut Window, cx: &mut App) -> Result<(), String> {
        self.editor.update(cx, |editor, cx| {
            editor.save_all_dirty_tabs(window, cx);
        });
        Ok(())
    }

    fn focus_handle(&self, cx: &App) -> Option<FocusHandle> {
        self.editor.read(cx).active_focus_handle(cx)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl DocumentPanel for EditorPanelView {
    fn attach_document_host(&mut self, host: Arc<dyn DocumentHost>, cx: &mut App) {
        let editor = self.editor.clone();
        editor.update(cx, |editor, cx| {
            editor.host = Some(host);
            if editor.session.has_tabs() {
                editor.sync_panes_with_active_tab(cx);
            }
        });
    }

    fn load_initial_document(&mut self, text: String, path: Option<PathBuf>, cx: &mut App) {
        self.editor.update(cx, |editor, cx| {
            let has_content = !text.is_empty() || path.is_some();
            if has_content {
                let tab = Editor::new_tab_from_markdown(text, path);
                editor.session.push_tab(tab);
            }
            cx.notify();
        });
    }

    fn open_file(&mut self, path: &Path, kind: TabKind, window: &mut Window, cx: &mut App) {
        self.editor.update(cx, |editor, cx| {
            editor.open_file_in_panel(path, kind, window, cx);
        });
    }

    fn active_tab_path(&self, cx: &App) -> Option<PathBuf> {
        let editor = self.editor.read(cx);
        editor.active_tab().and_then(|tab| tab.file.path.clone())
    }

    fn tab_display_name(&self, index: usize, cx: &App) -> Option<String> {
        let editor = self.editor.read(cx);
        editor.session.tab(index).map(|tab| {
            tab.file
                .path
                .as_ref()
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| "Untitled".to_string())
        })
    }

    fn save_tab_at(&mut self, index: usize, window: &mut Window, cx: &mut App) {
        self.editor.update(cx, |editor, cx| {
            editor.save_tab_at(index, window, cx);
        });
    }

    fn close_tab(&mut self, index: usize, cx: &mut App) {
        self.editor.update(cx, |editor, cx| {
            editor.close_tab(index, cx);
        });
    }

    fn discard_tab_at(&mut self, index: usize, cx: &mut App) {
        self.editor.update(cx, |editor, cx| {
            if let Some(tab) = editor.session.tab_mut(index) {
                tab.file.dirty = false;
            }
            editor.close_tab(index, cx);
        });
    }

    fn clear_tabs(&mut self, cx: &mut App) {
        self.editor.update(cx, |editor, cx| {
            editor.session.clear_tabs();
            cx.notify();
        });
    }

    fn has_unsaved_dialog(&self, cx: &App) -> bool {
        let editor = self.editor.read(cx);
        editor
            .session
            .tabs()
            .any(|tab| tab.file.show_unsaved_changes_dialog)
    }

    fn cancel_close_dialog(&mut self, cx: &mut App) {
        self.editor.update(cx, |editor, cx| {
            editor.cancel_close_dialog(cx);
        });
    }

    fn save_and_close_dialog(&mut self, window: &mut Window, cx: &mut App) {
        self.editor.update(cx, |editor, cx| {
            editor.save_and_close(window, cx);
        });
    }

    fn discard_and_close_dialog(&mut self, cx: &mut App) {
        self.editor.update(cx, |editor, cx| {
            editor.discard_and_close(cx);
        });
    }

    fn has_drop_replace_dialog(&self, cx: &App) -> bool {
        let editor = self.editor.read(cx);
        editor
            .session
            .tabs()
            .any(|tab| tab.file.show_drop_replace_dialog)
    }

    fn cancel_drop_replace_dialog(&mut self, cx: &mut App) {
        self.editor.update(cx, |editor, cx| {
            editor.cancel_drop_replace_dialog(cx);
        });
    }

    fn save_and_replace_pending_drop(&mut self, window: &mut Window, cx: &mut App) {
        self.editor.update(cx, |editor, cx| {
            editor.save_and_replace_pending_drop(window, cx);
        });
    }

    fn discard_pending_drop_replace(&mut self, window: &mut Window, cx: &mut App) {
        self.editor.update(cx, |editor, cx| {
            editor.discard_pending_drop_replace(window, cx);
        });
    }

    fn focus_active_pane(&mut self, cx: &mut App) {
        self.editor.update(cx, |editor, cx| {
            let active_pane = editor.active_pane_id();
            if let Some(state) = editor.pane_state_mut(active_pane) {
                let _ = state.pane.focus_handle(cx);
            }
            cx.notify();
        });
    }

    fn request_save_document(&mut self, cx: &mut App) {
        self.editor.update(cx, |editor, cx| {
            editor.request_save_document(cx);
        });
    }

    fn request_save_document_as(&mut self, cx: &mut App) {
        self.editor.update(cx, |editor, cx| {
            editor.request_save_document_as(cx);
        });
    }

    fn save_document(&mut self, window: &mut Window, cx: &mut App) {
        self.editor.update(cx, |editor, cx| {
            editor.save_document(window, cx);
        });
    }

    fn save_document_as(&mut self, window: &mut Window, cx: &mut App) {
        self.editor.update(cx, |editor, cx| {
            editor.save_document_as(window, cx);
        });
    }

    fn export_document(
        &mut self,
        format: core_contracts::ExportFormat,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.editor.update(cx, |editor, cx| {
            editor.export_document_via_prompt(format, window, cx);
        });
    }
}

/// Panel descriptor for the Editor plugin.
#[derive(Clone, Debug, Default)]
pub struct EditorPanelDescriptor {}

impl EditorPanelDescriptor {
    pub fn new() -> Self {
        Self {}
    }
}

impl PanelDescriptor for EditorPanelDescriptor {
    fn kind(&self) -> PanelKind {
        PanelKind::from_static(PANEL_KIND)
    }

    fn capabilities(&self) -> PanelCapabilities {
        PanelCapabilities::DOCUMENTS
    }

    fn display_name(&self) -> SharedString {
        "Editor".into()
    }

    fn icon(&self) -> Option<&'static str> {
        Some("plugin://splitype.editor/panel.svg")
    }

    fn create_panel(
        &self,
        panel_id: PanelId,
        _host: Arc<dyn PanelHost>,
        cx: &mut App,
    ) -> Box<dyn PanelView> {
        let session = crate::session::EditorSession::empty();
        let editor = cx.new(|cx| Editor::with_session(panel_id, session, cx));
        Box::new(EditorPanelView::new(editor))
    }

    fn restore_panel(
        &self,
        panel_id: PanelId,
        _host: Arc<dyn PanelHost>,
        state: Box<dyn Any>,
        cx: &mut App,
    ) -> Option<Box<dyn PanelView>> {
        let session = state
            .downcast::<crate::session::EditorSession>()
            .ok()
            .map(|boxed| *boxed)?;
        let editor = cx.new(|cx| Editor::with_session(panel_id, session, cx));
        Some(Box::new(EditorPanelView::new(editor)))
    }

    fn retained_dirty_info(&self, state: &dyn Any, _cx: &App) -> (bool, Option<String>) {
        let Some(session) = state.downcast_ref::<crate::session::EditorSession>() else {
            return (false, None);
        };
        let mut first_name = None;
        for tab in session.tabs() {
            if tab.file.dirty {
                first_name.get_or_insert_with(|| {
                    tab.file
                        .path
                        .as_ref()
                        .and_then(|path| path.file_name())
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Untitled".to_string())
                });
            }
        }
        (first_name.is_some(), first_name)
    }

    fn discard_retained(&self, state: &mut Box<dyn Any>, _cx: &mut App) {
        let Some(session) = state.downcast_mut::<crate::session::EditorSession>() else {
            return;
        };
        for tab in session.tabs_mut() {
            tab.file.dirty = false;
        }
    }

    fn serialize_state(&self, state: &dyn Any) -> Option<serde_json::Value> {
        let session = state.downcast_ref::<crate::session::EditorSession>()?;
        serde_json::to_value(session).ok()
    }

    fn deserialize_state(&self, json: &serde_json::Value) -> Option<Box<dyn Any>> {
        let session: crate::session::EditorSession = serde_json::from_value(json.clone()).ok()?;
        Some(Box::new(session))
    }
}
