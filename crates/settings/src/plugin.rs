use std::any::Any;
use std::sync::Arc;
use gpui::*;
use window::{PanelDescriptor, PanelHost, PanelId, PanelKind, PanelRenderContext, PanelView};
use crate::state::SettingsUiState;
use crate::{render_settings_body, render_settings_bottombar, render_settings_topbar};

/// View wrapper implementing [`PanelView`] for the Settings panel.
pub struct SettingsPanelView {
    pub panel_id: PanelId,
    pub state: Entity<SettingsUiState>,
}

impl SettingsPanelView {
    pub fn new(panel_id: PanelId, cx: &mut App) -> Self {
        Self {
            panel_id,
            state: cx.new(|_cx| SettingsUiState::new()),
        }
    }
}

impl PanelView for SettingsPanelView {
    fn kind(&self) -> PanelKind {
        PanelKind::new("settings")
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
        let state = self.state.clone();
        let topbar = render_settings_topbar(
            ctx.panel_id,
            PanelKind::new("settings"),
            theme,
            ctx.leaf_count,
            ctx.is_maximized,
            cx,
        );
        let body = render_settings_body(ctx.panel_id, state, theme, ctx.strings, cx);
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
    fn kind(&self) -> PanelKind {
        PanelKind::new("settings")
    }

    fn display_name(&self) -> SharedString {
        "Settings".into()
    }

    fn icon(&self) -> Option<&'static str> {
        Some("icons/settings/panel.svg")
    }

    fn create_panel(
        &self,
        panel_id: PanelId,
        _host: Arc<dyn PanelHost>,
        cx: &mut App,
    ) -> Box<dyn PanelView> {
        Box::new(SettingsPanelView::new(panel_id, cx))
    }
}
