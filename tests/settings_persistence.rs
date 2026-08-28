//! Cross-crate contract tests for configuration persistence.
//!
//! Exercises recent-file tracking and JSONC parsing through the public
//! `splitype-infra` API, isolating all file I/O under a temp root so the
//! user's real configuration is never touched.

use splitype::infra::config::dirs::SplitypeConfigDirs;
use splitype::infra::config::jsonc::{parse_jsonc_value, sanitize_config_file_stem};
use splitype::infra::config::recent::{
    read_recent_files_with_dirs, read_recent_folders_with_dirs, record_recent_file_with_dirs,
    record_recent_folder_with_dirs, remove_recent_file_with_dirs,
};

struct TempRoot(std::path::PathBuf);

impl TempRoot {
    fn new(name: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("splitype-it-{name}-{nanos}"));
        std::fs::create_dir_all(&root).expect("create temp root");
        Self(root)
    }

    fn dirs(&self) -> SplitypeConfigDirs {
        SplitypeConfigDirs::from_root(self.0.clone())
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn temp_markdown_path(root: &TempRoot, name: &str) -> std::path::PathBuf {
    let path = root.0.join(name);
    std::fs::write(&path, "hello").expect("write temp file");
    path
}

/// Recorded recent files round-trip through persistence.
#[test]
fn recent_files_round_trip() {
    let root = TempRoot::new("files");
    let a = temp_markdown_path(&root, "a.md");
    let b = temp_markdown_path(&root, "b.md");

    record_recent_file_with_dirs(&a, &root.dirs()).expect("record a");
    record_recent_file_with_dirs(&b, &root.dirs()).expect("record b");

    let recent = read_recent_files_with_dirs(&root.dirs()).expect("read");
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0], b, "most recent file comes first");
    assert_eq!(recent[1], a);
}

/// Removing a recent file drops it from the list.
#[test]
fn recent_file_removal() {
    let root = TempRoot::new("remove");
    let a = temp_markdown_path(&root, "a.md");
    let b = temp_markdown_path(&root, "b.md");
    record_recent_file_with_dirs(&a, &root.dirs()).expect("record a");
    record_recent_file_with_dirs(&b, &root.dirs()).expect("record b");

    remove_recent_file_with_dirs(&a, &root.dirs()).expect("remove a");
    let recent = read_recent_files_with_dirs(&root.dirs()).expect("read");
    assert_eq!(recent, vec![b]);
}

/// Recent folders follow the same persistence contract.
#[test]
fn recent_folders_round_trip() {
    let root = TempRoot::new("folders");
    let dir_a = root.0.join("folder-a");
    let dir_b = root.0.join("folder-b");
    std::fs::create_dir_all(&dir_a).expect("mkdir a");
    std::fs::create_dir_all(&dir_b).expect("mkdir b");

    record_recent_folder_with_dirs(&dir_a, &root.dirs()).expect("record a");
    record_recent_folder_with_dirs(&dir_b, &root.dirs()).expect("record b");

    let recent = read_recent_folders_with_dirs(&root.dirs()).expect("read");
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0], dir_b);
    assert_eq!(recent[1], dir_a);
}

/// JSONC parsing strips comments but keeps values.
#[test]
fn jsonc_parser_strips_comments() {
    let jsonc = r#"{
        // line comment
        "theme": "dark", /* block comment */
        "recent": ["a.md", "b.md"] // trailing
    }"#;
    let value = parse_jsonc_value(jsonc).expect("parse jsonc");
    assert_eq!(value["theme"], serde_json::json!("dark"));
    assert_eq!(value["recent"][1], serde_json::json!("b.md"));
}

/// Config file stems are sanitized to safe filenames.
#[test]
fn config_stem_sanitization() {
    // Whitespace becomes underscores; separators are dropped.
    assert_eq!(sanitize_config_file_stem("My Theme"), "My_Theme");
    assert_eq!(sanitize_config_file_stem("日本語"), "日本語");
    assert_eq!(sanitize_config_file_stem("a/b\\c"), "abc");
    // Empty or symbol-only input falls back to "custom".
    assert_eq!(sanitize_config_file_stem("///"), "custom");
}

/// JSONC parsing handles trailing commas in objects and arrays.
#[test]
fn jsonc_parser_strips_trailing_commas() {
    let jsonc = r#"{
        "theme": "dark",
        "recent": [
            "a.md",
            "b.md",
        ],
    }"#;
    let value = parse_jsonc_value(jsonc).expect("parse jsonc with trailing commas");
    assert_eq!(value["theme"], serde_json::json!("dark"));
    assert_eq!(value["recent"][1], serde_json::json!("b.md"));
}

/// Comprehensive settings round-trip test covering all domain categories.
#[test]
fn app_settings_comprehensive_persistence() {
    use splitype::infra::config::settings::{
        AppSettings, EditorBehaviorSettings, ExplorerSettings, ExplorerSortMode,
        ExplorerSortOrder, ImagePasteBehavior, InterfaceSettings, MarkdownSettings,
        StartupOpenSetting, StartupSettings, StatusBarSettings, TypographySettings,
        read_app_settings_with_dirs, save_app_settings_with_dirs,
    };

    let root = TempRoot::new("comprehensive");
    let dirs = root.dirs();

    let initial = read_app_settings_with_dirs(&dirs).expect("read initial");
    assert_eq!(initial.startup.open, StartupOpenSetting::NewFile);
    assert_eq!(initial.interface.theme_id, "splitype");
    assert_eq!(initial.editor.tab_size, 4);
    assert_eq!(initial.markdown.show_table_headers, true);
    assert_eq!(initial.explorer.hide_hidden, true);

    let mut custom = AppSettings {
        startup: StartupSettings {
            open: StartupOpenSetting::LastOpenedFile,
            restore_window_state: true,
        },
        interface: InterfaceSettings {
            theme_id: "solarized_dark".to_string(),
            language_id: "zh-CN".to_string(),
        },
        status_bar: StatusBarSettings {
            enabled: true,
            show_word_count: true,
            show_cursor_position: true,
            show_character_count: true,
            show_reading_time: true,
        },
        editor: EditorBehaviorSettings {
            line_numbers: false,
            word_wrap: false,
            tab_size: 2,
            insert_spaces: false,
            highlight_active_line: true,
        },
        typography: TypographySettings {
            ui_font_family: Some("Segoe UI".to_string()),
            prose_font_family: Some("Georgia".to_string()),
            code_font_family: Some("Cascadia Code".to_string()),
            font_size: 18,
            line_height: 1.75,
        },
        markdown: MarkdownSettings {
            show_table_headers: false,
            image_paste_behavior: ImagePasteBehavior::CopyToNamedAssetsFolder,
            render_math: true,
            render_diagrams: true,
        },
        explorer: ExplorerSettings {
            hide_hidden: false,
            sort_mode: ExplorerSortMode::DirectoriesFirst,
            sort_order: ExplorerSortOrder::Descending,
            auto_reveal: false,
        },
        keybindings: std::collections::BTreeMap::new(),
    };
    custom.keybindings.insert(
        "save_document".to_string(),
        vec!["ctrl-alt-s".to_string()],
    );

    save_app_settings_with_dirs(&custom, &dirs).expect("save custom settings");

    let reloaded = read_app_settings_with_dirs(&dirs).expect("reload custom settings");
    assert_eq!(reloaded, custom);
    assert_eq!(reloaded.startup.open, StartupOpenSetting::LastOpenedFile);
    assert_eq!(reloaded.interface.language_id, "zh-CN");
    assert_eq!(reloaded.editor.tab_size, 2);
    assert_eq!(reloaded.editor.line_numbers, false);
    assert_eq!(reloaded.markdown.image_paste_behavior, ImagePasteBehavior::CopyToNamedAssetsFolder);
    assert_eq!(reloaded.explorer.sort_order, ExplorerSortOrder::Descending);
    assert_eq!(reloaded.status_bar.show_reading_time, true);
    assert_eq!(
        reloaded.keybindings.get("save_document"),
        Some(&vec!["ctrl-alt-s".to_string()])
    );
}

