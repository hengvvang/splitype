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

/// Partial JSON deserialization fills missing keys from English defaults.
#[test]
fn partial_json_pack_falls_back_to_english() {
    let json = r#"{"dirty_title_marker": "*"}"#;
    let strings: I18nStrings = serde_json::from_str(json).expect("parse partial pack");
    assert_eq!(strings.dirty_title_marker, "*");
    assert_eq!(
        strings.unsaved_changes_title,
        I18nStrings::en_us().unsaved_changes_title
    );
}

/// An empty JSON pack is equivalent to English.
#[test]
fn empty_json_pack_is_english() {
    let strings: I18nStrings = serde_json::from_str("{}").expect("parse empty pack");
    let english = I18nStrings::en_us();
    assert_eq!(key_set(&strings), key_set(&english));
}
