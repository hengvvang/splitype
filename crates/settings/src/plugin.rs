use crate::state::{PersistedSettingsState, SettingsUiState};
use crate::{render_settings_body, render_settings_bottombar, render_settings_topbar};
use platform_contracts::{PanelDescriptor, PanelId, PanelKind, PanelRenderContext, PanelView};
use gpui::*;
use std::any::Any;

/// Stable kind identifier of the settings panel plugin.
pub const PANEL_KIND: &str = "splitype.panel.settings";

/// Stable plugin identifier of the settings plugin.
pub const PLUGIN_ID: &str = "splitype.settings";

/// Asset directory holding the settings panel's topbar chrome icons.
pub const TOPBAR_ICON_PREFIX: &str = "icons/settings";

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
        PanelKind::from_static(PANEL_KIND)
    }

    fn clone_state(&self, cx: &mut App) -> Option<Box<dyn Any>> {
        Some(Box::new(PersistedSettingsState {
            tab: self.state.read(cx).tab,
        }))
    }

    fn display_name(&self) -> SharedString {
        "Settings".into()
    }

    fn icon(&self) -> Option<&'static str> {
        Some("plugin://splitype.settings/panel.svg")
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
            TOPBAR_ICON_PREFIX,
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
        PanelKind::from_static(PANEL_KIND)
    }

    fn display_name(&self) -> SharedString {
        "Settings".into()
    }

    fn icon(&self) -> Option<&'static str> {
        Some("plugin://splitype.settings/panel.svg")
    }

    fn create_panel(&self, panel_id: PanelId, cx: &mut App) -> Box<dyn PanelView> {
        Box::new(SettingsPanelView::new(panel_id, cx))
    }

    fn restore_panel(
        &self,
        panel_id: PanelId,
        state: Box<dyn Any>,
        cx: &mut App,
    ) -> Option<Box<dyn PanelView>> {
        let state = state
            .downcast::<PersistedSettingsState>()
            .ok()
            .map(|boxed| *boxed)?;
        let view = SettingsPanelView::new(panel_id, cx);
        view.state.update(cx, |ui, _cx| ui.tab = state.tab);
        Some(Box::new(view))
    }

    fn serialize_state(&self, state: &dyn Any) -> Option<serde_json::Value> {
        let state = state.downcast_ref::<PersistedSettingsState>()?;
        serde_json::to_value(state).ok()
    }

    fn deserialize_state(&self, json: &serde_json::Value) -> Option<Box<dyn Any>> {
        let state: PersistedSettingsState = serde_json::from_value(json.clone()).ok()?;
        Some(Box::new(state))
    }
}
