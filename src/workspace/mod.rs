//! Workspace module – consolidated config, theme, and i18n functionality.
//!
//! This module re-exports everything previously spread across `config`, `theme`,
//! and `i18n` so the rest of the codebase has a single import path.

mod locale;
mod settings;
mod storage;
mod theme;

// ── Settings ────────────────────────────────────────────────────────────────
pub(crate) use settings::{
    EditorSettings, ImagePasteBehavior, StartupOpenSetting, StatusBarButton, StatusBarSettings,
    apply_configured_language, apply_configured_theme, first_existing_recent_markdown_file,
    import_language_config_and_select, import_theme_config_and_select, load_or_create_app_settings,
    open_settings_window, read_app_settings,
};

// ── Storage ─────────────────────────────────────────────────────────────────
pub(crate) use storage::{read_recent_files, record_recent_file, remove_recent_file};

// ── Theme ───────────────────────────────────────────────────────────────────
pub use theme::{FontWeightDef, Theme, ThemeColors, ThemeDimensions, ThemeManager};

// ── Locale / i18n ───────────────────────────────────────────────────────────
pub use locale::{I18nManager, I18nStrings};
