//! Window creation and lifecycle operations.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use gpui::*;

use crate::chrome::MenuBarState;
use crate::layout::WindowPanels;
use crate::menus::install_menus;
use crate::shell::{Shell, ShellEditorHost};
use config::recent::record_recent_file;
use editor::Editor;
use splitter::NodeId;
use splitter::tree::SplitTree;
use ui::custom_titlebar::splitype_window_options;
use window::{PanelId, PanelKind, PanelView};

fn window_title(file_path: Option<&Path>) -> SharedString {
    if let Some(path) = file_path {
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
pub fn open_editor_window(
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
                let explorer_id = PanelId(1);
                let editor_id = PanelId(2);
                let editor = cx.new(|cx| {
                    let mut ed = Editor::new(markdown, file_path, cx);
                    ed.set_panel_id(editor_id);
                    ed
                });
                let explorer_view: Box<dyn PanelView> =
                    Box::new(explorer::ExplorerPanelView::new(explorer_id, cx));
                let editor_view: Box<dyn PanelView> =
                    Box::new(editor::EditorPanelView::new(editor.clone()));

                let shell = cx.new(move |_cx| Shell {
                    panel_views: [(explorer_id, explorer_view), (editor_id, editor_view)].into(),
                    retained_panel_states: HashMap::new(),
                    menu_bar: MenuBarState::default(),
                    panels: WindowPanels::default(),
                    last_viewport: None,
                    info_dialog: None,
                    unsaved_dialog: None,
                    update_check_in_progress: false,
                    close_guard_installed: false,
                    about_bg_emojis: Vec::new(),
                });
                let shell_weak = shell.downgrade();
                editor.update(cx, |e, _cx| {
                    e.host = Some(std::sync::Arc::new(ShellEditorHost::new(
                        shell_weak.clone(),
                    )));
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

/// Opens a new window hosting a cloned sub-tree handed over by a Shift-drag gesture.
pub fn open_cloned_window(
    tree: SplitTree<PanelKind>,
    next_node_id: NodeId,
    retained: HashMap<PanelId, crate::shell::RetainedPanel>,
    cx: &mut App,
) -> WindowHandle<Shell> {
    let bounds = Bounds::centered(None, size(px(1024.0), px(768.0)), cx);
    let handle = cx
        .open_window(
            splitype_window_options(SharedString::new("Splitype"), bounds),
            move |_window, cx| {
                let mut leaf_ids = Vec::new();
                tree.leaf_ids(&mut leaf_ids);
                let leaf_kinds: Vec<(NodeId, window::PanelKind)> = leaf_ids
                    .iter()
                    .filter_map(|leaf_id| tree.find_leaf_kind(*leaf_id).map(|kind| (*leaf_id, kind)))
                    .collect();
                let mut panels = WindowPanels::default();
                panels.layout.tree = tree;
                panels.layout.next_node_id = next_node_id;
                if let Some(container) = panels.layout.tree.find_first_leaf_by_kind(window::PanelKind::new("editor")) {
                    panels.layout.activate_leaf(container.id);
                } else {
                    panels.layout.active_leaf = None;
                    panels.layout.activation_history.clear();
                }
                let shell = cx.new(move |_cx| Shell {
                    panel_views: HashMap::new(),
                    retained_panel_states: HashMap::new(),
                    menu_bar: MenuBarState::default(),
                    panels,
                    last_viewport: None,
                    info_dialog: None,
                    unsaved_dialog: None,
                    update_check_in_progress: false,
                    close_guard_installed: false,
                    about_bg_emojis: Vec::new(),
                });
                let shell_weak = shell.downgrade();
                let panel_host = crate::shell::ShellPanelHost::shared(shell_weak.clone());

                let mut panel_views: HashMap<PanelId, Box<dyn PanelView>> = HashMap::new();
                for (panel_id, parked) in retained {
                    match window::PanelRegistry::restore_registered_panel(
                        parked.kind,
                        panel_id,
                        panel_host.clone(),
                        parked.state,
                        cx,
                    ) {
                        Ok(Some(view)) => {
                            panel_views.insert(panel_id, view);
                        }
                        Ok(None) => {
                            tracing::error!(kind = %parked.kind, "panel descriptor could not restore its state");
                        }
                        Err(error) => {
                            tracing::error!(kind = %parked.kind, %error, "failed to restore registered panel");
                        }
                    }
                }

                for (leaf_id, kind) in leaf_kinds {
                    let panel_id = PanelId(leaf_id);
                    if let std::collections::hash_map::Entry::Vacant(entry) =
                        panel_views.entry(panel_id)
                    {
                        match window::PanelRegistry::create_registered_panel(
                            kind,
                            panel_id,
                            panel_host.clone(),
                            cx,
                        ) {
                            Ok(Some(view)) => {
                                entry.insert(view);
                            }
                            Ok(None) => {
                                tracing::error!(%kind, "no panel descriptor is registered");
                            }
                            Err(error) => {
                                tracing::error!(%kind, %error, "failed to create registered panel");
                            }
                        }
                    }
                }

                shell.update(cx, |shell, cx| {
                    shell.panel_views = panel_views;
                    for view in shell.panel_views.values_mut() {
                        // Wire editor document hosts for restored editor views.
                        if let Some(panel) = view.as_any().downcast_ref::<editor::EditorPanelView>() {
                            let editor = panel.editor.clone();
                            editor.update(cx, |editor, cx| {
                                editor.host = Some(std::sync::Arc::new(ShellEditorHost::new(shell_weak.clone())));
                                if editor.session.has_tabs() {
                                    editor.sync_panes_with_active_tab(cx);
                                }
                            });
                        }
                    }
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

pub fn open_file_in_new_window(cx: &mut App, path: &Path) -> anyhow::Result<()> {
    let markdown = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read '{}'", path.display()))?;
    open_editor_window(cx, markdown, Some(path.to_path_buf()));
    record_recent_file_and_refresh(path, cx);
    Ok(())
}

pub fn record_recent_file_and_refresh(path: &Path, cx: &mut App) {
    if let Err(err) = record_recent_file(path) {
        tracing::warn!(path = %path.display(), error = %err, "failed to update recent file history");
        return;
    }
    install_menus(cx);
    cx.refresh_windows();
}
