use crate::persist::PersistedExplorerState;
use crate::state::state::ExplorerState;
use crate::{
    render_explorer_body, render_explorer_bottombar, render_explorer_file_context_menu,
    render_explorer_topbar,
};
use core_contracts::{PanelCapabilities, SidebarPanel};
use core_contracts::{
    PanelDescriptor, PanelHost, PanelId, PanelKind, PanelRenderContext, PanelView,
};
use gpui::*;
use std::any::Any;
use std::path::PathBuf;
use std::sync::Arc;
use theme::ThemeManager;

/// Stable kind identifier of the explorer panel plugin.
pub const PANEL_KIND: &str = "splitype.panel.explorer";

/// Stable plugin identifier of the explorer plugin.
pub const PLUGIN_ID: &str = "splitype.explorer";

/// Asset directory holding the explorer panel's topbar chrome icons.
pub const TOPBAR_ICON_PREFIX: &str = "icons/explorer";

/// View wrapper implementing [`PanelView`] for the Explorer sidebar.
pub struct ExplorerPanelView {
    pub panel_id: PanelId,
    /// The panel's own explorer state entity (one per panel instance, so
    /// splits and multi-window panels never share tree state).
    pub state: Entity<ExplorerState>,
}

impl ExplorerPanelView {
    pub fn new(panel_id: PanelId, cx: &mut App) -> Self {
        Self {
            panel_id,
            state: ExplorerState::entity(cx),
        }
    }
}

impl PanelView for ExplorerPanelView {
    fn kind(&self) -> PanelKind {
        PanelKind::from_static(PANEL_KIND)
    }

    fn capabilities(&self) -> PanelCapabilities {
        PanelCapabilities::SIDEBAR
    }

    fn as_sidebar_panel(&self) -> Option<&dyn SidebarPanel> {
        Some(self)
    }

    fn as_sidebar_panel_mut(&mut self) -> Option<&mut dyn SidebarPanel> {
        Some(self)
    }

    fn render_overlay(&mut self, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let theme = cx.global::<ThemeManager>().current_arc();
        render_explorer_file_context_menu(&self.state, &theme, window.viewport_size(), cx)
    }

    fn dismiss_overlays(&mut self, cx: &mut App) -> bool {
        self.state
            .update(cx, |state, _cx| state.file_menu.take().is_some())
    }

    fn display_name(&self) -> SharedString {
        "Explorer".into()
    }

    fn icon(&self) -> Option<&'static str> {
        Some("plugin://splitype.explorer/panel.svg")
    }

    fn render(
        &mut self,
        ctx: &PanelRenderContext,
        _window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let theme = ctx.theme;
        let c = &theme.colors;
        let topbar = render_explorer_topbar(
            ctx.panel_id,
            TOPBAR_ICON_PREFIX,
            theme,
            ctx.leaf_count,
            ctx.is_maximized,
            cx,
        );
        let body = render_explorer_body(ctx.panel_id, &self.state, theme, ctx.strings, cx);
        let bottombar = render_explorer_bottombar(ctx.panel_id, &self.state, theme, cx);

        div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .bg(c.editor_background)
            .child(topbar)
            .child(
                div()
                    .w_full()
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_hidden()
                    .child(body),
            )
            .child(bottombar)
            .into_any_element()
    }

    fn clone_state(&self, cx: &mut App) -> Option<Box<dyn Any>> {
        let state = self.state.read(cx);
        let open_folders = state
            .worktrees
            .iter()
            .map(|worktree| worktree.read(cx).root().to_path_buf())
            .collect();
        Some(Box::new(PersistedExplorerState {
            is_open: state.is_open,
            open_folders,
        }))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl SidebarPanel for ExplorerPanelView {
    fn set_active_document_path(&mut self, path: Option<PathBuf>, cx: &mut App) {
        self.state.update(cx, |state, _cx| state.active_file = path);
    }

    fn on_document_path_changed(&mut self, cx: &mut App) {
        self.state.update(cx, |state, cx| {
            state.sync_explorer_after_document_path_change(cx)
        });
    }

    fn toggle_drawer(&mut self, window: &mut Window, cx: &mut App) {
        self.state.update(cx, |state, cx| {
            state.toggle_explorer_drawer(window, cx);
        });
    }

    fn close_active_folder(&mut self, cx: &mut App) {
        self.state
            .update(cx, |state, cx| state.close_explorer_folder(cx));
    }
}

/// Panel descriptor for the Explorer plugin.
#[derive(Clone, Debug, Default)]
pub struct ExplorerPanelDescriptor {}

impl ExplorerPanelDescriptor {
    pub fn new() -> Self {
        Self {}
    }
}

impl PanelDescriptor for ExplorerPanelDescriptor {
    fn kind(&self) -> PanelKind {
        PanelKind::from_static(PANEL_KIND)
    }

    fn capabilities(&self) -> PanelCapabilities {
        PanelCapabilities::SIDEBAR
    }

    fn display_name(&self) -> SharedString {
        "Explorer".into()
    }

    fn icon(&self) -> Option<&'static str> {
        Some("plugin://splitype.explorer/panel.svg")
    }

    fn create_panel(
        &self,
        panel_id: PanelId,
        _host: Arc<dyn PanelHost>,
        cx: &mut App,
    ) -> Box<dyn PanelView> {
        Box::new(ExplorerPanelView::new(panel_id, cx))
    }

    fn restore_panel(
        &self,
        panel_id: PanelId,
        _host: Arc<dyn PanelHost>,
        state: Box<dyn Any>,
        cx: &mut App,
    ) -> Option<Box<dyn PanelView>> {
        let state = state
            .downcast::<PersistedExplorerState>()
            .ok()
            .map(|boxed| *boxed)?;
        let view = ExplorerPanelView::new(panel_id, cx);
        view.state.update(cx, |explorer, cx| {
            explorer.is_open = state.is_open;
            for path in state.open_folders {
                explorer.restore_worktree(path, cx);
            }
        });
        Some(Box::new(view))
    }

    fn serialize_state(&self, state: &dyn Any) -> Option<serde_json::Value> {
        let state = state.downcast_ref::<PersistedExplorerState>()?;
        serde_json::to_value(state).ok()
    }

    fn deserialize_state(&self, json: &serde_json::Value) -> Option<Box<dyn Any>> {
        let state: PersistedExplorerState = serde_json::from_value(json.clone()).ok()?;
        Some(Box::new(state))
    }
}
