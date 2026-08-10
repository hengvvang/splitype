//! Interactive explorer probes (local development only).
//!
//! Data-layer regression tests live in the modules they exercise (see
//! `worktree.rs` / `state.rs`); these probes drive the panel-level state
//! machine (add worktree → scan → toggle expand) exactly the way the UI
//! click handlers do, so a broken tree build or expansion path surfaces
//! here first.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gpui::{AppContext, TestAppContext};

use crate::editor::controller::Editor;
use crate::editor::explorer::state::*;

fn init_explorer_test_app(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::infra::i18n::I18nManager::init(cx);
        crate::infra::theme::ThemeManager::init(cx);
        crate::editor::keybindings::init(cx);
        crate::infra::config::settings::ExplorerSettingsStore::init(cx);
    });
}

fn temp_explorer_root(test_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "splitype-explorer-{test_name}-{}-{nanos}",
        std::process::id()
    ));
    let sub = root.join("subdir");
    fs::create_dir_all(&sub).expect("create temp tree");
    fs::write(sub.join("inner.md"), "# inner").expect("write inner.md");
    fs::write(sub.join("inner.txt"), "inner").expect("write inner.txt");
    fs::write(root.join("top.md"), "# top").expect("write top.md");
    root
}

fn visible_labels(editor: &Editor) -> Vec<String> {
    editor
        .panels
        .explorer
        .entries
        .iter()
        .filter_map(|row| match row {
            ExplorerRow::Entry(entry) => Some(entry.label.clone()),
            ExplorerRow::Edit { .. } => None,
        })
        .collect()
}

/// A worktree rooted at a single file (added via the import/replace
/// dialogs) renders as a file row: the scan keeps the file as the root and
/// the tree builder tags it as a file, not a folder.
#[gpui::test]
async fn single_file_worktree_roots_at_the_file(cx: &mut TestAppContext) {
    init_explorer_test_app(cx);
    let root = temp_explorer_root("file-root");
    let file = root.join("top.md");

    let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));
    editor.update(cx, |editor, cx| {
        editor.add_explorer_worktree(file.clone(), cx);
    });
    cx.run_until_parked();

    editor.read_with(cx, |editor, _cx| {
        let trees = &editor.panels.explorer.trees_cache;
        assert_eq!(trees.len(), 1, "one worktree");
        let root_node = &trees[0];
        assert_eq!(
            root_node.kind,
            ExplorerEntryKind::MarkdownFile,
            "a .md root is a markdown file, not a directory"
        );
        assert_eq!(root_node.path, file);
        assert!(root_node.children.is_empty());

        let rows = visible_labels(editor);
        assert_eq!(rows, vec!["top.md"], "only the file row is visible");
    });
}

/// The scan must produce a tree in which the subfolder carries its files,
/// and `toggle_explorer_node` (the row click / arrow handler) must make
/// those children visible.
#[gpui::test]
async fn toggle_on_subfolder_reveals_its_children(cx: &mut TestAppContext) {
    init_explorer_test_app(cx);
    let root = temp_explorer_root("toggle-subfolder");

    let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));
    editor.update(cx, |editor, cx| {
        editor.add_explorer_worktree(root.clone(), cx);
    });
    cx.run_until_parked();

    let subdir_id = editor.read_with(cx, |editor, _cx| {
        let trees = &editor.panels.explorer.trees_cache;
        assert_eq!(trees.len(), 1, "one worktree");
        let root_node = &trees[0];
        assert_eq!(root_node.kind, ExplorerEntryKind::Directory);
        assert!(
            root_node.children.iter().any(|c| c.label == "top.md"),
            "tree contains top.md: {:?}",
            root_node
                .children
                .iter()
                .map(|c| c.label.as_str())
                .collect::<Vec<_>>()
        );
        let subdir = root_node
            .children
            .iter()
            .find(|c| c.label == "subdir")
            .expect("subdir present in tree");
        assert_eq!(subdir.kind, ExplorerEntryKind::Directory);
        assert_eq!(
            subdir
                .children
                .iter()
                .map(|c| c.label.as_str())
                .collect::<Vec<_>>(),
            vec!["inner.md", "inner.txt"],
            "scan kept the files inside the subfolder"
        );

        // Root row is expanded by default → subfolder already visible.
        let labels_before = visible_labels(editor);
        assert!(
            labels_before.contains(&"subdir".to_string()),
            "{labels_before:?}"
        );
        assert!(
            !labels_before.contains(&"inner.md".to_string()),
            "{labels_before:?}"
        );
        subdir.id
    });

    // Toggle the subfolder (what the row click does) → children appear.
    editor.update(cx, |editor, cx| {
        editor.toggle_explorer_node(subdir_id, cx);
    });
    editor.read_with(cx, |editor, _cx| {
        let labels_after = visible_labels(editor);
        assert!(
            labels_after.contains(&"inner.md".to_string()),
            "inner.md visible after toggle: {labels_after:?}"
        );
        assert!(
            labels_after.contains(&"inner.txt".to_string()),
            "inner.txt visible after toggle: {labels_after:?}"
        );
    });

    // Toggling again collapses it back.
    editor.update(cx, |editor, cx| {
        editor.toggle_explorer_node(subdir_id, cx);
    });
    editor.read_with(cx, |editor, _cx| {
        let labels_collapsed = visible_labels(editor);
        assert!(
            !labels_collapsed.contains(&"inner.md".to_string()),
            "{labels_collapsed:?}"
        );
    });
}

/// The rescan triggered by a background filesystem change must keep the
/// expansion set alive (stable ids), so an expanded folder stays expanded.
#[gpui::test]
async fn rescan_preserves_expanded_subfolder(cx: &mut TestAppContext) {
    init_explorer_test_app(cx);
    let root = temp_explorer_root("rescan-expansion");

    let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));
    editor.update(cx, |editor, cx| {
        editor.add_explorer_worktree(root.clone(), cx);
    });
    cx.run_until_parked();

    let subdir_id = editor.read_with(cx, |editor, _cx| {
        editor
            .panels
            .explorer
            .trees_cache
            .first()
            .unwrap()
            .children
            .iter()
            .find(|c| c.label == "subdir")
            .unwrap()
            .id
    });
    editor.update(cx, |editor, cx| {
        editor.toggle_explorer_node(subdir_id, cx);
    });

    // Simulate the fs watcher firing: force a full rescan.
    editor.update(cx, |editor, cx| {
        editor.rescan_explorer_worktrees(cx);
    });
    cx.run_until_parked();

    editor.read_with(cx, |editor, _cx| {
        let labels = visible_labels(editor);
        assert!(
            labels.contains(&"inner.md".to_string()),
            "expansion survived the rescan: {labels:?}"
        );
    });
}
