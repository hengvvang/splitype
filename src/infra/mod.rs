//! System infrastructure — capabilities independent of document content.
//!
//! - `config` — configuration persistence (settings, recent files, keybinding
//!   schema, JSONC helpers).
//! - `i18n` — localization.
//! - `net` — network transport (remote image loading) and update checks.
//! - `theme` — the visual theme system (color tokens, dimensions, typography,
//!   and the `ThemeManager` service).
//!
//! Application-specific by nature: it stays in the main crate as a plain
//! module (no independent release, no other consumer), while the reusable
//! engines (`model`, `layout`) remain separate crates.

pub mod config;
pub mod error;
pub mod i18n;
pub mod net;
pub mod theme;

pub use error::*;
