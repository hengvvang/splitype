pub(crate) mod chrome;
pub(crate) mod dialogs;
pub(crate) mod layout;
pub mod panels;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use gpui::*;

use crate::app::menus::install_menus;
use crate::app::shell::{PanelContent, Shell};
use crate::app::window::chrome::MenuBarState;
use crate::app::window::panels::WindowPanels;
use workspace::{PanelId, DEFAULT_EDITOR_PANEL_ID, ROOT_PANEL_ID, WindowPanelKind};
use crate::editor::engine::controller::Editor;
use crate::editor::engine::session::EditorSession;

use explorer::ExplorerState;

use config::recent::record_recent_file;
use splitter::NodeId;
use ui::custom_titlebar::splitype_window_options;
use splitter::tree::SplitTree;

fn window_title(file_path: Option<&Path>) -> SharedString {
    if let Some(path) = file_path {
        // OsStr::to_string_lossy returns Cow<str>; calling .to_string() on
        // it always allocates a fresh String, even for the valid-UTF-8 path
        // (the common case). Borrow the Cow directly into format! — its
        // Display impl writes the borrowed bytes straight into the output
        // String, no intermediate allocation.
        format!(
            "Splitype - {}",
            path.file_name()
                .map(|name| name.to_string_lossy())
                .unwrap_or_else(|| "".into())
        )
        .into()
    } else {
        "Splitype".into()
    }
}

/// Opens an editor window for the given Markdown content and optional path.
pub(crate) fn open_editor_window(
    cx: &mut App,
    markdown: String,
    file_path: Option<PathBuf>,
) -> WindowHandle<Shell> {
    if let Some(ref path) = file_path {
        let _ = record_recent_file(path);
    }
    let title = window_title(file_path.as_deref());
    let bounds = Bounds::centered(None, size(px(1024.0), px(768.0)), cx);
    let handle = cx
        .open_window(
            splitype_window_options(title, bounds),
            move |_window, cx| {
                let editor = cx.new(|cx| {
                    // No content and no path → welcome state with zero tabs.
                    if markdown.is_empty() && file_path.is_none() {
                        crate::editor::Editor::empty(cx)
                    } else {
                        crate::editor::Editor::from_markdown(cx, markdown, file_path)
                    }

                });
                let shell = cx.new(move |_cx| Shell {
                    // The default layout is Explorer (left) + Editor (right);
                    // only Editor panel_contents carry content entities.
                    panel_contents: [
                        (PanelId(ROOT_PANEL_ID), PanelContent::Explorer),
                        (PanelId(DEFAULT_EDITOR_PANEL_ID), PanelContent::Editor(editor)),
                    ]
                    .into(),
                    retained_editor_sessions: HashMap::new(),
                    menu_bar: MenuBarState::default(),
                    panels: WindowPanels::default(),
                    last_viewport: None,
                    info_dialog: None,
                    unsaved_dialog: None,
                    update_check_in_progress: false,
                    close_guard_installed: false,
                    about_bg_emojis: Vec::new(),
                });
                // Wire the editor entity to its Shell.
                let shell_weak = shell.downgrade();
                let editors: Vec<Entity<Editor>> = shell
                    .read(cx)
                    .panel_contents
                    .values()
                    .filter_map(|content| content.as_editor().cloned())
                    .collect();
                for editor in editors {
                    editor.update(cx, |e, _cx| e.shell = Some(shell_weak.clone()));
                }
                shell
            },
        )
        .unwrap();

    handle
        .update(cx, |shell, window, cx| {
            window.activate_window();
            shell.force_install_close_guard(window, cx);
        })
        .expect("newly opened shell window should be updateable");

    handle
}

/// Opens a new window hosting a cloned sub-tree handed over by a
/// Shift-drag gesture. Materializes one `Editor` entity per cloned
/// session, inherits the window layout tree, and clones the file
/// explorer state so the new window sees the same directory tree.
pub(crate) fn open_cloned_window(
    tree: SplitTree<WindowPanelKind>,
    next_node_id: NodeId,
    sessions: HashMap<PanelId, EditorSession>,
    explorer: Option<ExplorerState>,
    cx: &mut App,
) -> WindowHandle<Shell> {
    let bounds = Bounds::centered(None, size(px(1024.0), px(768.0)), cx);
    let handle = cx
        .open_window(
            splitype_window_options(SharedString::new("Splitype"), bounds),
            move |_window, cx| {
                // Materialize one Editor entity per cloned session.
                let mut panel_contents = HashMap::new();
                for (panel_id, session) in sessions {
                    let editor = cx.new(|cx| crate::editor::Editor::with_session(panel_id, session, cx));

                    panel_contents.insert(panel_id, PanelContent::Editor(editor));
                }
                // The Shell owns the cloned outer layout; the explorer
                // state travels as the app-wide global.
                let mut panels = WindowPanels::default();
                panels.layout.tree = tree;
                panels.layout.next_node_id = next_node_id;
                if let Some(explorer) = explorer {
                    cx.set_global(explorer);
                }
                // Activate the first Editor leaf of the cloned layout
                if let Some(container) = panels.layout.tree.find_first_leaf_by_kind(WindowPanelKind::Editor) {
                    panels.layout.activate_leaf(container.id);
                } else {
                    panels.layout.active_leaf = None;
                    panels.layout.activation_history.clear();
                }
                let shell = cx.new(move |_cx| Shell {
                    panel_contents,
                    retained_editor_sessions: HashMap::new(),
                    menu_bar: MenuBarState::default(),
                    panels,
                    last_viewport: None,
                    info_dialog: None,
                    unsaved_dialog: None,
                    update_check_in_progress: false,
                    close_guard_installed: false,
                    about_bg_emojis: Vec::new(),
                });
                // Wire every editor entity to its Shell.
                let shell_weak = shell.downgrade();
                let editors: Vec<Entity<Editor>> = shell
                    .read(cx)
                    .panel_contents
                    .values()
                    .filter_map(|content| content.as_editor().cloned())
                    .collect();
                for editor in editors {
                    editor.update(cx, |e, _cx| e.shell = Some(shell_weak.clone()));
                }
                shell
            },
        )
        .unwrap();

    handle
        .update(cx, |shell, window, cx| {
            window.activate_window();
            shell.force_install_close_guard(window, cx);
        })
        .expect("newly opened shell window should be updateable");

    handle
}

pub(crate) fn open_file_in_new_window(cx: &mut App, path: &Path) -> anyhow::Result<()> {
    let markdown = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read '{}'", path.display()))?;
    open_editor_window(cx, markdown, Some(path.to_path_buf()));
    record_recent_file_and_refresh(path, cx);
    Ok(())
}

pub(crate) fn record_recent_file_and_refresh(path: &Path, cx: &mut App) {
    if let Err(err) = record_recent_file(path) {
        tracing::warn!(path = %path.display(), error = %err, "failed to update recent file history");
        return;
    }
    install_menus(cx);
    cx.refresh_windows();
}
