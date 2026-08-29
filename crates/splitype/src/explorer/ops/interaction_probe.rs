//! Interactive explorer probes (local development only).
//!
//! Data-layer regression tests live in the modules they exercise (see
//! `worktree.rs` / `state.rs`); these probes drive the panel-level state
//! machine (add worktree → scan → toggle expand) exactly the way the UI
//! click handlers do, so a broken tree build or expansion path surfaces
//! here first.
//!
//! The explorer model lives on the Shell (window entity), so each probe
//! builds a minimal Shell with one Editor content entity — the same wiring
//! the real window uses.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gpui::{AppContext, TestAppContext};

use crate::app::shell::{PanelContent, Shell};
use crate::app::window::chrome::MenuBarState;
use crate::app::window::panels::WindowPanels;
use workspace::{PanelId, DEFAULT_EDITOR_PANEL_ID};
use crate::editor::engine::controller::Editor;
use crate::explorer::state::state::*;

fn init_explorer_test_app(cx: &mut TestAppContext) {
    cx.update(|cx| {
        i18n::I18nManager::init(cx);
        theme::ThemeManager::init(cx);
        crate::editor::keybindings::init(cx);
        config::settings::SettingsStore::init_default(cx);
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

fn new_test_shell(cx: &mut TestAppContext) -> gpui::Entity<Shell> {
    cx.new(|cx| {
        let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));
        Shell {
            panel_contents: [(PanelId(DEFAULT_EDITOR_PANEL_ID), PanelContent::Editor(editor))].into(),
            retained_editor_sessions: HashMap::new(),
            menu_bar: MenuBarState::default(),
            panels: WindowPanels::default(),
            last_viewport: None,
            explorer_file_menu: None,
            info_dialog: None,
            unsaved_dialog: None,
            update_check_in_progress: false,
            close_guard_installed: false,
            about_bg_emojis: Vec::new(),
        }
    })
}

/// Helper: collect visible entry labels from the flat row list.
fn visible_labels(shell: &Shell) -> Vec<String> {
    shell
        .panels
        .explorer
        .entries
        .iter()
        .filter_map(|row| match row {
            ExplorerRow::Entry(entry) => Some(entry.label.clone()),
            _ => None,
        })
        .collect()
}

/// A worktree rooted at a single file (added via the import/replace
/// dialogs) renders as a file row: the scan keeps the file as the root and
/// the tree builder tags it as a file, not a folder.
#[gpui::test]
fn single_file_worktree_roots_at_the_file(cx: &mut TestAppContext) {
    init_explorer_test_app(cx);
    let root = temp_explorer_root("file-root");
    let file = root.join("top.md");

    let shell = new_test_shell(cx);
    shell.update(cx, |shell, cx| {
        shell.add_explorer_worktree(file.clone(), cx);
    });
    cx.run_until_parked();

    shell.read_with(cx, |shell, _cx| {
        let snaps = &shell.panels.explorer.snapshots;
        assert_eq!(snaps.len(), 1, "one worktree");
        let snap = &snaps[0];
        let root_entry = snap.root_entry().expect("root entry exists");
        assert_eq!(root_entry.path, file);

        let rows = visible_labels(shell);
        assert_eq!(rows, vec!["top.md"], "only the file row is visible");
    });
}

/// The scan must produce a tree in which the subfolder carries its files,
/// and `toggle_explorer_node` (the row click / arrow handler) must make
/// those children visible.
#[gpui::test]
fn toggle_on_subfolder_reveals_its_children(cx: &mut TestAppContext) {
    init_explorer_test_app(cx);
    let root = temp_explorer_root("toggle-subfolder");

    let shell = new_test_shell(cx);
    shell.update(cx, |shell, cx| {
        shell.add_explorer_worktree(root.clone(), cx);
    });
    cx.run_until_parked();

    let subdir_id = shell.read_with(cx, |shell, _cx| {
        let snaps = &shell.panels.explorer.snapshots;
        assert_eq!(snaps.len(), 1, "one worktree");
        let snap = &snaps[0];
        let root_entry = snap.root_entry().expect("root entry");
        assert_eq!(
            root_entry.kind,
            crate::explorer::state::worktree::WorktreeEntryKind::Directory
        );

        // Verify scan found subdir
        let subdir_path = root.join("subdir");
        let subdir_entry = snap
            .entry_for_path(&subdir_path)
            .expect("subdir present in snapshot");

        // Verify subdir children exist in snapshot
        let child_names: Vec<&str> = snap
            .child_entries(&subdir_entry.path)
            .filter_map(|e| e.path.file_name().and_then(|n| n.to_str()))
            .collect();
        assert!(
            child_names.contains(&"inner.md"),
            "scan kept inner.md: {child_names:?}"
        );
        assert!(
            child_names.contains(&"inner.txt"),
            "scan kept inner.txt: {child_names:?}"
        );

        // Root row is expanded by default → subfolder already visible.
        let labels_before = visible_labels(shell);
        assert!(
            labels_before.contains(&"subdir".to_string()),
            "{labels_before:?}"
        );
        assert!(
            !labels_before.contains(&"inner.md".to_string()),
            "{labels_before:?}"
        );
        subdir_entry.id
    });

    // Toggle the subfolder (what the row click does) → children appear.
    shell.update(cx, |shell, cx| {
        shell.toggle_explorer_node(subdir_id, cx);
    });
    shell.read_with(cx, |shell, _cx| {
        let labels_after = visible_labels(shell);
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
    shell.update(cx, |shell, cx| {
        shell.toggle_explorer_node(subdir_id, cx);
    });
    shell.read_with(cx, |shell, _cx| {
        let labels_collapsed = visible_labels(shell);
        assert!(
            !labels_collapsed.contains(&"inner.md".to_string()),
            "{labels_collapsed:?}"
        );
    });
}

/// The rescan triggered by a background filesystem change must keep the
/// expansion set alive (stable ids), so an expanded folder stays expanded.
#[gpui::test]
fn rescan_preserves_expanded_subfolder(cx: &mut TestAppContext) {
    init_explorer_test_app(cx);
    let root = temp_explorer_root("rescan-expansion");

    let shell = new_test_shell(cx);
    shell.update(cx, |shell, cx| {
        shell.add_explorer_worktree(root.clone(), cx);
    });
    cx.run_until_parked();

    let subdir_id = shell.read_with(cx, |shell, _cx| {
        let snap = &shell.panels.explorer.snapshots[0];
        let subdir_path = root.join("subdir");
        snap.entry_for_path(&subdir_path)
            .expect("subdir in snapshot")
            .id
    });
    shell.update(cx, |shell, cx| {
        shell.toggle_explorer_node(subdir_id, cx);
    });

    // Simulate the fs watcher firing: force a full rescan.
    shell.update(cx, |shell, cx| {
        shell.rescan_explorer_worktrees(cx);
    });
    cx.run_until_parked();

    shell.read_with(cx, |shell, _cx| {
        let labels = visible_labels(shell);
        assert!(
            labels.contains(&"inner.md".to_string()),
            "expansion survived the rescan: {labels:?}"
        );
    });
}

/// Closing the explorer folder must clear worktrees, snapshots, and
/// visible rows even when the editor has an active document from that folder.
#[gpui::test]
fn close_explorer_folder_clears_trees_and_entries_even_with_open_file(cx: &mut TestAppContext) {
    init_explorer_test_app(cx);
    let root = temp_explorer_root("close-explorer");
    let file = root.join("top.md");

    let shell = cx.new(|cx| {
        let editor = cx.new(|cx| Editor::from_markdown(cx, "# top".to_string(), Some(file.clone())));
        Shell {
            panel_contents: [(PanelId(DEFAULT_EDITOR_PANEL_ID), PanelContent::Editor(editor))].into(),
            retained_editor_sessions: HashMap::new(),
            menu_bar: MenuBarState::default(),
            panels: WindowPanels::default(),
            last_viewport: None,
            explorer_file_menu: None,
            info_dialog: None,
            unsaved_dialog: None,
            update_check_in_progress: false,
            close_guard_installed: false,
            about_bg_emojis: Vec::new(),
        }
    });

    shell.update(cx, |shell, cx| {
        shell.add_explorer_worktree(root.clone(), cx);
    });
    cx.run_until_parked();

    shell.read_with(cx, |shell, _cx| {
        assert_eq!(shell.panels.explorer.worktrees.len(), 1);
        assert!(!shell.panels.explorer.snapshots.is_empty());
        assert!(!shell.panels.explorer.entries.is_empty());
    });

    shell.update(cx, |shell, cx| {
        shell.close_explorer_folder(cx);
    });

    // Simulate subsequent frame syncs
    shell.update(cx, |shell, cx| {
        shell.sync_explorer_models(cx);
    });

    shell.read_with(cx, |shell, _cx| {
        assert!(
            shell.panels.explorer.worktrees.is_empty(),
            "worktrees must remain empty after close"
        );
        assert!(
            shell.panels.explorer.snapshots.is_empty(),
            "snapshots must remain empty after close"
        );
        assert!(
            shell.panels.explorer.entries.is_empty(),
            "entries must remain empty after close"
        );
    });
}
