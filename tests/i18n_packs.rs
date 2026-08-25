//! Cross-crate contract tests for localization.
//!
//! Verifies that every built-in language pack fills every string key and
//! that partial JSON packs fall back to the English defaults.

use serde_json::{Map, Value};
use splitype::infra::i18n::strings::I18nStrings;

fn key_set(strings: &I18nStrings) -> Map<String, Value> {
    serde_json::to_value(strings)
        .expect("i18n strings serialize")
        .as_object()
        .expect("object")
        .clone()
}

/// English is the fallback pack and must be complete and non-empty.
#[test]
fn english_pack_has_no_empty_values() {
    let strings = I18nStrings::en_us();
    for (key, value) in key_set(&strings) {
        let text = value
            .as_str()
            .unwrap_or_else(|| panic!("{key} is not a string"));
        assert!(!text.is_empty(), "en_us.{key} is empty");
    }
}

/// Every pack exposes exactly the same keys as English.
#[test]
fn all_packs_share_the_english_key_set() {
    let english = key_set(&I18nStrings::en_us());
    let chinese = key_set(&I18nStrings::zh_cn());
    assert_eq!(
        chinese.keys().collect::<Vec<_>>(),
        english.keys().collect::<Vec<_>>(),
        "zh_cn key set must match en_us"
    );
}

/// Chinese values are translated (at least the headline strings).
#[test]
fn chinese_pack_is_translated() {
    let strings = I18nStrings::zh_cn();
    assert_eq!(strings.unsaved_changes_title, "不保存并关闭？");
    assert_eq!(strings.dirty_title_marker, "\u{00B7}");
    assert!(strings.unsaved_changes_message.contains("保存"));
}

/// Canonical I18nStrings round-trips with serde_json.
#[test]
fn canonical_json_roundtrips() {
    let english = I18nStrings::en_us();
    let json = serde_json::to_string(&english).expect("serialize");
    let restored: I18nStrings = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(key_set(&english), key_set(&restored));
}

/// Partial language pack JSON via I18nLanguagePack fills missing keys from base defaults.
#[test]
fn partial_language_pack_falls_back_to_base_defaults() {
    let json = r#"{
        "id": "custom-test",
        "name": "Custom",
        "strings": {
            "dirty_title_marker": "*"
        }
    }"#;
    let pack =
        splitype::infra::i18n::packs::I18nLanguagePack::from_json(json).expect("parse partial pack");
    assert_eq!(pack.strings.dirty_title_marker, "*");
    assert_eq!(
        pack.strings.unsaved_changes_title,
        I18nStrings::en_us().unsaved_changes_title
    );
}

/// Incomplete raw I18nStrings is rejected by canonical deserializer.
#[test]
fn incomplete_raw_strings_rejected_by_canonical_deserializer() {
    let result: Result<I18nStrings, _> = serde_json::from_str("{}");
    assert!(result.is_err());
}
