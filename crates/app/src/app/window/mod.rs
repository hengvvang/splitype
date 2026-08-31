pub(crate) mod chrome;
pub(crate) mod dialogs;
pub(crate) mod layout;
pub mod panels;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use gpui::*;

use crate::app::menus::install_menus;
use crate::app::shell::{Shell, ShellEditorHost};
use crate::app::window::chrome::MenuBarState;
use crate::app::window::panels::WindowPanels;
use workspace::{PanelId, PanelKindId, PanelView, DEFAULT_EDITOR_PANEL_ID, ROOT_PANEL_ID};
use editor::{Editor, EditorSession};

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
                // The explorer state is app-wide global; install a fresh
                // one per window (the shell renders it from the global).
                cx.set_global(explorer::ExplorerState::default());
                let editor = cx.new(|cx| { let mut ed = Editor::new(markdown, file_path, cx); ed.set_panel_id(PanelId(DEFAULT_EDITOR_PANEL_ID)); ed });
                let explorer_view: Box<dyn PanelView> =
                    Box::new(explorer::ExplorerPanelView::new(PanelId(ROOT_PANEL_ID)));
                let editor_view: Box<dyn PanelView> =
                    Box::new(editor::EditorPanelView::new(editor.clone()));

                let shell = cx.new(move |_cx| Shell {
                    // The default layout is Explorer (left) + Editor (right).
                    panel_views: [
                        (PanelId(ROOT_PANEL_ID), explorer_view),
                        (PanelId(DEFAULT_EDITOR_PANEL_ID), editor_view),
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
                editor.update(cx, |e, _cx| {
                    e.host = Some(std::sync::Arc::new(ShellEditorHost::new(shell_weak.clone())));
                });
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
    tree: SplitTree<PanelKindId>,
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
                // Materialize one Editor entity per cloned session, and ensure all leaves exist.
                let mut panel_views: HashMap<PanelId, Box<dyn PanelView>> = HashMap::new();
                for (panel_id, session) in sessions {
                    let editor = cx.new(|cx| editor::Editor::with_session(panel_id, session, cx));
                    panel_views.insert(panel_id, Box::new(editor::EditorPanelView::new(editor)));
                }

                let mut leaf_ids = Vec::new();
                tree.leaf_ids(&mut leaf_ids);
                let registry = workspace::PanelRegistry::global().lock().unwrap();
                for leaf_id in leaf_ids {
                    let panel_id = PanelId(leaf_id);
                    if let Some(kind) = tree.find_leaf_kind(leaf_id) {
                        if !panel_views.contains_key(&panel_id) {
                            if let Some(view) = registry.create_panel(kind, panel_id, cx) {
                                panel_views.insert(panel_id, view);
                            }
                        }
                    }
                }

                // The Shell owns the cloned outer layout; the explorer
                // state travels as the app-wide global (a fresh state when
                // the drag carried none).
                cx.set_global(explorer.unwrap_or_else(explorer::ExplorerState::default));
                let mut panels = WindowPanels::default();
                panels.layout.tree = tree;
                panels.layout.next_node_id = next_node_id;
                // Activate the first Editor leaf of the cloned layout
                if let Some(container) = panels.layout.tree.find_first_leaf_by_kind(PanelKindId::EDITOR) {
                    panels.layout.activate_leaf(container.id);
                } else {
                    panels.layout.active_leaf = None;
                    panels.layout.activation_history.clear();
                }
                let shell = cx.new(move |_cx| Shell {
                    panel_views,
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
                    .panel_views
                    .values()
                    .filter_map(|view| view.as_any().downcast_ref::<editor::EditorPanelView>())
                    .map(|p| p.editor.clone())
                    .collect();
                for editor in editors {
                    editor.update(cx, |e, _cx| {
                        e.host = Some(std::sync::Arc::new(ShellEditorHost::new(shell_weak.clone())));
                    });
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

