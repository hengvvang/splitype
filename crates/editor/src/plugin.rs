use std::any::Any;
use std::path::Path;
use std::sync::Arc;
use gpui::*;
use window::{PanelDescriptor, PanelHost, PanelId, PanelKind, PanelRenderContext, PanelView};
use crate::editor::Editor;

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
        PanelKind::new("editor")
    }

    fn display_name(&self) -> SharedString {
        "Editor".into()
    }

    fn icon(&self) -> Option<&'static str> {
        Some("icons/editor/panel.svg")
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

    fn save(&mut self, window: &mut Window, cx: &mut App) -> Result<(), String> { self.editor.update(cx, |editor, cx| { editor.save_document(window, cx); }); Ok(()) }

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
        PanelKind::new("editor")
    }

    fn display_name(&self) -> SharedString {
        "Editor".into()
    }

    fn icon(&self) -> Option<&'static str> {
        Some("icons/editor/panel.svg")
    }

    fn create_panel(
        &self,
        panel_id: PanelId,
        _host: Option<Arc<dyn PanelHost>>,
        cx: &mut App,
    ) -> Box<dyn PanelView> {
        let session = crate::EditorSession::empty();
        let editor = cx.new(|cx| Editor::with_session(panel_id, session, cx));
        Box::new(EditorPanelView::new(editor))
    }
}

