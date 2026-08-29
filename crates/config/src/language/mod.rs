//! Localization: language packs, the runtime language manager and the
//! string tables.
//!
//! The language preference lives in `crate::settings`
//! (`interface.language_id`); this module owns the language catalog
//! (builtin + imported packs), the global `I18nManager` and the
//! en-US / zh-CN string tables.

pub mod manager;
pub mod packs;
pub mod strings;

pub use manager::{apply_configured_language, import_language_config_and_select, I18nManager};
pub use packs::LanguageId;
pub use strings::I18nStrings;
