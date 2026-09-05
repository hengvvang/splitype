//! Settings — the schema-driven settings host, exposed as both a standalone
//! window and an in-window tiled panel.
//!
//! Both surfaces are thin shells over [`host`], which renders the settings
//! UI entirely from the plugin registry's manifest-declared settings
//! schemas: navigation from the plugin names, one control per
//! [`SettingDeclaration`] dispatched on its [`SettingKind`], and values
//! read/written through the canonical `SettingsStore`. The host imports no
//! other plugin — plugins contribute settings purely as manifest data.

pub mod assets;
pub mod form;
pub mod host;
pub mod plugin;
pub mod state;
pub mod topbar;
pub use host::render_settings_body;
pub use plugin::*;
pub use state::SettingsUiState;
pub use topbar::render_settings_topbar;
