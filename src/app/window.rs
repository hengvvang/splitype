//! Editor window creation and file-open routing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use gpui::*;

use crate::app::menus::install_menus;
use crate::app::shell::{AreaContent, Shell};
use crate::app::window_area::{DEFAULT_EDITOR_AREA_ID, WindowAreaKind};
use crate::app::window_chrome::MenuBarState;
use crate::editor::controller::Editor;
use crate::editor::explorer_state::state::ExplorerState;
use crate::editor::session::EditorSession;
use crate::infra::config::recent::record_recent_file;
use crate::splitter::NodeId;
use crate::ui::custom_titlebar::splitype_window_options;
use splitype_splitter::tree::SplitTree;

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
                .unwrap_or_else(|| path.to_string_lossy())
        )
        .into()
    } else {
        SharedString::new("Splitype")
    }
}

/// Opens an editor window for the given Markdown content and optional path.
pub(crate) fn open_editor_window(
    cx: &mut App,
    markdown: String,
    file_path: Option<PathBuf>,
) -> WindowHandle<Shell> {
    let bounds = Bounds::centered(None, size(px(1080.), px(720.)), cx);
    let title = window_title(file_path.as_deref());
    let handle = cx
        .open_window(
            splitype_window_options(title, bounds),
            move |_window, cx| {
                let editor = cx.new(|cx| {
                    // No content and no path → welcome state with zero tabs.
                    if markdown.is_empty() && file_path.is_none() {
                        Editor::empty(cx)
                    } else {
                        Editor::from_markdown(cx, markdown, file_path)
                    }
                });
                let shell = cx.new(move |_cx| Shell {
                    // The default layout is Explorer (left) + Editor (right);
                    // only Editor areas carry content entities.
                    areas: [(DEFAULT_EDITOR_AREA_ID, AreaContent::Editor(editor))].into(),
                    retained_editor_sessions: HashMap::new(),
                    menu_bar: MenuBarState::default(),
                });
                // Wire the editor entity to its Shell.
                let shell_weak = shell.downgrade();
                let editors: Vec<Entity<Editor>> = shell
                    .read(cx)
                    .areas
                    .values()
                    .filter_map(|content| match content {
                    AreaContent::Editor(entity) => Some(entity.clone()),
                })
                    .collect();
                for editor in editors {
                    let _ = editor.update(cx, |e, _cx| e.shell = Some(shell_weak.clone()));
                }
                shell
            },
        )
        .unwrap();

    handle
        .update(cx, |shell, window, cx| {
            window.activate_window();
            if let Some(editor) = shell.primary_editor() {
                editor.update(cx, |editor, cx| {
                    editor.force_install_close_guard(cx, window);
                });
            }
        })
        .expect("newly opened shell window should be updateable");

    handle
}

/// Opens a new independent window hosting a cloned container (Shift-drag
/// default): the caller supplies the fresh tree (a single-leaf layout of
/// the dragged panel when produced by the default Shift policy), the
/// deep-copied sessions of its Editor areas, and — for an Explorer area —
/// the deep-copied file-tree state.
pub(crate) fn open_cloned_window(
    tree: SplitTree<WindowAreaKind>,
    next_node_id: usize,
    sessions: HashMap<NodeId, EditorSession>,
    explorer: Option<ExplorerState>,
    cx: &mut App,
) -> WindowHandle<Shell> {
    let bounds = Bounds::centered(None, size(px(1080.), px(720.)), cx);
    let handle = cx
        .open_window(
            splitype_window_options(SharedString::new("Splitype"), bounds),
            move |_window, cx| {
                // Materialize one Editor entity per cloned session.
                let mut areas = HashMap::new();
                let mut primary: Option<Entity<Editor>> = None;
                for (area_id, session) in sessions {
                    let editor = cx.new(|cx| Editor::with_session(area_id, session, cx));
                    if primary.is_none() {
                        primary = Some(editor.clone());
                    }
                    areas.insert(area_id, AreaContent::Editor(editor));
                }
                // The primary editor carries the window-level transition
                // state (outer layout + explorer) until it moves to the
                // Shell.
                let editor = primary.expect("cloned window has at least one editor area");
                editor.update(cx, |ed, _cx| {
                    ed.panels.layout.tree = tree;
                    ed.panels.layout.next_node_id = next_node_id;
                    if let Some(explorer) = explorer {
                        ed.panels.explorer = explorer;
                    }
                    // Activate the first Editor leaf of the cloned layout
                    // (the empty constructor seeds the default area id,
                    // which the clone may not contain).
                    let mut ids = Vec::new();
                    ed.panels.layout.tree.leaf_ids(&mut ids);
                    if let Some(id) = ids.into_iter().find(|id| {
                        ed.panels.layout.tree.find_leaf_kind(*id) == Some(WindowAreaKind::Editor)
                    }) {
                        ed.panels.layout.activate_area(id);
                    } else {
                        ed.panels.layout.active_area = None;
                        ed.panels.layout.activation_history.clear();
                    }
                });
                let shell = cx.new(move |_cx| Shell {
                    areas,
                    retained_editor_sessions: HashMap::new(),
                    menu_bar: MenuBarState::default(),
                });
                // Wire every editor entity to its Shell.
                let shell_weak = shell.downgrade();
                let editors: Vec<Entity<Editor>> = shell
                    .read(cx)
                    .areas
                    .values()
                    .filter_map(|content| match content {
                    AreaContent::Editor(entity) => Some(entity.clone()),
                })
                    .collect();
                for editor in editors {
                    let _ = editor.update(cx, |e, _cx| e.shell = Some(shell_weak.clone()));
                }
                shell
            },
        )
        .unwrap();

    handle
        .update(cx, |shell, window, cx| {
            window.activate_window();
            if let Some(editor) = shell.primary_editor() {
                editor.update(cx, |editor, cx| {
                    editor.force_install_close_guard(cx, window);
                });
            }
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
        eprintln!("failed to update recent file history: {err}");
        return;
    }
    install_menus(cx);
    cx.refresh_windows();
}
