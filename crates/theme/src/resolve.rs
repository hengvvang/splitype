//! Theme resolution — the single merge pipeline from content to runtime theme.

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use anyhow::{Context as _, bail};
use gpui::Hsla;
use serde_json::Map;

use config::settings::{Appearance, ThemeSettingsContent};

use super::content::{ThemeContent, ThemeStyleContent};
use super::dimensions::ThemeDimensionsPatch;
use super::registry::{ThemeRegistry, TokenDeclaration};
use super::theme::Theme;
use super::typography::ThemeTypographyPatch;

/// A fully resolved theme plus its concrete id.
#[derive(Debug)]
pub struct ResolvedTheme {
    /// Concrete theme id: `family.variant`.
    pub id: String,
    pub theme: Arc<Theme>,
}

/// Resolves `theme_ref` — a bare family id or a `family.variant` id —
/// against the registry and merges it into the runtime [`Theme`].
///
/// Merge order (later wins): built-in default theme → base chain (root
/// first, cycle-checked) → the target variant's own patch → settings
/// overrides (colors, dimensions, typography). Extension tokens start from
/// the registry's token schema defaults and are then overridden by the
/// chain and the settings overrides. `appearance` must already be resolved
/// (the caller maps `Auto` onto the system appearance).
pub fn resolve_theme(
    registry: &ThemeRegistry,
    theme_ref: &str,
    appearance: Appearance,
    settings: &ThemeSettingsContent,
) -> anyhow::Result<ResolvedTheme> {
    // Ids are lowercased slugs; refs are matched case-insensitively.
    let (family_id, variant) = locate(registry, theme_ref, appearance)?;

    // Walk the base chain (root first) with cycle detection.
    let mut chain: Vec<ThemeStyleContent> = Vec::new();
    let mut visited = HashSet::new();
    let mut base_ref: Option<String> = variant.style.base.clone();
    while let Some(current_ref) = base_ref {
        if !visited.insert(current_ref.clone()) {
            bail!("theme base cycle detected at '{current_ref}'");
        }
        let (_, base_variant) = locate(registry, &current_ref, appearance)?;
        chain.push(base_variant.style.clone());
        base_ref = base_variant.style.base.clone();
    }

    let mut theme = Theme {
        name: variant.name.clone(),
        appearance: variant.appearance,
        ..Theme::default_theme()
    };
    for style in &chain {
        style.apply_to(&mut theme);
    }
    variant.style.apply_to(&mut theme);

    // Extension tokens: schema defaults, then chain and variant patches.
    // Patch keys must address registered tokens — unknown keys are hard
    // errors, like every other unknown key in the theme schema.
    let mut extension = registry.token_defaults();
    for style in chain.iter().chain(std::iter::once(&variant.style)) {
        if let Some(patch) = &style.extension {
            for key in patch.keys() {
                if !registry.token_schema().contains_key(key) {
                    bail!("unknown extension token '{key}' in theme patch");
                }
            }
            for (key, value) in patch {
                extension.insert(key.clone(), *value);
            }
        }
    }

    // Settings color overrides sit above every theme file.
    for (key, value) in &settings.overrides {
        apply_override(
            &mut theme,
            &mut extension,
            registry.token_schema(),
            key,
            *value,
        )?;
    }

    // Dimension overrides: one field per patch, validated by the patch
    // schema's deny-unknown-fields.
    for (field, value) in &settings.dimension_overrides {
        let mut object = Map::new();
        object.insert(field.clone(), serde_json::Value::from(*value));
        let patch: ThemeDimensionsPatch = serde_json::from_value(serde_json::Value::Object(object))
            .with_context(|| format!("unknown dimension override '{field}'"))?;
        patch.apply_to(&mut theme.dimensions);
    }

    // Typography overrides: sizes are numbers, weights are lowercase names.
    for (field, value) in &settings.typography_overrides {
        let mut object = Map::new();
        object.insert(field.clone(), value.clone());
        let patch: ThemeTypographyPatch = serde_json::from_value(serde_json::Value::Object(object))
            .with_context(|| format!("unknown typography override '{field}'"))?;
        patch.apply_to(&mut theme.typography);
    }
    theme.extension = extension;

    Ok(ResolvedTheme {
        id: format!("{family_id}.{}", variant.id()),
        theme: Arc::new(theme),
    })
}

/// Finds the family and variant addressed by `theme_ref`; a bare family id
/// selects the appearance-matching variant (falling back to the first one).
/// Refs are matched case-insensitively against the lowercased slug ids.
fn locate<'a>(
    registry: &'a ThemeRegistry,
    theme_ref: &str,
    appearance: Appearance,
) -> anyhow::Result<(String, &'a ThemeContent)> {
    let theme_ref = theme_ref.trim().to_ascii_lowercase();
    let (family_ref, variant_ref) = match theme_ref.split_once('.') {
        Some((family, variant)) => (family, Some(variant)),
        None => (theme_ref.as_str(), None),
    };
    let family = registry
        .family(family_ref)
        .with_context(|| format!("unknown theme family '{family_ref}'"))?;
    let variant = match variant_ref {
        Some(variant_ref) => family
            .themes
            .iter()
            .find(|theme| theme.id() == variant_ref)
            .with_context(|| format!("unknown theme variant '{theme_ref}'"))?,
        None => family
            .themes
            .iter()
            .find(|theme| theme.appearance == appearance)
            .or_else(|| family.themes.first())
            .with_context(|| format!("theme family '{family_ref}' has no variants"))?,
    };
    Ok((family.id(), variant))
}

/// Applies one settings override: `colors.<field>` keys target color tokens,
/// any other key must address a registered extension token. Unknown keys are
/// hard errors.
fn apply_override(
    theme: &mut Theme,
    extension: &mut BTreeMap<String, Hsla>,
    token_schema: &BTreeMap<String, TokenDeclaration>,
    key: &str,
    value: Hsla,
) -> anyhow::Result<()> {
    if let Some(field) = key.strip_prefix("colors.") {
        if !theme.colors.set_color_token(field, value) {
            bail!("unknown color override key '{key}'");
        }
        return Ok(());
    }
    if token_schema.contains_key(key) {
        extension.insert(key.to_string(), value);
        return Ok(());
    }
    bail!("unknown theme override key '{key}'")
}
