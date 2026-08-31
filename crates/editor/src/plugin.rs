use std::any::Any;
use gpui::*;
use workspace::{PanelDescriptor, PanelId, PanelKindId, PanelRenderContext, PanelView};
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
    fn kind(&self) -> PanelKindId {
        PanelKindId::EDITOR
    }

    fn display_name(&self) -> SharedString {
        "Editor".into()
    }

    fn icon(&self) -> Option<&'static str> {
        Some("icons/editor/panel.svg")
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
    fn kind(&self) -> PanelKindId {
        PanelKindId::EDITOR
    }

    fn display_name(&self) -> SharedString {
        "Editor".into()
    }

    fn icon(&self) -> Option<&'static str> {
        Some("icons/editor/panel.svg")
    }

    fn create_panel(&self, panel_id: PanelId, cx: &mut App) -> Box<dyn PanelView> {
        let session = crate::EditorSession::welcome();
        let editor = cx.new(|cx| Editor::with_session(panel_id, session, cx));
        Box::new(EditorPanelView::new(editor))
    }
}

