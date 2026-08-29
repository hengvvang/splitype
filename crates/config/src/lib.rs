//! Application configuration and localization: settings, recent files,
//! keybindings, JSONC helpers, and the language manager.

pub mod dirs;
pub mod jsonc;
pub mod keybindings;
pub mod language;
pub mod recent;
pub mod settings;

pub use language::{I18nManager, I18nStrings, LanguageId};
