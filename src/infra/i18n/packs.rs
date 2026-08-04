//! Language packs: built-in catalog, custom pack import, and locale
//! detection.

use anyhow::{Context as _, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::infra::config::jsonc::{
    object_without_empty_values, prune_empty_json_values,
};

use super::strings::{I18N_STRING_KEYS, I18nStrings, I18nStringsDe};

pub const BUILTIN_LANGUAGE_EN_US_ID: &str = "en-US";

pub struct LanguageCatalogEntry {
    pub id: String,
    pub name: String,
}

const BUILTIN_LANGUAGE_ZH_CN_ID: &str = "zh-CN";
const BUILTIN_LANGUAGE_ZH_CN_NAME: &str = "简体中文";
const BUILTIN_LANGUAGE_EN_US_NAME: &str = "English";

pub fn builtin_language_catalog() -> Vec<LanguageCatalogEntry> {
    vec![
        LanguageCatalogEntry {
            id: BUILTIN_LANGUAGE_ZH_CN_ID.into(),
            name: BUILTIN_LANGUAGE_ZH_CN_NAME.into(),
        },
        LanguageCatalogEntry {
            id: BUILTIN_LANGUAGE_EN_US_ID.into(),
            name: BUILTIN_LANGUAGE_EN_US_NAME.into(),
        },
    ]
}

/// A JSON language pack with metadata and fallback-completed strings.
#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct I18nLanguagePack {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    pub strings: I18nStrings,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct I18nLanguagePackDe {
    id: String,
    name: Option<String>,
    author: Option<String>,
    description: Option<String>,
    version: Option<String>,
    homepage: Option<String>,
    license: Option<String>,
    #[serde(default)]
    strings: I18nStringsDe,
}

#[allow(dead_code)]
impl I18nLanguagePack {
    /// Parses a language pack from JSON text.
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        let mut value: Value = serde_json::from_str(json)?;
        prune_empty_json_values(&mut value);
        Self::from_value(value)
    }

    fn from_value(value: Value) -> anyhow::Result<Self> {
        let raw: I18nLanguagePackDe = serde_json::from_value(value)?;
        Ok(Self::from_partial(raw))
    }

    fn from_partial(raw: I18nLanguagePackDe) -> Self {
        let fallback = I18nStrings::for_language_id(&raw.id).unwrap_or_else(I18nStrings::en_us);
        let name = raw
            .name
            .unwrap_or_else(|| language_name_for_id(&raw.id).unwrap_or(&raw.id).to_string());
        Self {
            id: raw.id,
            name,
            author: raw.author,
            description: raw.description,
            version: raw.version,
            homepage: raw.homepage,
            license: raw.license,
            strings: raw.strings.into_strings(fallback),
        }
    }
}

#[allow(dead_code)]
fn language_name_for_id(language_id: &str) -> Option<&'static str> {
    match language_id {
        BUILTIN_LANGUAGE_ZH_CN_ID => Some(BUILTIN_LANGUAGE_ZH_CN_NAME),
        BUILTIN_LANGUAGE_EN_US_ID => Some(BUILTIN_LANGUAGE_EN_US_NAME),
        _ => None,
    }
}

fn is_builtin_language_id(language_id: &str) -> bool {
    matches!(
        language_id,
        BUILTIN_LANGUAGE_ZH_CN_ID | BUILTIN_LANGUAGE_EN_US_ID
    )
}

fn is_valid_custom_language_id(language_id: &str) -> bool {
    !language_id.trim().is_empty()
        && language_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        && language_id.chars().any(|ch| ch.is_ascii_alphabetic())
}

/// Selects a built-in language id from preferred system locales.
pub fn language_id_for_locale_settings<I, S>(locales: I) -> &'static str
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    locales
        .into_iter()
        .find_map(|locale| language_id_for_locale(locale.as_ref()))
        .unwrap_or(BUILTIN_LANGUAGE_EN_US_ID)
}

fn language_id_for_locale(locale: &str) -> Option<&'static str> {
    let locale = locale.trim();
    if locale.is_empty() {
        return None;
    }

    let no_encoding = locale
        .split_once('.')
        .map_or(locale, |(locale, _encoding)| locale);
    let no_modifier = no_encoding
        .split_once('@')
        .map_or(no_encoding, |(locale, _modifier)| locale);
    let locale = no_modifier.replace('_', "-");
    let language = locale.split('-').next()?.to_ascii_lowercase();
    if !language.chars().all(|ch| ch.is_ascii_alphabetic()) {
        return None;
    }

    match language.as_str() {
        "zh" => Some(BUILTIN_LANGUAGE_ZH_CN_ID),
        "en" => Some(BUILTIN_LANGUAGE_EN_US_ID),
        _ => None,
    }
}

pub fn custom_language_pack_from_value(
    mut value: Value,
) -> anyhow::Result<(I18nLanguagePack, Value)> {
    prune_empty_json_values(&mut value);
    let Value::Object(object) = value else {
        bail!("language config must be a JSON object");
    };
    let object = object_without_empty_values(object);
    let id = required_string(&object, "id")?;
    if is_builtin_language_id(&id) {
        bail!("custom language id '{id}' would override a built-in language");
    }
    if !is_valid_custom_language_id(&id) {
        bail!("custom language id '{id}' contains unsupported characters");
    }
    let name = required_string(&object, "name")?;
    let mut normalized_object = Map::new();
    normalized_object.insert("id".into(), Value::String(id.clone()));
    normalized_object.insert("name".into(), Value::String(name));
    for key in ["author", "description", "version", "homepage", "license"] {
        if let Some(value) = object.get(key) {
            normalized_object.insert(key.into(), value.clone());
        }
    }
    if let Some(strings) = object.get("strings").and_then(Value::as_object) {
        let mut normalized_strings = Map::new();
        for key in I18N_STRING_KEYS {
            if let Some(value) = strings.get(*key) {
                normalized_strings.insert((*key).into(), value.clone());
            }
        }
        if !normalized_strings.is_empty() {
            normalized_object.insert("strings".into(), Value::Object(normalized_strings));
        }
    }
    let normalized = Value::Object(normalized_object);
    let pack = I18nLanguagePack::from_value(normalized.clone())
        .with_context(|| format!("failed to parse language config '{id}'"))?;
    Ok((pack, normalized))
}

fn required_string(object: &Map<String, Value>, key: &str) -> anyhow::Result<String> {
    let Some(value) = object.get(key) else {
        bail!("missing required field '{key}'");
    };
    let Some(text) = value
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
    else {
        bail!("field '{key}' must be a non-empty string");
    };
    Ok(text.to_string())
}
