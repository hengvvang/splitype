//! Typography definitions (font families, sizes, weights).

use gpui::{BorrowAppContext, FontWeight};
use serde::{Deserialize, Serialize};

/// Serializable font weight that maps to GPUI's [`FontWeight`] constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontWeightDef {
    /// Thin font weight.
    Thin,
    /// Light font weight.
    Light,
    /// Normal font weight.
    Normal,
    /// Medium font weight.
    Medium,
    /// Semibold font weight.
    Semibold,
    /// Bold font weight.
    Bold,
    /// Extra-bold font weight.
    Extrabold,
    /// Black font weight.
    Black,
}

impl From<FontWeightDef> for FontWeight {
    #[inline]
    fn from(def: FontWeightDef) -> Self {
        match def {
            FontWeightDef::Thin => FontWeight::THIN,
            FontWeightDef::Light => FontWeight::LIGHT,
            FontWeightDef::Normal => FontWeight::NORMAL,
            FontWeightDef::Medium => FontWeight::MEDIUM,
            FontWeightDef::Semibold => FontWeight::SEMIBOLD,
            FontWeightDef::Bold => FontWeight::BOLD,
            FontWeightDef::Extrabold => FontWeight::EXTRA_BOLD,
            FontWeightDef::Black => FontWeight::BLACK,
        }
    }
}

impl From<&FontWeightDef> for FontWeight {
    #[inline]
    fn from(def: &FontWeightDef) -> Self {
        FontWeight::from(*def)
    }
}

impl FontWeightDef {
    /// Converts the serialized theme value into GPUI's runtime font weight.
    #[inline]
    pub fn to_font_weight(&self) -> FontWeight {
        FontWeight::from(*self)
    }
}

theme_section!(
    plain
    /// All configurable typography settings (font sizes, weights, line heights).
    struct ThemeTypography,
    /// Partial typography overrides; every `None` field inherits from the base
    /// theme during resolution.
    struct ThemeTypographyPatch,
    {
        /// Default body text font size.
        text_size: f32,
        /// Default body text line height as a ratio of font size.
        text_line_height: f32,
        /// H1 heading font size.
        h1_size: f32,
        /// H1 heading font weight.
        h1_weight: FontWeightDef,
        /// H2 heading font size.
        h2_size: f32,
        /// H2 heading font weight.
        h2_weight: FontWeightDef,
        /// H3 heading font size.
        h3_size: f32,
        /// H3 heading font weight.
        h3_weight: FontWeightDef,
        /// H4 heading font size.
        h4_size: f32,
        /// H4 heading font weight.
        h4_weight: FontWeightDef,
        /// H5 heading font size.
        h5_size: f32,
        /// H5 heading font weight.
        h5_weight: FontWeightDef,
        /// H6 heading font size.
        h6_size: f32,
        /// H6 heading font weight.
        h6_weight: FontWeightDef,
        /// Code-block text font size.
        code_size: f32,
        /// Dialog title font size.
        dialog_title_size: f32,
        /// Dialog title font weight.
        dialog_title_weight: FontWeightDef,
        /// Dialog body font size.
        dialog_body_size: f32,
        /// Dialog body font weight.
        dialog_body_weight: FontWeightDef,
        /// Dialog button font size.
        dialog_button_size: f32,
        /// Dialog button font weight.
        dialog_button_weight: FontWeightDef,
    }
);

/// The numeric (size) typography fields, in declaration order.
pub const TYPOGRAPHY_SIZE_FIELDS: &[&str] = &[
    "text_size",
    "text_line_height",
    "h1_size",
    "h2_size",
    "h3_size",
    "h4_size",
    "h5_size",
    "h6_size",
    "code_size",
    "dialog_title_size",
    "dialog_body_size",
    "dialog_button_size",
];

/// The font-weight typography fields, in declaration order.
pub const TYPOGRAPHY_WEIGHT_FIELDS: &[&str] = &[
    "h1_weight",
    "h2_weight",
    "h3_weight",
    "h4_weight",
    "h5_weight",
    "h6_weight",
    "dialog_title_weight",
    "dialog_body_weight",
    "dialog_button_weight",
];

/// Three-tier typography scopes (Application Chrome UI, Document Prose, and Syntax Code).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypographyScope {
    /// 1. Application Chrome / Interface controls (Shell, Titlebar, Menus, Explorer, Settings, Statusbar)
    Ui,
    /// 2. Document Prose (Markdown text, Headings H1~H6, Blockquotes, Lists, Tables across Editor and Preview)
    Prose,
    /// 3. Syntax / Monospace (Fenced Code Blocks, Inline Code, YAML Frontmatter)
    Code,
}

impl TypographyScope {
    pub const ALL: [Self; 3] = [Self::Ui, Self::Prose, Self::Code];

    pub fn display_label(&self) -> &'static str {
        match self {
            Self::Ui => "Interface Font",
            Self::Prose => "Prose Text Font",
            Self::Code => "Code Block Font",
        }
    }

    pub fn display_description(&self) -> &'static str {
        match self {
            Self::Ui => "Font used for menus, file navigation panel, and application chrome",
            Self::Prose => {
                "Font used for Markdown prose, headings, and tables (both Editor and Preview)"
            }
            Self::Code => "Monospace font used for code blocks and inline code",
        }
    }
}

/// Global typography store managing active font instances for Ui, Prose, and Code scopes.
pub struct TypographyStore {
    settings: config::settings::TypographySettings,
    cached_ui_font: gpui::Font,
    cached_prose_font: gpui::Font,
    cached_code_font: gpui::Font,
}

impl gpui::Global for TypographyStore {}

impl TypographyStore {
    /// Installs the store, resolving fonts from the settings store's
    /// typography settings. `SettingsStore` must be initialized first.
    pub fn init(cx: &mut gpui::App) {
        let settings = config::settings::PluginSettings::<config::settings::CoreSettings>::get(cx)
            .typography
            .clone();
        let (ui, prose, code) = Self::resolve_fonts(&settings);
        cx.set_global(Self {
            settings,
            cached_ui_font: ui,
            cached_prose_font: prose,
            cached_code_font: code,
        });
    }

    /// Registers the settings sync hook that keeps the active fonts in lock
    /// step with the settings store. Call once during application bootstrap.
    pub fn register_settings_sync_hook() {
        config::settings::SettingsStore::register_sync_hook(|cx, settings| {
            let typography = settings
                .plugin_settings::<config::settings::CoreSettings>()
                .typography;
            cx.update_global::<TypographyStore, _>(|store, _cx| {
                store.apply_settings(&typography);
            });
        });
    }

    /// Applies the given settings snapshot, returning whether it changed.
    pub fn apply_settings(&mut self, settings: &config::settings::TypographySettings) -> bool {
        if settings == &self.settings {
            return false;
        }
        let (ui, prose, code) = Self::resolve_fonts(settings);
        self.settings = settings.clone();
        self.cached_ui_font = ui;
        self.cached_prose_font = prose;
        self.cached_code_font = code;
        true
    }

    pub fn font(cx: &gpui::App, scope: TypographyScope) -> gpui::Font {
        if let Some(store) = cx.try_global::<Self>() {
            match scope {
                TypographyScope::Ui => store.cached_ui_font.clone(),
                TypographyScope::Prose => store.cached_prose_font.clone(),
                TypographyScope::Code => store.cached_code_font.clone(),
            }
        } else {
            Self::default_font(scope)
        }
    }

    #[inline]
    pub fn ui_font(cx: &gpui::App) -> gpui::Font {
        Self::font(cx, TypographyScope::Ui)
    }

    #[inline]
    pub fn prose_font(cx: &gpui::App) -> gpui::Font {
        Self::font(cx, TypographyScope::Prose)
    }

    #[inline]
    pub fn code_font(cx: &gpui::App) -> gpui::Font {
        Self::font(cx, TypographyScope::Code)
    }

    pub fn default_font(scope: TypographyScope) -> gpui::Font {
        let (ui, prose, code) =
            Self::resolve_fonts(&config::settings::TypographySettings::default());
        match scope {
            TypographyScope::Ui => ui,
            TypographyScope::Prose => prose,
            TypographyScope::Code => code,
        }
    }

    fn resolve_fonts(
        settings: &config::settings::TypographySettings,
    ) -> (gpui::Font, gpui::Font, gpui::Font) {
        let ui_family = if settings.ui_font_family.is_empty() {
            "Lexend"
        } else {
            settings.ui_font_family.as_str()
        };
        let prose_family = if settings.prose_font_family.is_empty() {
            "Lexend"
        } else {
            settings.prose_font_family.as_str()
        };
        let code_family = if settings.code_font_family.is_empty() {
            if cfg!(target_os = "windows") {
                "Consolas"
            } else if cfg!(target_os = "macos") {
                "Menlo"
            } else {
                "monospace"
            }
        } else {
            settings.code_font_family.as_str()
        };

        (
            gpui::font(ui_family),
            gpui::font(prose_family),
            gpui::font(code_family),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typography_store_applies_settings_snapshots() {
        let mut store = TypographyStore {
            settings: config::settings::TypographySettings::default(),
            cached_ui_font: gpui::font("Arial"),
            cached_prose_font: gpui::font("Arial"),
            cached_code_font: gpui::font("Consolas"),
        };
        // The same snapshot re-applies without re-resolving fonts.
        assert!(!store.apply_settings(&config::settings::TypographySettings::default()));
        let settings = config::settings::TypographySettings {
            code_font_family: "Menlo".into(),
            ..Default::default()
        };
        assert!(store.apply_settings(&settings));
        assert_eq!(store.settings.code_font_family, "Menlo");
        assert!(!store.apply_settings(&settings));
    }
}
