//! Settings — the standalone settings window and the in-window settings
//! panel.
//!
//! The panel is a window-shell leaf like the explorer: it renders through
//! free functions against the [`SettingsUiState`] gpui global, and its
//! topbar dispatches `workspace` layout actions that the shell handles.
//! The panel never references the shell.

pub mod bottombar;
pub(crate) mod components;
pub mod panel;
pub mod plugin;
pub(crate) mod state;
pub mod topbar;
pub(crate) mod ui_helpers;
pub(crate) mod window;

pub use bottombar::render_settings_bottombar;
pub use panel::render_settings_body;
pub use plugin::*;
pub use topbar::render_settings_topbar;
pub use state::SettingsUiState;
pub use window::open_settings_window;
