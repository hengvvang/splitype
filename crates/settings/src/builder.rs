use gpui::App;
use crate::state::SettingsUiState;

/// Fluent builder for the Settings panel.
pub struct SettingsBuilder {}

impl Default for SettingsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsBuilder {
    pub fn new() -> Self {
        Self {}
    }

    pub fn build(self, cx: &mut App) {
        cx.set_global(SettingsUiState::new());
    }
}
