//! Visual theme system — color tokens, dimensions, typography, and the
//! theme manager service.
//!
//! Theme persistence reads configuration through `config`; keeping the two
//! in one layer makes the dependency direction unambiguous. `ThemeManager`
//! is an app-level gpui `Global`: it loads, switches, and persists themes.
//! The UI component layer (`ui`) consumes this module exclusively.

pub mod colors;
pub mod dimensions;
pub mod manager;
pub mod theme;
pub mod typography;

pub use colors::ThemeColors;
pub use dimensions::ThemeDimensions;
pub use manager::{apply_configured_theme, import_theme_config_and_select, ThemeManager};
pub use theme::{Theme, ThemeCatalogEntry};
pub use typography::{FontFamilyCache, FontWeightDef, TypographyScope, TypographyStore};

