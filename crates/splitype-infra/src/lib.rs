//! System infrastructure — capabilities independent of document content.
//!
//! - `config` — configuration persistence (settings, recent files, keybinding
//!   schema, JSONC helpers).
//! - `i18n` — localization.
//! - `net` — network transport (remote image loading) and update checks.
//! - `theme` — the visual theme system (color tokens, dimensions, typography,
//!   and the `ThemeManager` service). Lived at the crate root until the
//!   theme ⇄ config persistence cycle was resolved by absorbing it here;
//!   consumed by every layer above.
//!
//! Everything here depends only on gpui and other `infra` submodules.

pub mod config;
pub mod i18n;
pub mod net;
pub mod theme;
