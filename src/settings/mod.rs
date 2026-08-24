//! Settings — the standalone settings window and the in-editor settings
//! panel.

pub(crate) mod bottombar;
pub(crate) mod common;
pub(crate) mod components;
pub(crate) mod panel;
pub(crate) mod shortcuts_data;
pub(crate) mod state;
pub(crate) mod topbar;
pub(crate) mod window;

pub(crate) use window::open_settings_window;
