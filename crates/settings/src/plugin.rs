use std::any::Any;
use gpui::*;
use workspace::{PanelDescriptor, PanelId, PanelKindId, PanelRenderContext, PanelView, WindowPanelKind};
use crate::{render_settings_body, render_settings_bottombar, render_settings_topbar};

/// View wrapper implementing [`PanelView`] for the Settings panel.
pub struct SettingsPanelView {
    pub panel_id: PanelId,
}

impl SettingsPanelView {
    pub fn new(panel_id: PanelId) -> Self {
        Self { panel_id }
    }
}

impl PanelView for SettingsPanelView {
    fn kind(&self) -> PanelKindId {
        PanelKindId::SETTINGS
    }

    fn display_name(&self) -> SharedString {
        "Settings".into()
    }

    fn icon(&self) -> Option<&'static str> {
        Some("icons/settings/panel.svg")
    }

    fn render(
        &mut self,
        ctx: &PanelRenderContext,
        _window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let theme = ctx.theme;
        let c = &theme.colors;
        let topbar = render_settings_topbar(
            ctx.panel_id,
            WindowPanelKind::Settings,
            theme,
            ctx.leaf_count,
            ctx.is_maximized,
            cx,
        );
        let body = render_settings_body(ctx.panel_id, theme, ctx.strings, cx);
        let bottombar = render_settings_bottombar(ctx.panel_id, theme, cx);

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

/// Panel descriptor for the Settings plugin.
#[derive(Clone, Debug, Default)]
pub struct SettingsPanelDescriptor {}

impl SettingsPanelDescriptor {
    pub fn new() -> Self {
        Self {}
    }
}

impl PanelDescriptor for SettingsPanelDescriptor {
    fn kind(&self) -> PanelKindId {
        PanelKindId::SETTINGS
    }

    fn display_name(&self) -> SharedString {
        "Settings".into()
    }

    fn icon(&self) -> Option<&'static str> {
        Some("icons/settings/panel.svg")
    }

    fn create_panel(&self, panel_id: PanelId, _cx: &mut App) -> Box<dyn PanelView> {
        Box::new(SettingsPanelView::new(panel_id))
    }
}
