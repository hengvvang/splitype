pub mod manager;
pub mod packs;
pub mod strings;

pub use manager::{apply_configured_language, import_language_config_and_select, I18nManager};
pub use packs::LanguageId;
pub use strings::I18nStrings;

