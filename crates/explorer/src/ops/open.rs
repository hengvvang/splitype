//! Opening files from the explorer: single click (transient preview tab),
//! double click (permanent tab + focus editor), and Ctrl/Cmd+double-click
//! (open in a freshly split area).
//!
//! The explorer updates its own selection/reveal state and dispatches
//! `workspace` open actions; the shell performs the editor orchestration.

use std::path::Path;

use gpui::*;

use workspace::actions::{OpenInEditor, OpenInSplit};

use crate::state::state::ExplorerState;

impl ExplorerState {
    /// Select and reveal `path` in the tree: expand ancestor directories,
    /// rebuild the visible rows, and scroll the selection into view.
    pub(crate) fn reveal_and_select(&mut self, path: &Path) {
        self.selected = self.explorer_id_for_path(path);
        self.expand_to_path(path);
        self.rebuild_explorer_entries();
        self.autoscroll_explorer_selection();
    }

    /// Open a file: single click opens a transient temporary tab, double
    /// click opens a persistent tab (the shell focuses the editor pane).
    pub(crate) fn open_explorer_file(
        &mut self,
        path: std::path::PathBuf,
        persistent: bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.reveal_and_select(&path);
        window.dispatch_action(
            Box::new(OpenInEditor {
                path: path.to_string_lossy().into_owned(),
                persistent,
            }),
            cx,
        );
    }

    /// Open a file from a row click.
    pub(crate) fn open_explorer_file_click(
        &mut self,
        path: std::path::PathBuf,
        is_double_click: bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.open_explorer_file(path, is_double_click, window, cx);
    }

    /// Ctrl/Cmd+double-click: open the file in a freshly split editor area
    /// (mirrors Zed's split-on-open).
    pub(crate) fn split_explorer_file(
        &mut self,
        path: std::path::PathBuf,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.dispatch_action(
            Box::new(OpenInSplit {
                path: path.to_string_lossy().into_owned(),
            }),
            cx,
        );
    }

    /// Notify the shell that open editor tabs may need their paths updated
    /// after a filesystem move/rename.
    pub(crate) fn sync_open_tabs_after_fs_change(
        &mut self,
        change: &crate::state::undo::ExplorerChange,
        window: &mut Window,
        cx: &mut App,
    ) {
        match change {
            crate::state::undo::ExplorerChange::Moved { from, to }
            | crate::state::undo::ExplorerChange::Renamed { from, to } => {
                window.dispatch_action(
                    Box::new(crate::ops::selection::UpdateOpenTabPaths {
                        from: from.to_string_lossy().into_owned(),
                        to: to.to_string_lossy().into_owned(),
                    }),
                    cx,
                );
            }
            crate::state::undo::ExplorerChange::Batch(changes) => {
                for change in changes {
                    self.sync_open_tabs_after_fs_change(change, window, cx);
                }
            }
            _ => {}
        }
    }
}
