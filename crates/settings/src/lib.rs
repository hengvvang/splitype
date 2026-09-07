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
pub mod bottombar;
pub mod form;
pub mod host;
pub mod plugin;
pub mod state;
pub mod topbar;
pub use bottombar::render_settings_bottombar;
pub use form::settings_card_row;
pub use host::{render_settings_body, COMPACT_BREAKPOINT};
pub use plugin::*;
pub use state::SettingsUiState;
pub use topbar::render_settings_topbar;

pub const MANIFEST_TOML: &str = include_str!("../manifest.toml");
