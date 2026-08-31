use gpui::App;
use crate::state::state::ExplorerState;

/// Fluent builder for the Explorer panel.
pub struct ExplorerBuilder {}

impl Default for ExplorerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ExplorerBuilder {
    pub fn new() -> Self {
        Self {}
    }

    pub fn build(self, cx: &mut App) {
        cx.set_global(ExplorerState::default());
    }
}
