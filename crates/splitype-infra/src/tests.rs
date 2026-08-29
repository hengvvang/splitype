#[cfg(test)]
mod tests {
    use crate::config::jsonc::parse_jsonc_value;
    use crate::config::settings::AppSettings;
    use crate::i18n::packs::language_id_for_locale_settings;
    use crate::theme::Theme;

    #[test]
    fn test_jsonc_comment_parsing() {
        let jsonc_str = r#"
        {
            // This is a line comment
            "key": "value", /* block comment */
            "number": 42,
        }
        "#;
        let parsed = parse_jsonc_value(jsonc_str).expect("should parse JSONC");
        assert_eq!(parsed["key"], "value");
        assert_eq!(parsed["number"], 42);
    }

    #[test]
    fn test_app_settings_default_and_roundtrip() {
        let default_settings = AppSettings::default();
        let json = serde_json::to_string(&default_settings).expect("serialization should succeed");
        let deserialized: AppSettings =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(
            default_settings.typography.font_size,
            deserialized.typography.font_size
        );
    }

    #[test]
    fn test_language_id_locale_fallback() {
        assert_eq!(
            language_id_for_locale_settings(Some("zh-CN")),
            "zh-CN".to_string()
        );
        assert_eq!(
            language_id_for_locale_settings(Some("en-US")),
            "en-US".to_string()
        );
    }

    #[test]
    fn test_default_theme_color_tokens() {
        let theme = Theme::default_theme();
        assert_eq!(theme.name, "Dark");
        assert!(theme.colors.editor_background.l < 0.5); // Dark theme background is dark
    }
}
