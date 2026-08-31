use std::any::Any;
use std::sync::Arc;
use gpui::*;
use window::{PanelDescriptor, PanelHost, PanelId, PanelKind, PanelRenderContext, PanelView};
use crate::{render_explorer_body, render_explorer_bottombar, render_explorer_topbar};

/// View wrapper implementing [`PanelView`] for the Explorer sidebar.
pub struct ExplorerPanelView {
    pub panel_id: PanelId,
}

impl ExplorerPanelView {
    pub fn new(panel_id: PanelId) -> Self {
        Self { panel_id }
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
        let body = render_explorer_body(ctx.panel_id, theme, ctx.strings, cx);
        let bottombar = render_explorer_bottombar(ctx.panel_id, theme, cx);

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
        _host: Option<Arc<dyn PanelHost>>,
        _cx: &mut App,
    ) -> Box<dyn PanelView> {
        Box::new(ExplorerPanelView::new(panel_id))
    }
}
