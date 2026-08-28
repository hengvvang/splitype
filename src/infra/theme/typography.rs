//! Typography definitions (font families, sizes, weights).

use gpui::FontWeight;
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

/// All configurable typography settings (font sizes, weights, line heights).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeTypography {
    /// Default body text font size.
    pub text_size: f32,
    /// Default body text line height as a ratio of font size.
    pub text_line_height: f32,
    /// H1 heading font size.
    pub h1_size: f32,
    /// H1 heading font weight.
    pub h1_weight: FontWeightDef,
    /// H2 heading font size.
    pub h2_size: f32,
    /// H2 heading font weight.
    pub h2_weight: FontWeightDef,
    /// H3 heading font size.
    pub h3_size: f32,
    /// H3 heading font weight.
    pub h3_weight: FontWeightDef,
    /// H4 heading font size.
    pub h4_size: f32,
    /// H4 heading font weight.
    pub h4_weight: FontWeightDef,
    /// H5 heading font size.
    pub h5_size: f32,
    /// H5 heading font weight.
    pub h5_weight: FontWeightDef,
    /// H6 heading font size.
    pub h6_size: f32,
    /// H6 heading font weight.
    pub h6_weight: FontWeightDef,
    /// Code-block text font size.
    pub code_size: f32,
    /// Dialog title font size.
    pub dialog_title_size: f32,
    /// Dialog title font weight.
    pub dialog_title_weight: FontWeightDef,
    /// Dialog body font size.
    pub dialog_body_size: f32,
    /// Dialog body font weight.
    pub dialog_body_weight: FontWeightDef,
    /// Dialog button font size.
    pub dialog_button_size: f32,
    /// Dialog button font weight.
    pub dialog_button_weight: FontWeightDef,
}

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
            Self::Ui => "Font used for menus, explorer sidebar, and application chrome",
            Self::Prose => "Font used for Markdown prose, headings, and tables (both Editor and Preview)",
            Self::Code => "Monospace font used for code blocks and inline code",
        }
    }
}

/// Global typography store managing active font instances for Ui, Prose, and Code scopes.
pub struct TypographyStore {
    settings: crate::infra::config::settings::TypographySettings,
    cached_ui_font: gpui::Font,
    cached_prose_font: gpui::Font,
    cached_code_font: gpui::Font,
}

impl gpui::Global for TypographyStore {}

impl TypographyStore {
    pub fn init(cx: &mut gpui::App, settings: crate::infra::config::settings::TypographySettings) {
        let (ui, prose, code) = Self::resolve_fonts(&settings);
        cx.set_global(Self {
            settings,
            cached_ui_font: ui,
            cached_prose_font: prose,
            cached_code_font: code,
        });
    }

    pub fn update(cx: &mut gpui::App, settings: crate::infra::config::settings::TypographySettings) {
        let (ui, prose, code) = Self::resolve_fonts(&settings);
        cx.set_global(Self {
            settings,
            cached_ui_font: ui,
            cached_prose_font: prose,
            cached_code_font: code,
        });
        cx.refresh_windows();
    }

    pub fn settings(cx: &gpui::App) -> crate::infra::config::settings::TypographySettings {
        cx.try_global::<Self>()
            .map(|store| store.settings.clone())
            .unwrap_or_default()
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
        let (ui, prose, code) = Self::resolve_fonts(&crate::infra::config::settings::TypographySettings::default());
        match scope {
            TypographyScope::Ui => ui,
            TypographyScope::Prose => prose,
            TypographyScope::Code => code,
        }
    }

    fn resolve_fonts(
        settings: &crate::infra::config::settings::TypographySettings,
    ) -> (gpui::Font, gpui::Font, gpui::Font) {
        let ui_family = settings
            .ui_font_family
            .as_deref()
            .unwrap_or("Lexend");
        let prose_family = settings
            .prose_font_family
            .as_deref()
            .unwrap_or("Lexend");
        let code_family = settings.code_font_family.as_deref().unwrap_or_else(|| {
            if cfg!(target_os = "windows") {
                "Consolas"
            } else if cfg!(target_os = "macos") {
                "Menlo"
            } else {
                "monospace"
            }
        });

        (
            gpui::font(ui_family),
            gpui::font(prose_family),
            gpui::font(code_family),
        )
    }
}

/// Global cache of all installed and available font families on the system.
///
/// Enumerating fonts from the OS text system is done via `cx.text_system().all_font_names()`
/// and cached using `OnceLock` to provide instant searching and rendering in settings.
pub struct FontFamilyCache;

impl FontFamilyCache {
    pub fn list_font_families(cx: &gpui::App) -> Vec<gpui::SharedString> {
        static CACHED: std::sync::OnceLock<Vec<gpui::SharedString>> = std::sync::OnceLock::new();
        CACHED
            .get_or_init(|| {
                let mut font_names: Vec<String> = cx.text_system().all_font_names();
                font_names.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
                font_names.dedup();
                font_names
                    .into_iter()
                    .map(gpui::SharedString::from)
                    .collect()
            })
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typography_scope_properties() {
        assert_eq!(TypographyScope::ALL.len(), 3);
        assert_eq!(TypographyScope::Ui.display_label(), "Interface Font");
        assert_eq!(TypographyScope::Prose.display_label(), "Prose Text Font");
        assert_eq!(TypographyScope::Code.display_label(), "Code Block Font");
    }
}
