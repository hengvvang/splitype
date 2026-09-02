//! Theme file schema — the one canonical JSONC format for theme families.
//!
//! A theme *file* describes a family of variants. Every variant carries a
//! partial style patch: omitted sections and fields inherit from the
//! variant's `base` chain, which bottoms out at the built-in default theme.
//! Color values are `#rrggbb[aa]` hex strings, parsed by gpui's `Hsla`
//! serde implementation. Unknown keys are hard errors.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::bail;
use gpui::Hsla;
use serde::{Deserialize, Serialize};

use config::jsonc::sanitize_config_file_stem;
use config::settings::Appearance;

use super::colors::ThemeColorsPatch;
use super::dimensions::ThemeDimensionsPatch;
use super::theme::{PlaceholdersPatch, Theme};
use super::typography::ThemeTypographyPatch;

/// A theme family: one or more variants sharing a name and author.
///
/// The family name is also its registry id (sanitized); a family from the
/// user themes directory shadows any plugin or builtin family with the
/// same id.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeFamilyContent {
    /// Family display name (e.g. "Night Writer").
    pub name: String,
    /// Optional theme author, shown in the theme menu.
    #[serde(default)]
    pub author: String,
    /// The family's variants; at least one is required.
    pub themes: Vec<ThemeContent>,
}

/// One selectable variant of a family.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeContent {
    /// Variant display name (e.g. "Dark").
    pub name: String,
    /// Whether this variant is a dark or light theme.
    #[serde(default)]
    pub appearance: Appearance,
    /// Partial style patch; omitted sections inherit from the base chain.
    #[serde(default)]
    pub style: ThemeStyleContent,
}

/// Partial style patch.
///
/// `base` references another theme by id — a bare family id or a
/// `family.variant` id — resolved against the registry. `extension`
/// overrides plugin extension tokens keyed by token id. Unknown keys are
/// hard errors; empty/absent values inherit.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeStyleContent {
    /// Id of the base theme this patch builds on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colors: Option<ThemeColorsPatch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<ThemeDimensionsPatch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typography: Option<ThemeTypographyPatch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholders: Option<PlaceholdersPatch>,
    /// Overrides for plugin extension tokens keyed by token id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<BTreeMap<String, Hsla>>,
}

impl ThemeFamilyContent {
    /// Stable registry id derived from the family name (lowercased slug).
    pub fn id(&self) -> String {
        sanitize_config_file_stem(&self.name).to_lowercase()
    }
}

impl ThemeContent {
    /// Stable variant id derived from the variant name (lowercased slug).
    pub fn id(&self) -> String {
        sanitize_config_file_stem(&self.name).to_lowercase()
    }
}

impl ThemeStyleContent {
    /// Overlays the set sections onto a resolved theme in place.
    pub fn apply_to(&self, theme: &mut Theme) {
        if let Some(colors) = &self.colors {
            colors.apply_to(&mut theme.colors);
        }
        if let Some(dimensions) = &self.dimensions {
            dimensions.apply_to(&mut theme.dimensions);
        }
        if let Some(typography) = &self.typography {
            typography.apply_to(&mut theme.typography);
        }
        if let Some(placeholders) = &self.placeholders {
            placeholders.apply_to(&mut theme.placeholders);
        }
    }

    /// Builds an all-set patch from a full theme, used to seed built-in
    /// families so they flow through the same resolution pipeline as user
    /// themes.
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            base: None,
            colors: Some(ThemeColorsPatch::from_full(&theme.colors)),
            dimensions: Some(ThemeDimensionsPatch::from_full(&theme.dimensions)),
            typography: Some(ThemeTypographyPatch::from_full(&theme.typography)),
            placeholders: Some(PlaceholdersPatch::from_full(&theme.placeholders)),
            extension: Some(theme.extension.clone()),
        }
    }
}

/// Validates the family invariants shared by imports and registry inserts.
pub fn validate_family(family: &ThemeFamilyContent) -> anyhow::Result<()> {
    if family.name.trim().is_empty() {
        bail!("theme family name must not be empty");
    }
    if family.themes.is_empty() {
        bail!(
            "theme family '{}' must declare at least one theme",
            family.name
        );
    }
    let mut seen = BTreeSet::new();
    for theme in &family.themes {
        if theme.name.trim().is_empty() {
            bail!(
                "theme family '{}' has a variant with an empty name",
                family.name
            );
        }
        let id = theme.id();
        if !seen.insert(id.clone()) {
            bail!(
                "theme family '{}' declares duplicate variant '{id}'",
                family.name
            );
        }
    }
    Ok(())
}
