use crate::state::state::ExplorerState;
use crate::{render_explorer_body, render_explorer_bottombar, render_explorer_topbar};
use gpui::*;
use std::any::Any;
use std::sync::Arc;
use window::{PanelDescriptor, PanelHost, PanelId, PanelKind, PanelRenderContext, PanelView};

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
        PanelKind::new("explorer")
    }

    fn display_name(&self) -> SharedString {
        "Explorer".into()
    }

    fn icon(&self) -> Option<&'static str> {
        Some("icons/explorer/panel.svg")
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
            PanelKind::new("explorer"),
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
        Some(Box::new(self.state.read(cx).clone_for_new_window()))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
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
        PanelKind::new("explorer")
    }

    fn display_name(&self) -> SharedString {
        "Explorer".into()
    }

    fn icon(&self) -> Option<&'static str> {
        Some("icons/explorer/panel.svg")
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
        let state = state.downcast::<ExplorerState>().ok().map(|boxed| *boxed)?;
        let entity = cx.new(|cx| {
            let mut restored = state;
            restored.self_weak = cx.weak_entity();
            restored
        });
        Some(Box::new(ExplorerPanelView {
            panel_id,
            state: entity,
        }))
    }
}
