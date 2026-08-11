//! Window panel state aggregate — the sidebar and tiled-layout state of a
//! window, owned by the Shell.
//!
//! Pure state records; rendering lives in `crate::app::window_area_layout`
//! (outer layout), `crate::explorer`, and `crate::settings`. The per-area
//! editor sessions and inner-panel operations live on each `Editor` entity
//! (see `crate::editor::session_ops`).

use crate::app::window_area::WindowLayout;
use crate::editor::explorer_state::state::ExplorerState;
use crate::editor::settings_state::SettingsUiState;

/// Sidebar and tiled-layout state of the window.
pub struct WindowPanels {
    pub(crate) explorer: ExplorerState,
    pub(crate) layout: WindowLayout,
    pub(crate) settings: SettingsUiState,
}

impl Default for WindowPanels {
    fn default() -> Self {
        Self {
            explorer: ExplorerState::default(),
            layout: crate::app::window_area::default_layout(),
            settings: SettingsUiState::default(),
        }
    }
}
