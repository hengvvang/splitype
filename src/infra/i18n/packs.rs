//! Language packs: built-in catalog, custom pack import, and locale
//! detection.

use anyhow::bail;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::infra::config::jsonc::{
    merge_non_empty_json_values, object_without_empty_values, prune_empty_json_values,
};

use super::strings::{I18N_STRING_KEYS, I18nStrings};

pub const BUILTIN_LANGUAGE_EN_US_ID: &str = "en-US";
pub const BUILTIN_LANGUAGE_ZH_CN_ID: &str = "zh-CN";
const BUILTIN_LANGUAGE_ZH_CN_NAME: &str = "简体中文";
const BUILTIN_LANGUAGE_EN_US_NAME: &str = "English";

/// Strongly typed, normalized BCP-47 language identifier (e.g. "en-US", "zh-CN").
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LanguageId(String);

impl LanguageId {
    pub const EN_US: &'static str = BUILTIN_LANGUAGE_EN_US_ID;
    pub const ZH_CN: &'static str = BUILTIN_LANGUAGE_ZH_CN_ID;

    pub fn new(raw: impl AsRef<str>) -> Self {
        let s = raw.as_ref().trim();
        let normalized = if s.contains('_') {
            s.replace('_', "-")
        } else {
            s.to_string()
        };
        Self(normalized)
    }

    pub fn en_us() -> Self {
        Self(Self::EN_US.to_string())
    }

    pub fn zh_cn() -> Self {
        Self(Self::ZH_CN.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for LanguageId {
    fn default() -> Self {
        Self::en_us()
    }
}

impl std::fmt::Display for LanguageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for LanguageId {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s))
    }
}

impl std::ops::Deref for LanguageId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for LanguageId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<&str> for LanguageId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for LanguageId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

pub struct LanguageCatalogEntry {
    pub id: String,
    pub name: String,
}

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

/// A JSON language pack with metadata and canonical strings.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl I18nLanguagePack {
    /// Parses a language pack from JSON text.
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        let mut value: Value = serde_json::from_str(json)?;
        prune_empty_json_values(&mut value);
        let Value::Object(object) = value else {
            bail!("language config must be a JSON object");
        };
        let object = object_without_empty_values(object);
        let id = required_string(&object, "id")?;
        let name = object
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| id.clone());
        let base_strings = I18nStrings::for_language_id(&id).unwrap_or_else(I18nStrings::en_us);
        let mut merged_strings = serde_json::to_value(base_strings)?;
        if let Some(strings) = object.get("strings").and_then(Value::as_object) {
            let mut normalized_strings = Map::new();
            for key in I18N_STRING_KEYS {
                if let Some(val) = strings.get(*key) {
                    normalized_strings.insert((*key).into(), val.clone());
                }
            }
            merge_non_empty_json_values(
                &mut merged_strings,
                &Value::Object(normalized_strings),
            );
        }
        let mut pack_object = Map::new();
        pack_object.insert("id".into(), Value::String(id));
        pack_object.insert("name".into(), Value::String(name));
        for key in ["author", "description", "version", "homepage", "license"] {
            if let Some(val) = object.get(key) {
                pack_object.insert(key.into(), val.clone());
            }
        }
        pack_object.insert("strings".into(), merged_strings);
        let pack: Self = serde_json::from_value(Value::Object(pack_object))?;
        Ok(pack)
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
    let Value::Object(object) = &value else {
        bail!("language config must be a JSON object");
    };
    let object = object_without_empty_values(object.clone());
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
        if let Some(val) = object.get(key) {
            normalized_object.insert(key.into(), val.clone());
        }
    }
    if let Some(strings) = object.get("strings").and_then(Value::as_object) {
        let mut normalized_strings = Map::new();
        for key in I18N_STRING_KEYS {
            if let Some(val) = strings.get(*key) {
                normalized_strings.insert((*key).into(), val.clone());
            }
        }
        if !normalized_strings.is_empty() {
            normalized_object.insert("strings".into(), Value::Object(normalized_strings));
        }
    }
    let pack = I18nLanguagePack::from_json(&serde_json::to_string(&value)?)?;
    let normalized = Value::Object(normalized_object);
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
