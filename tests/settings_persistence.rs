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
