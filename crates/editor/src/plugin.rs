use crate::document::DocumentStore;
use crate::editor::Editor;
use crate::session::{DocumentTab, EditorSession, PersistedEditorSession};
use editor_contracts::{DocumentHost, DocumentId, DocumentPanel, TabKind};
use gpui::*;
use platform_contracts::{PanelDescriptor, PanelId, PanelKind, PanelRenderContext, PanelView};
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

    fn display_name(&self) -> SharedString {
        "Editor".into()
    }

    fn icon(&self) -> Option<&'static str> {
        Some("plugin://splitype.editor/panel.svg")
    }

    fn is_dirty(&self, cx: &App) -> bool {
        self.editor.read(cx).session.has_unsaved_buffers(cx)
    }

    fn first_dirty_title(&self, cx: &App) -> Option<String> {
        self.editor.read(cx).first_dirty_title(cx)
    }

    fn on_fs_path_renamed(&mut self, from: &Path, to: &Path, cx: &mut App) {
        self.editor.update(cx, |editor, cx| {
            editor.on_fs_path_renamed(from, to, cx);
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

    /// Esc dismissal (routed by the shell's global `DismissTransientUi`
    /// action): cancels in-progress pane split operations. Never invoked
    /// from click-away dismissal, so drags started by the current
    /// mouse-down are not cancelled by the same event.
    fn handle_dismiss_transient_ui(&mut self, cx: &mut App) -> bool {
        self.editor
            .update(cx, |editor, cx| editor.dismiss_transient_ui(cx))
    }

    /// Parks the live session when this panel kind switches away. The
    /// shared buffers stay registered in the store; restoring this kind
    /// hands the same session (pane states included) back.
    fn suspend_state(&mut self, cx: &mut App) -> Option<Box<dyn Any>> {
        let editor = self.editor.clone();
        Some(Box::new(editor.update(cx, |editor, cx| {
            editor.clear_search_highlights_from_document(cx);
            editor.search.visible = false;
            editor.search.matches.clear();
            std::mem::replace(&mut editor.session, EditorSession::empty())
        })))
    }

    /// Clones the session as a durable projection referencing the same
    /// buffers — split editors and cloned windows share one document source.
    fn clone_state(&self, cx: &mut App) -> Option<Box<dyn Any>> {
        let editor = self.editor.clone();
        Some(Box::new(
            editor.update(cx, |editor, cx| editor.session.to_persisted(cx)),
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
            editor.discard_changes(cx);
        });
    }

    fn release_documents(&mut self, cx: &mut App) {
        self.editor.update(cx, |editor, cx| {
            editor.release_documents(cx);
        });
    }

    fn save_all(&mut self, window: &mut Window, cx: &mut App) -> Result<(), String> {
        self.editor.update(cx, |editor, cx| {
            editor.save_all_dirty_tabs(window, cx);
        });
        Ok(())
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
            if !has_content {
                return;
            }
            // A path already open elsewhere resolves to its shared buffer —
            // in-memory content (possibly dirty) wins over the passed text.
            let buffer = DocumentStore::create(text, path, cx);
            editor.attach_tab(DocumentTab::new(buffer, TabKind::Persistent), cx);
            editor.activate_tab(editor.session.tab_count() - 1, cx);
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
        editor
            .active_tab()
            .and_then(|tab| tab.buffer.read(cx).path.clone())
    }

    fn tab_display_name(&self, index: usize, cx: &App) -> Option<String> {
        let editor = self.editor.read(cx);
        editor.session.tab(index).map(|tab| {
            tab.buffer
                .read(cx)
                .path
                .as_ref()
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| "Untitled".to_string())
        })
    }

    fn document_buffer_ids(&self, cx: &App) -> Vec<DocumentId> {
        let editor = self.editor.read(cx);
        let mut ids: Vec<DocumentId> = editor
            .session
            .tabs()
            .map(|tab| tab.buffer.read(cx).id)
            .collect();
        ids.dedup();
        ids
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
            editor.discard_tab_at(index, cx);
        });
    }

    fn clear_tabs(&mut self, cx: &mut App) {
        self.editor.update(cx, |editor, cx| {
            editor.clear_tabs(cx);
        });
    }

    fn has_unsaved_dialog(&self, cx: &App) -> bool {
        let editor = self.editor.read(cx);
        editor
            .session
            .tabs()
            .any(|tab| tab.pending.show_unsaved_changes_dialog)
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
            .any(|tab| tab.pending.show_drop_replace_dialog)
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
        format: editor_contracts::ExportFormat,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.editor.update(cx, |editor, cx| {
            editor.export_document_via_prompt(format, window, cx);
        });
    }
}

/// Casts a panel view to its document role when it is an editor panel.
///
/// Registered with the composition root's document-routing table. The
/// concrete downcast lives here because only the editor knows its view type;
/// the shell never imports [`EditorPanelView`].
pub fn document_role(view: &dyn PanelView) -> Option<&dyn DocumentPanel> {
    view.as_any()
        .downcast_ref::<EditorPanelView>()
        .map(|view| view as &dyn DocumentPanel)
}

/// Mutable variant of [`document_role`].
pub fn document_role_mut(view: &mut dyn PanelView) -> Option<&mut dyn DocumentPanel> {
    view.as_any_mut()
        .downcast_mut::<EditorPanelView>()
        .map(|view| view as &mut dyn DocumentPanel)
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

    fn display_name(&self) -> SharedString {
        "Editor".into()
    }

    fn icon(&self) -> Option<&'static str> {
        Some("plugin://splitype.editor/panel.svg")
    }

    fn create_panel(&self, panel_id: PanelId, cx: &mut App) -> Box<dyn PanelView> {
        let session = crate::session::EditorSession::empty();
        let editor = cx.new(|cx| Editor::with_session(panel_id, session, cx));
        Box::new(EditorPanelView::new(editor))
    }

    fn restore_panel(
        &self,
        panel_id: PanelId,
        state: Box<dyn Any>,
        cx: &mut App,
    ) -> Option<Box<dyn PanelView>> {
        // Two distinct sources converge on one live model: a suspended
        // live session (panel kind switch) keeps its registered views, while
        // a durable projection (clone / window restore) must register each
        // resolved buffer as a new view.
        let session = if state.is::<crate::session::EditorSession>() {
            let live = state
                .downcast::<crate::session::EditorSession>()
                .expect("just checked");
            *live
        } else {
            let persisted = state.downcast::<PersistedEditorSession>().ok()?;
            let session = EditorSession::from_persisted(*persisted, cx);
            for tab in session.tabs() {
                let id = tab.buffer.read(cx).id;
                cx.global_mut::<DocumentStore>().acquire(id);
            }
            session
        };
        let editor = cx.new(|cx| Editor::with_session(panel_id, session, cx));
        Some(Box::new(EditorPanelView::new(editor)))
    }

    fn retained_dirty_info(&self, state: &dyn Any, cx: &App) -> (bool, Option<String>) {
        let Some(session) = state.downcast_ref::<crate::session::EditorSession>() else {
            return (false, None);
        };
        let store = cx.global::<DocumentStore>();
        let mut owned_dirty = false;
        let mut first_name = None;
        for tab in session.tabs() {
            let buffer = tab.buffer.read(cx);
            if !buffer.dirty {
                continue;
            }
            let own_views = session
                .tabs()
                .filter(|other| other.buffer == tab.buffer)
                .count();
            if store.view_count(buffer.id) == own_views {
                owned_dirty = true;
                first_name.get_or_insert_with(|| {
                    buffer
                        .path
                        .as_ref()
                        .and_then(|path| path.file_name())
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Untitled".to_string())
                });
            }
        }
        (owned_dirty, first_name)
    }

    fn discard_retained(&self, state: &mut Box<dyn Any>, cx: &mut App) {
        let Some(session) = state.downcast_mut::<crate::session::EditorSession>() else {
            return;
        };
        for tab in session.tabs() {
            let buffer = tab.buffer.clone();
            let (id, dirty) = {
                let buffer = buffer.read(cx);
                (buffer.id, buffer.dirty)
            };
            if dirty && cx.global::<DocumentStore>().view_count(id) == 1 {
                buffer.update(cx, |buffer, cx| buffer.mark_discarded(cx));
                cx.global_mut::<DocumentStore>().discard(id);
            } else {
                cx.global_mut::<DocumentStore>().release(id, false);
            }
        }
        session.clear_tabs();
    }

    fn release_retained(&self, state: &mut Box<dyn Any>, cx: &mut App) {
        let Some(session) = state.downcast_mut::<crate::session::EditorSession>() else {
            return;
        };
        for tab in session.tabs() {
            let buffer = tab.buffer.clone();
            let id = buffer.read(cx).id;
            cx.global_mut::<DocumentStore>().release(id, false);
        }
        session.clear_tabs();
    }

    fn retained_buffer_ids(&self, state: &dyn Any, cx: &App) -> Vec<DocumentId> {
        let Some(session) = state.downcast_ref::<crate::session::EditorSession>() else {
            return Vec::new();
        };
        let mut ids: Vec<DocumentId> = session.tabs().map(|tab| tab.buffer.read(cx).id).collect();
        ids.dedup();
        ids
    }

    fn serialize_state(&self, state: &dyn Any) -> Option<serde_json::Value> {
        let session = state.downcast_ref::<PersistedEditorSession>()?;
        serde_json::to_value(session).ok()
    }

    fn deserialize_state(&self, json: &serde_json::Value) -> Option<Box<dyn Any>> {
        let session: PersistedEditorSession = serde_json::from_value(json.clone()).ok()?;
        Some(Box::new(session))
    }
}
