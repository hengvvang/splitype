//! Language packs: built-in catalog, custom pack schema, and locale
//! detection.

use std::collections::BTreeMap;

use anyhow::{Context as _, bail};
use serde::{Deserialize, Serialize};

use crate::jsonc::parse_jsonc_value;

use super::strings::{I18N_STRING_KEYS, I18nStrings};

pub const BUILTIN_LANGUAGE_EN_US_ID: &str = "en-US";
pub const BUILTIN_LANGUAGE_ZH_CN_ID: &str = "zh-CN";
const BUILTIN_LANGUAGE_ZH_CN_NAME: &str = "简体中文";
const BUILTIN_LANGUAGE_EN_US_NAME: &str = "English";

/// One selectable language exposed in menus and settings.
#[derive(Debug, Clone, PartialEq)]
pub struct LanguageCatalogEntry {
    pub id: String,
    pub name: String,
    pub author: String,
}

pub fn builtin_language_catalog() -> Vec<LanguageCatalogEntry> {
    vec![
        LanguageCatalogEntry {
            id: BUILTIN_LANGUAGE_ZH_CN_ID.into(),
            name: BUILTIN_LANGUAGE_ZH_CN_NAME.into(),
            author: String::new(),
        },
        LanguageCatalogEntry {
            id: BUILTIN_LANGUAGE_EN_US_ID.into(),
            name: BUILTIN_LANGUAGE_EN_US_NAME.into(),
            author: String::new(),
        },
    ]
}

/// The one canonical language pack file schema: metadata plus a partial
/// string patch. Every omitted string key inherits from the base language
/// (`base` when set, otherwise English); unknown keys and empty values are
/// hard errors.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguagePackContent {
    /// BCP-47 language id (e.g. `ja-JP`).
    pub id: String,
    /// Display name shown in the language menu.
    pub name: String,
    /// Optional author, shown in the language menu.
    #[serde(default)]
    pub author: String,
    /// Optional built-in base language (e.g. `zh-CN`); defaults to English.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    /// Partial UI string overrides keyed by canonical string id.
    pub strings: BTreeMap<String, String>,
}

impl LanguagePackContent {
    /// Parses and validates a language pack JSONC document.
    pub fn from_jsonc(text: &str) -> anyhow::Result<Self> {
        let value = parse_jsonc_value(text).context("invalid language pack document")?;
        let content: Self =
            serde_json::from_value(value).context("invalid language pack schema")?;
        content.validate()?;
        Ok(content)
    }

    /// Validates the pack invariants shared by imports and directory loads.
    pub fn validate(&self) -> anyhow::Result<()> {
        let id = self.id.trim();
        if id.is_empty() {
            bail!("language pack id must not be empty");
        }
        if is_builtin_language_id(id) {
            bail!("language pack id '{id}' would override a built-in language");
        }
        if !is_valid_custom_language_id(id) {
            bail!("language pack id '{id}' contains unsupported characters");
        }
        if let Some(base) = &self.base {
            let base = base.trim();
            if !is_builtin_language_id(base) {
                bail!("language pack base '{base}' is not a built-in language");
            }
        }
        if self.name.trim().is_empty() {
            bail!("language pack name must not be empty");
        }
        for key in self.strings.keys() {
            if !I18N_STRING_KEYS.contains(&key.as_str()) {
                bail!("unknown language string key '{key}'");
            }
            if self.strings[key].trim().is_empty() {
                bail!("language string '{key}' must not be empty");
            }
        }
        Ok(())
    }

    /// Resolves the pack onto its base language, producing the runtime
    /// string set.
    pub fn resolve(&self) -> I18nLanguagePack {
        let base_id = self.base.as_deref().unwrap_or(BUILTIN_LANGUAGE_EN_US_ID);
        let base = I18nStrings::for_language_id(base_id).unwrap_or_else(I18nStrings::en_us);
        let mut merged = serde_json::to_value(base).expect("built-in strings must serialize");
        if let serde_json::Value::Object(object) = &mut merged {
            for (key, value) in &self.strings {
                object.insert(key.clone(), serde_json::Value::String(value.clone()));
            }
        }
        let strings: I18nStrings =
            serde_json::from_value(merged).expect("patched strings must deserialize");
        I18nLanguagePack {
            id: self.id.clone(),
            name: self.name.clone(),
            author: self.author.clone(),
            strings,
        }
    }
}

/// A resolved language pack: metadata plus the complete runtime strings.
#[derive(Debug, Clone)]
pub struct I18nLanguagePack {
    pub id: String,
    pub name: String,
    pub author: String,
    pub strings: I18nStrings,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_resolves_partial_packs() {
        let pack = LanguagePackContent::from_jsonc(
            r#"{
                "id": "ja-JP",
                "name": "日本語",
                "author": "Tanaka",
                "strings": { "menu_file": "ファイル" }
            }"#,
        )
        .expect("pack should parse");
        let resolved = pack.resolve();
        assert_eq!(resolved.id, "ja-JP");
        assert_eq!(resolved.author, "Tanaka");
        assert_eq!(resolved.strings.menu_file, "ファイル");
        // Omitted keys inherit from English.
        assert_eq!(
            resolved.strings.menu_export,
            I18nStrings::en_us().menu_export
        );

        // Custom packs based on a built-in base language inherit it.
        let zh = LanguagePackContent::from_jsonc(
            r#"{
                "id": "zh-HK",
                "name": "香港中文",
                "base": "zh-CN",
                "strings": { "menu_file": "文件菜单" }
            }"#,
        )
        .expect("pack should parse");
        let resolved = zh.resolve();
        assert_eq!(resolved.strings.menu_file, "文件菜单");
        assert_eq!(
            resolved.strings.menu_export,
            I18nStrings::zh_cn().menu_export
        );
    }

    #[test]
    fn rejects_unknown_keys_and_builtin_ids() {
        assert!(
            LanguagePackContent::from_jsonc(
                r#"{ "id": "ja-JP", "name": "日本語", "strings": { "nope": "x" } }"#
            )
            .is_err()
        );
        assert!(
            LanguagePackContent::from_jsonc(
                r#"{ "id": "ja-JP", "name": "日本語", "strings": { "menu_file": "" } }"#
            )
            .is_err()
        );
        assert!(
            LanguagePackContent::from_jsonc(
                r#"{ "id": "en-US", "name": "Override", "strings": {} }"#
            )
            .is_err()
        );
        assert!(LanguagePackContent::from_jsonc(r#"{ "id": "ja-JP", "strings": {} }"#).is_err());
        assert!(
            LanguagePackContent::from_jsonc(
                r#"{ "id": "ja-JP", "name": "日本語", "base": "fr-FR", "strings": {} }"#
            )
            .is_err()
        );
    }

    #[test]
    fn built_in_chinese_strings_are_utf8() {
        let strings = I18nStrings::zh_cn();
        assert_eq!(strings.menu_file, "文件");
        assert_eq!(strings.menu_export, "导出");
        assert_eq!(strings.menu_language, "语言");
    }
}
