use crate::bottombar::render_explorer_bottombar;
use crate::persist::PersistedExplorerState;
use crate::render::{render_explorer_body, render_explorer_file_context_menu};
use crate::state::ExplorerState;
use crate::topbar::render_explorer_topbar;
use gpui::*;
use platform_contracts::{PanelDescriptor, PanelId, PanelKind, PanelRenderContext, PanelView};
use std::any::Any;
use std::path::PathBuf;
use theme::ThemeManager;

/// Stable kind identifier of the explorer panel plugin.
pub const PANEL_KIND: &str = "splitype.panel.explorer";

/// Stable plugin identifier of the explorer plugin.
pub const PLUGIN_ID: &str = "splitype.explorer";

/// Asset directory holding the explorer panel's topbar chrome icons.
pub const TOPBAR_ICON_PREFIX: &str = "plugin://splitype.explorer";

/// View wrapper implementing [`PanelView`] for the Explorer file-tree panel.
pub struct ExplorerPanelView {
    pub panel_id: PanelId,
    /// The panel's own explorer view-state entity (one per panel instance);
    /// the scanned trees it shows are process-level shared resources from
    /// the worktree store.
    pub state: Entity<ExplorerState>,
}

impl ExplorerPanelView {
    pub fn new(panel_id: PanelId, cx: &mut App) -> Self {
        Self {
            panel_id,
            state: ExplorerState::entity(cx),
        }
    }

    /// Reuses a live state entity (suspend/restore of a panel kind switch).
    pub fn with_state(panel_id: PanelId, state: Entity<ExplorerState>) -> Self {
        Self { panel_id, state }
    }
}

impl PanelView for ExplorerPanelView {
    fn kind(&self) -> PanelKind {
        PanelKind::from_static(PANEL_KIND)
    }

    fn render_overlay(&mut self, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let theme = cx.global::<ThemeManager>().current_arc();
        render_explorer_file_context_menu(&self.state, &theme, window.viewport_size(), cx)
    }

    fn dismiss_overlays(&mut self, cx: &mut App) -> bool {
        self.state.update(cx, |state, _cx| {
            let menu = state.file_menu.take().is_some();
            let bb = std::mem::take(&mut state.bottombar_menu_open);
            menu || bb
        })
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

    /// Parks the live state entity when this panel kind switches away; the
    /// shared worktrees stay registered in the store, and restoring this
    /// kind hands the same state (expansion, selection included) back.
    fn suspend_state(&mut self, _cx: &mut App) -> Option<Box<dyn Any>> {
        Some(Box::new(self.state.clone()))
    }

    fn clone_state(&self, cx: &mut App) -> Option<Box<dyn Any>> {
        let state = self.state.read(cx);
        let open_folders = state
            .worktrees
            .iter()
            .map(|worktree| worktree.read(cx).root().to_path_buf())
            .collect();
        Some(Box::new(PersistedExplorerState {
            tree_visible: state.tree_visible,
            open_folders,
        }))
    }

    /// The explorer holds no dirty content: discarding a panel simply
    /// releases its tree views (idempotent — trees drain on first call).
    fn discard_changes(&mut self, cx: &mut App) {
        self.state
            .update(cx, |state, cx| state.release_worktrees(cx));
    }

    /// Releases every tree view of this panel ahead of teardown.
    fn release_documents(&mut self, cx: &mut App) {
        self.state
            .update(cx, |state, cx| state.release_worktrees(cx));
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Pushes the active document path into a panel view of this plugin's kind.
/// Registered with the composition root's explorer hooks.
pub fn set_active_document_path(view: &mut dyn PanelView, path: Option<PathBuf>, cx: &mut App) {
    if let Some(view) = view.as_any_mut().downcast_mut::<ExplorerPanelView>() {
        view.state.update(cx, |state, _cx| state.active_file = path);
    }
}

/// Notifies a panel view of this plugin's kind that a document's backing
/// path changed.
pub fn on_document_path_changed(view: &mut dyn PanelView, cx: &mut App) {
    if let Some(view) = view.as_any_mut().downcast_mut::<ExplorerPanelView>() {
        view.state.update(cx, |state, cx| {
            state.sync_explorer_after_document_path_change(cx)
        });
    }
}

/// Toggles the file tree of a panel view of this plugin's kind.
pub fn toggle_tree(view: &mut dyn PanelView, window: &mut Window, cx: &mut App) {
    if let Some(view) = view.as_any_mut().downcast_mut::<ExplorerPanelView>() {
        view.state
            .update(cx, |state, cx| state.toggle_tree(window, cx));
    }
}

/// Closes the open folder scope of a panel view of this plugin's kind.
pub fn close_folder_scope(view: &mut dyn PanelView, cx: &mut App) {
    if let Some(view) = view.as_any_mut().downcast_mut::<ExplorerPanelView>() {
        view.state
            .update(cx, |state, cx| state.close_folder_scope(cx));
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

    fn display_name(&self) -> SharedString {
        "Explorer".into()
    }

    fn icon(&self) -> Option<&'static str> {
        Some("plugin://splitype.explorer/panel.svg")
    }

    fn create_panel(&self, panel_id: PanelId, cx: &mut App) -> Box<dyn PanelView> {
        Box::new(ExplorerPanelView::new(panel_id, cx))
    }

    fn restore_panel(
        &self,
        panel_id: PanelId,
        state: Box<dyn Any>,
        cx: &mut App,
    ) -> Option<Box<dyn PanelView>> {
        // Two distinct sources converge on one live model: a suspended live
        // state entity (panel kind switch) keeps its registered tree views,
        // while a durable projection (clone / window restore) resolves its
        // folders through the shared worktree store.
        if state.is::<Entity<ExplorerState>>() {
            let live = state
                .downcast::<Entity<ExplorerState>>()
                .expect("just checked");
            return Some(Box::new(ExplorerPanelView::with_state(panel_id, *live)));
        }
        let persisted = state.downcast::<PersistedExplorerState>().ok()?;
        let view = ExplorerPanelView::new(panel_id, cx);
        view.state.update(cx, |explorer, cx| {
            explorer.tree_visible = persisted.tree_visible;
            for path in persisted.open_folders {
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

    /// Releases the tree views held inside a suspended live state (panel
    /// teardown); shared trees lose only this panel's reference.
    fn release_retained(&self, state: &mut Box<dyn Any>, cx: &mut App) {
        if let Some(entity) = state.downcast_ref::<Entity<ExplorerState>>() {
            entity.update(cx, |state, cx| state.release_worktrees(cx));
        }
    }

    /// The explorer has no dirty content to discard — discarding a retained
    /// panel releases its tree views.
    fn discard_retained(&self, state: &mut Box<dyn Any>, cx: &mut App) {
        self.release_retained(state, cx);
    }
}
