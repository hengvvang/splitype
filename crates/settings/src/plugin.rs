//! The Settings panel plugin — a thin host over the schema-driven settings
//! UI.
//!
//! The panel is a minimal shell: topbar chrome plus the shared settings body
//! rendered from manifest-declared settings schemas ([`host`]). It imports
//! no other plugin.

use crate::host::render_settings_body;
use crate::render_settings_topbar;
use crate::state::{PersistedSettingsState, SettingsUiState};
use gpui::*;
use platform_contracts::{PanelDescriptor, PanelId, PanelKind, PanelRenderContext, PanelView};
use std::any::Any;

/// Stable kind identifier of the settings panel plugin.
pub const PANEL_KIND: &str = "splitype.panel.settings";

/// Stable plugin identifier of the settings plugin.
pub const PLUGIN_ID: &str = "splitype.settings";

/// Asset directory holding the settings panel's topbar chrome icons.
pub const TOPBAR_ICON_PREFIX: &str = "plugin://splitype.settings";

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

    /// Reuses a live state entity (suspend/restore of a panel kind switch).
    pub fn with_state(panel_id: PanelId, state: Entity<SettingsUiState>) -> Self {
        Self { panel_id, state }
    }
}

impl PanelView for SettingsPanelView {
    fn kind(&self) -> PanelKind {
        PanelKind::from_static(PANEL_KIND)
    }

    /// Parks the live state entity when this panel kind switches away;
    /// restoring this kind hands the same entity back (current page,
    /// dropdowns, and edit buffers intact).
    fn suspend_state(&mut self, _cx: &mut App) -> Option<Box<dyn Any>> {
        Some(Box::new(self.state.clone()))
    }

    fn clone_state(&self, cx: &mut App) -> Option<Box<dyn Any>> {
        Some(Box::new(PersistedSettingsState {
            active_plugin: self.state.read(cx).active_plugin.clone(),
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
        let body = render_settings_body(&format!("panel-{}", ctx.panel_id.0), state, theme, cx);

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
        // A suspended live entity (panel kind switch) is reused as-is; a
        // durable projection (clone / window restore) rebuilds fresh UI
        // state carrying only the active plugin page.
        if state.is::<Entity<SettingsUiState>>() {
            let live = state
                .downcast::<Entity<SettingsUiState>>()
                .expect("just checked");
            return Some(Box::new(SettingsPanelView::with_state(panel_id, *live)));
        }
        let persisted = state.downcast::<PersistedSettingsState>().ok()?;
        let view = SettingsPanelView::new(panel_id, cx);
        let restored = SettingsUiState::from_persisted(&persisted);
        view.state.update(cx, |ui, _cx| *ui = restored);
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
