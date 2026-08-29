//! Window panel state aggregate.
//!
//! The panel-kind vocabulary, panel ids, and the window-level split root
//! live in `workspace`; this module owns the *per-panel state instances*
//! held by the Shell: the explorer state (defined in `crates/explorer`)
//! and the window layout tree. The settings panel state is a gpui
//! `Global` in `crates/settings` and is not owned here.

use workspace::WindowLayout;

use crate::explorer::state::state::ExplorerState;

/// Sidebar and tiled-layout state of the window.
pub struct WindowPanels {
    pub(crate) explorer: ExplorerState,
    pub(crate) layout: WindowLayout,
}

impl Default for WindowPanels {
    fn default() -> Self {
        Self {
            explorer: ExplorerState::default(),
            layout: workspace::default_layout(),
        }
    }
}
