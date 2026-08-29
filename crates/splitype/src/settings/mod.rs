//! Settings — the standalone settings window and the in-editor settings
//! panel.

pub(crate) mod bottombar;
pub(crate) mod components;
pub(crate) mod panel;
pub(crate) mod state;
pub(crate) mod topbar;
pub(crate) mod ui_helpers;
pub(crate) mod window;

pub(crate) use window::open_settings_window;
