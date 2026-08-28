//! Runtime language selection: the global manager and pack installation.

use std::path::Path;
use std::sync::Arc;

use gpui::{App, Global};

use super::packs::{
    BUILTIN_LANGUAGE_EN_US_ID, I18nLanguagePack, LanguageCatalogEntry, builtin_language_catalog,
    custom_language_pack_from_value,
};
use super::strings::I18nStrings;
use crate::infra::config::dirs::SplitypeConfigDirs;
use crate::infra::config::jsonc::{read_json_or_jsonc, sanitize_config_file_stem};

pub struct I18nManager {
    current_language_id: String,
    strings: Arc<I18nStrings>,
    custom_languages: Vec<I18nLanguagePack>,
    language_catalog: Vec<LanguageCatalogEntry>,
}

impl Global for I18nManager {}

impl Default for I18nManager {
    fn default() -> Self {
        Self::new_with_language_id(BUILTIN_LANGUAGE_EN_US_ID)
    }
}

impl I18nManager {
    /// Test-only: installs the configured UI language into GPUI's global state.
    #[cfg(test)]
    pub fn init(cx: &mut App) {
        let language_id = crate::infra::config::settings::read_app_settings()
            .map(|settings| settings.interface.language_id)
            .unwrap_or_else(|_| BUILTIN_LANGUAGE_EN_US_ID.into());
        Self::init_with_language_id(cx, &language_id);
    }

    /// Installs a specific UI language into GPUI's global state.
    pub fn init_with_language_id(cx: &mut App, language_id: &str) {
        let mut manager = Self::new_with_language_id(BUILTIN_LANGUAGE_EN_US_ID);
        if let Ok(dirs) = SplitypeConfigDirs::from_system()
            && let Err(err) = manager.load_custom_languages_from_dirs(&dirs)
        {
            tracing::warn!(error = %err, "failed to load custom languages");
        }
        let _ = manager.set_language_by_id(language_id);
        cx.set_global(manager);
    }

    /// Creates a manager with a known language id, falling back to English.
    pub fn new_with_language_id(language_id: &str) -> Self {
        let current_language_id = if I18nStrings::for_language_id(language_id).is_some() {
            language_id
        } else {
            BUILTIN_LANGUAGE_EN_US_ID
        };
        Self {
            current_language_id: current_language_id.into(),
            strings: Arc::new(
                I18nStrings::for_language_id(current_language_id)
                    .unwrap_or_else(I18nStrings::en_us),
            ),
            custom_languages: Vec::new(),
            language_catalog: builtin_language_catalog(),
        }
    }

    /// Returns the identifier of the currently active UI language.
    pub fn current_language_id(&self) -> &str {
        &self.current_language_id
    }

    /// Returns the strings for the currently active UI language.
    pub fn strings(&self) -> &I18nStrings {
        &self.strings
    }

    /// Returns an `Arc` clone of the currently active strings — O(1), no
    /// per-field copy. Use this in hot render paths instead of cloning the
    /// whole `I18nStrings` struct (137 `String` fields).
    pub fn strings_arc(&self) -> Arc<I18nStrings> {
        self.strings.clone()
    }

    /// Returns all built-in and imported UI languages exposed in the menu.
    pub fn available_languages(&self) -> &[LanguageCatalogEntry] {
        &self.language_catalog
    }

    /// Activates a UI language by identifier.
    pub fn set_language_by_id(&mut self, language_id: &str) -> bool {
        let strings = if let Some(strings) = I18nStrings::for_language_id(language_id) {
            strings
        } else if let Some(pack) = self
            .custom_languages
            .iter()
            .find(|pack| pack.id == language_id)
        {
            pack.strings.clone()
        } else {
            return false;
        };
        let changed = self.current_language_id != language_id;
        self.current_language_id = language_id.into();
        self.strings = Arc::new(strings);
        changed
    }

    /// Imports a user language pack, persists a normalized copy, and activates it.
    pub fn import_language_config(&mut self, path: impl AsRef<Path>) -> anyhow::Result<String> {
        let dirs = SplitypeConfigDirs::from_system()?;
        self.import_language_config_with_dirs(path, &dirs)
    }

    fn import_language_config_with_dirs(
        &mut self,
        path: impl AsRef<Path>,
        dirs: &SplitypeConfigDirs,
    ) -> anyhow::Result<String> {
        let raw = read_json_or_jsonc(path.as_ref())?;
        let (pack, normalized) = custom_language_pack_from_value(raw)?;
        let file_name = format!("{}.json", sanitize_config_file_stem(&pack.id));
        let languages_dir = dirs.languages_dir();
        std::fs::create_dir_all(&languages_dir)?;
        std::fs::write(
            languages_dir.join(file_name),
            serde_json::to_string_pretty(&normalized)?,
        )?;
        let imported_id = pack.id.clone();
        self.upsert_custom_language(pack);
        self.set_language_by_id(&imported_id);
        Ok(imported_id)
    }

    fn load_custom_languages_from_dirs(&mut self, dirs: &SplitypeConfigDirs) -> anyhow::Result<()> {
        let languages_dir = dirs.languages_dir();
        if !languages_dir.exists() {
            return Ok(());
        }

        let mut loaded = Vec::new();
        for entry in std::fs::read_dir(&languages_dir)? {
            let path = entry?.path();
            if path.is_file() {
                match read_json_or_jsonc(&path).and_then(|value| {
                    custom_language_pack_from_value(value).map(|(pack, _normalized)| pack)
                }) {
                    Ok(pack) => loaded.push(pack),
                    Err(err) => tracing::warn!(
                        path = %path.display(),
                        error = %err,
                        "skipping custom language config"
                    ),
                }
            }
        }
        loaded.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
        for pack in loaded {
            self.upsert_custom_language(pack);
        }
        Ok(())
    }

    fn upsert_custom_language(&mut self, pack: I18nLanguagePack) {
        if let Some(existing) = self
            .custom_languages
            .iter_mut()
            .find(|existing| existing.id == pack.id)
        {
            *existing = pack;
        } else {
            self.custom_languages.push(pack);
        }
        self.rebuild_language_catalog();
    }

    fn rebuild_language_catalog(&mut self) {
        let mut catalog = builtin_language_catalog();
        catalog.extend(
            self.custom_languages
                .iter()
                .map(|pack| LanguageCatalogEntry {
                    id: pack.id.clone(),
                    name: pack.name.clone(),
                }),
        );
        self.language_catalog = catalog;
    }
}

#[cfg(test)]
mod tests {
    use super::{I18nLanguagePack, I18nManager, I18nStrings};
    use crate::infra::config::dirs::SplitypeConfigDirs;
    use crate::infra::i18n::packs::language_id_for_locale_settings;
    use crate::infra::theme::ThemeManager;

    #[test]
    fn built_in_chinese_strings_are_utf8() {
        let strings = I18nStrings::zh_cn();
        assert_eq!(strings.menu_file, "文件");
        assert_eq!(strings.menu_export, "导出");
        assert_eq!(strings.menu_language, "语言");
        assert_eq!(strings.save_failed_title, "保存失败");
        assert_eq!(strings.export_failed_title, "导出失败");
        assert_eq!(strings.pane_mode_switch_to_source, "切换到源码");
        assert_eq!(strings.context_menu_insert, "插入");
        assert_eq!(strings.table_insert_title, "插入表格");
        assert_eq!(strings.image_loading_without_alt, "正在加载图片...");
        assert_eq!(
            strings.help_check_updates_message,
            "正在检查 Splitype 的最新版本..."
        );
        assert_eq!(strings.update_open_release, "前往下载");
        assert_eq!(strings.help_about_github_label, "GitHub");
        assert_eq!(
            strings.help_about_star_message,
            "如果本项目对您有帮助，那不妨给本项目一颗 Star⭐，十分感谢！"
        );
    }

    #[test]
    fn manager_switches_builtin_languages() {
        let mut manager = I18nManager::default();
        assert_eq!(manager.current_language_id(), "en-US");
        assert_eq!(manager.strings().menu_file, "File");
        assert_eq!(manager.strings().menu_export, "Export");

        assert!(manager.set_language_by_id("zh-CN"));
        assert_eq!(manager.current_language_id(), "zh-CN");
        assert_eq!(manager.strings().menu_file, "文件");
        assert_eq!(manager.strings().menu_export, "导出");
        assert!(!manager.set_language_by_id("zh-CN"));
        assert!(!manager.set_language_by_id("missing"));
    }

    #[test]
    fn language_catalog_contains_chinese_and_english() {
        let manager = I18nManager::default();
        let ids = manager
            .available_languages()
            .iter()
            .map(|entry| (entry.id.as_str(), entry.name.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(ids, vec![("zh-CN", "简体中文"), ("en-US", "English")]);
    }

    #[test]
    fn manager_can_be_constructed_with_known_language() {
        let manager = I18nManager::new_with_language_id("zh-CN");
        assert_eq!(manager.current_language_id(), "zh-CN");
        assert_eq!(manager.strings().menu_file, "文件");

        let fallback = I18nManager::new_with_language_id("missing");
        assert_eq!(fallback.current_language_id(), "en-US");
        assert_eq!(fallback.strings().menu_file, "File");
    }

    #[test]
    fn theme_switch_does_not_modify_selected_language() {
        let mut theme_manager = ThemeManager::default();
        let mut i18n_manager = I18nManager::new_with_language_id("zh-CN");

        assert!(theme_manager.set_theme_by_id("splitype"));
        assert!(!i18n_manager.set_language_by_id("missing"));

        assert_eq!(theme_manager.current_theme_id(), "splitype");
        assert_eq!(i18n_manager.current_language_id(), "zh-CN");
        assert_eq!(i18n_manager.strings().menu_file, "文件");
    }

    #[test]
    fn locale_settings_map_to_builtin_languages() {
        assert_eq!(language_id_for_locale_settings(["zh-CN"]), "zh-CN");
        assert_eq!(language_id_for_locale_settings(["zh-HK"]), "zh-CN");
        assert_eq!(language_id_for_locale_settings(["zh-Hant-TW"]), "zh-CN");
        assert_eq!(language_id_for_locale_settings(["zh_SG.UTF-8"]), "zh-CN");
        assert_eq!(language_id_for_locale_settings(["en-US"]), "en-US");
        assert_eq!(language_id_for_locale_settings(["en_GB.UTF-8"]), "en-US");
        assert_eq!(language_id_for_locale_settings(["fr-FR", "zh-CN"]), "zh-CN");
        assert_eq!(language_id_for_locale_settings(Vec::<&str>::new()), "en-US");
        assert_eq!(language_id_for_locale_settings(["fr-FR"]), "en-US");
        assert_eq!(language_id_for_locale_settings(["!!!"]), "en-US");
    }

    #[test]
    fn imports_jsonc_language_pack_and_persists_normalized_json() {
        let root = std::env::temp_dir().join(format!("splitype-i18n-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("temp root should be created");
        let source = root.join("language.jsonc");
        std::fs::write(
            &source,
            r#"{
                // Required metadata.
                "id": "ja-JP",
                "name": "日本語",
                "author": "",
                "strings": {
                    "menu_file": "ファイル",
                    "menu_export": ""
                }
            }"#,
        )
        .expect("language config should be written");

        let dirs = SplitypeConfigDirs::from_root(&root);
        let mut manager = I18nManager::default();
        let imported_id = manager
            .import_language_config_with_dirs(&source, &dirs)
            .expect("language config should import");

        assert_eq!(imported_id, "ja-JP");
        assert_eq!(manager.current_language_id(), "ja-JP");
        assert_eq!(manager.strings().menu_file, "ファイル");
        assert_eq!(manager.strings().menu_export, "Export");
        assert!(
            manager
                .available_languages()
                .iter()
                .any(|entry| entry.id == "ja-JP" && entry.name == "日本語")
        );

        let normalized = std::fs::read_to_string(dirs.languages_dir().join("ja-JP.json"))
            .expect("normalized language config should exist");
        assert!(normalized.contains("\"menu_file\": \"ファイル\""));
        assert!(!normalized.contains("menu_export"));
        assert!(!normalized.contains("author"));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn custom_language_cannot_override_builtin_language_id() {
        let root = std::env::temp_dir().join(format!("splitype-i18n-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("temp root should be created");
        let source = root.join("language.json");
        std::fs::write(
            &source,
            r#"{
                "id": "en-US",
                "name": "Override",
                "strings": { "menu_file": "Override" }
            }"#,
        )
        .expect("language config should be written");

        let dirs = SplitypeConfigDirs::from_root(&root);
        let mut manager = I18nManager::default();
        let err = manager
            .import_language_config_with_dirs(&source, &dirs)
            .expect_err("built-in language ids should be rejected");
        assert!(err.to_string().contains("built-in language"));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn custom_language_pack_json_overlays_base_strings() {
        let pack = I18nLanguagePack::from_json(
            r#"{
                "id": "zh-CN",
                "name": "简体中文",
                "strings": {
                    "menu_file": "文件菜单",
                    "custom_extension": "extra_val",
                    "unknown_field": "ignored"
                }
            }"#,
        )
        .expect("language pack should load");

        assert_eq!(pack.id, "zh-CN");
        assert_eq!(pack.name, "简体中文");
        assert_eq!(pack.strings.menu_file, "文件菜单");
        assert_eq!(pack.strings.menu_export, "导出");
        assert_eq!(pack.strings.info_dialog_ok, "确定");
        assert_eq!(pack.strings.update_open_release, "前往下载");
        assert_eq!(pack.strings.help_about_github_label, "GitHub");
        assert_eq!(
            pack.strings.help_about_star_message,
            "如果本项目对您有帮助，那不妨给本项目一颗 Star⭐，十分感谢！"
        );
    }

    #[test]
    fn unknown_language_pack_falls_back_to_english_strings() {
        let pack = I18nLanguagePack::from_json(
            r#"{
                "id": "fr-FR",
                "strings": {
                    "menu_file": "Fichier"
                }
            }"#,
        )
        .expect("language pack should load");

        assert_eq!(pack.id, "fr-FR");
        assert_eq!(pack.name, "fr-FR");
        assert_eq!(pack.strings.menu_file, "Fichier");
        assert_eq!(pack.strings.menu_export, "Export");
        assert_eq!(pack.strings.info_dialog_ok, "OK");
        assert_eq!(pack.strings.update_open_release, "Open Releases");
        assert_eq!(pack.strings.menu_open_recent_file, "Open Recent File");
        assert_eq!(pack.strings.menu_no_recent_files, "No Recent Files");
        assert_eq!(
            pack.strings.recent_file_missing_title,
            "Recent File Missing"
        );
        assert_eq!(pack.strings.help_about_github_label, "GitHub");
        assert_eq!(
            pack.strings.help_about_star_message,
            "If this project helps you, consider giving it a Star⭐. Thank you!"
        );
    }
}
