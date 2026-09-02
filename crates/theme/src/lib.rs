//! Visual theme system — semantic color tokens, dimensions, typography,
//! the theme registry, the resolution pipeline, and the theme manager.
//!
//! # Architecture (single model, no legacy paths)
//!
//! 1. [`content`] — the one canonical JSONC file format. A theme *file*
//!    describes a [`ThemeFamilyContent`]: a named family of variant
//!    [`ThemeContent`]s, each carrying a partial [`ThemeStyleContent`]
//!    patch. Omitted sections and fields inherit from the variant's `base`
//!    chain.
//! 2. [`registry`] — every known family keyed by id with priority
//!    `user > plugin > builtin`, plus the plugin extension-token schema.
//! 3. [`resolve`] — the single merge pipeline: built-in default theme →
//!    base chain (root first, cycle-checked) → the target variant's own
//!    patch → settings color overrides.
//! 4. [`manager`] — the gpui global holding the registry and the resolved
//!    current theme. The manager never picks themes on its own:
//!    `ThemeSettingsContent` in the settings store is the single source of
//!    truth; settings writes flow through `SettingsStore` sync hooks.
//!
//! Color values in theme files and settings overrides are `#rrggbb[aa]` hex
//! strings. Parsing is gpui's own [`Hsla`] serde implementation — the one
//! color parser in the whole codebase.
//!
//! [`Hsla`]: gpui::Hsla

macro_rules! theme_section {
    // Plain section: full struct plus its all-optional patch counterpart.
    (
        plain
        $(#[$full_doc:meta])* struct $full:ident,
        $(#[$patch_doc:meta])* struct $patch:ident,
        { $( $(#[$field_doc:meta])* $field:ident : $ty:ty ),* $(,)? }
    ) => {
        $(#[$full_doc])*
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct $full {
            $( $(#[$field_doc])* pub $field: $ty, )*
        }

        $(#[$patch_doc])*
        #[derive(Debug, Clone, Default, Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct $patch {
            $(
                $(#[$field_doc])*
                #[serde(default, skip_serializing_if = "Option::is_none")]
                pub $field: Option<$ty>,
            )*
        }

        impl $patch {
            /// Overlays the set fields onto a full section in place.
            pub fn apply_to(&self, target: &mut $full) {
                $( if let Some(value) = &self.$field { target.$field = value.clone(); } )*
            }

            /// Converts a complete section into an all-set patch.
            pub fn from_full(full: &$full) -> Self {
                Self {
                    $( $field: Some(full.$field.clone()), )*
                }
            }
        }
    };

    // Color section: additionally exposes runtime color-token lookup used by
    // the resolution pipeline and the settings color-override panel.
    (
        colors
        $(#[$full_doc:meta])* struct $full:ident,
        $(#[$patch_doc:meta])* struct $patch:ident,
        { $( $(#[$field_doc:meta])* $field:ident : $ty:ty ),* $(,)? }
    ) => {
        $(#[$full_doc])*
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct $full {
            $( $(#[$field_doc])* pub $field: $ty, )*
        }

        $(#[$patch_doc])*
        #[derive(Debug, Clone, Default, Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct $patch {
            $(
                $(#[$field_doc])*
                #[serde(default, skip_serializing_if = "Option::is_none")]
                pub $field: Option<$ty>,
            )*
        }

        impl $patch {
            /// Overlays the set fields onto a full section in place.
            pub fn apply_to(&self, target: &mut $full) {
                $( if let Some(value) = &self.$field { target.$field = value.clone(); } )*
            }

            /// Converts a complete section into an all-set patch.
            pub fn from_full(full: &$full) -> Self {
                Self {
                    $( $field: Some(full.$field.clone()), )*
                }
            }
        }

        impl $full {
            /// Every color token field name, in declaration order.
            pub const TOKEN_FIELD_NAMES: &'static [&'static str] = &[
                $( stringify!($field) ),*
            ];

            /// Resolves a color token by field name.
            pub fn color_token(&self, field: &str) -> Option<gpui::Hsla> {
                match field {
                    $( stringify!($field) => Some(self.$field), )*
                    _ => None,
                }
            }

            /// Overwrites a color token by field name, reporting whether the
            /// token exists.
            pub fn set_color_token(&mut self, field: &str, value: gpui::Hsla) -> bool {
                match field {
                    $( stringify!($field) => { self.$field = value; true } )*
                    _ => false,
                }
            }
        }
    };
}

pub mod colors;
pub mod content;
pub mod dimensions;
pub mod manager;
pub mod registry;
pub mod resolve;
pub mod theme;
pub mod typography;

pub use colors::{ThemeColors, ThemeColorsPatch};
pub use config::settings::Appearance;
pub use content::{ThemeContent, ThemeFamilyContent, ThemeStyleContent, validate_family};
pub use dimensions::{ThemeDimensions, ThemeDimensionsPatch};
pub use manager::{
    BUILTIN_THEME_FAMILY_ID, ImportedTheme, ThemeManager, apply_theme_selection,
    import_theme_config_and_select,
};
pub use registry::{ThemeCatalogEntry, ThemeRegistry, TokenDeclaration};
pub use resolve::{ResolvedTheme, resolve_theme};
pub use theme::{CalloutStyle, HeadingStyle, Placeholders, PlaceholdersPatch, Theme};
pub use typography::{
    FontFamilyCache, FontWeightDef, ThemeTypography, ThemeTypographyPatch, TypographyScope,
    TypographyStore,
};
