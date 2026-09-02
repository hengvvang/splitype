//! Window creation and lifecycle operations.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use gpui::*;

use crate::chrome::MenuBarState;
use crate::layout::WindowPanels;
use crate::menus::install_menus;
use crate::shell::Shell;
use config::recent::record_recent_file;
use platform_contracts::{PanelId, PanelKind};
use splitter::NodeId;
use splitter::tree::SplitTree;
use ui::custom_titlebar::splitype_window_options;

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
///
/// The window layout and every panel are built through the panel registry
/// and capability declarations; no concrete plugin types are referenced.
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
                let panels = WindowPanels::default();
                let mut leaf_ids = Vec::new();
                panels.layout.tree.leaf_ids(&mut leaf_ids);
                let leaf_kinds: Vec<(NodeId, PanelKind)> = leaf_ids
                    .iter()
                    .filter_map(|leaf_id| {
                        panels
                            .layout
                            .tree
                            .find_leaf_kind(*leaf_id)
                            .map(|kind| (*leaf_id, kind))
                    })
                    .collect();

                let shell = cx.new(move |cx| Shell {
                    panel_views: HashMap::new(),
                    retained_panel_states: HashMap::new(),
                    menu_bar: MenuBarState::default(),
                    panels,
                    has_rendered: false,
                    info_dialog: None,
                    focus_handle: cx.focus_handle(),
                    unsaved_dialog: None,
                    update_check_in_progress: false,
                    close_guard_installed: false,
                    about_bg_emojis: Vec::new(),
                });
                shell.update(cx, |shell, cx| {
                    for (leaf_id, kind) in leaf_kinds {
                        shell.ensure_registered_panel_view(PanelId(leaf_id), kind, cx);
                    }
                    shell.load_initial_document(markdown, file_path, cx);
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
    let layout = window::WindowLayout {
        tree,
        next_node_id,
        active_splitter_drag: None,
        active_border_menu: None,
        active_leaf: None,
        activation_history: Vec::new(),
    };
    open_window_with_retained(cx, layout, retained)
}

/// Opens a window restoring the persisted layout topology and panel states.
pub fn open_restored_window(
    cx: &mut App,
    mut state: window::PersistedWindowState,
) -> WindowHandle<Shell> {
    let mut retained = HashMap::new();
    let panels = std::mem::take(&mut state.panels);
    for panel in panels {
        let Ok(Some(descriptor)) = window::PanelRegistry::registered(panel.kind.clone()) else {
            tracing::warn!(
                kind = %panel.kind,
                "skipping persisted panel without a registered descriptor"
            );
            continue;
        };
        let Some(blob) = descriptor.deserialize_state(&panel.state) else {
            tracing::warn!(
                kind = %panel.kind,
                "panel descriptor could not deserialize its persisted state"
            );
            continue;
        };
        retained.insert(
            panel.id,
            crate::shell::RetainedPanel {
                kind: panel.kind,
                state: blob,
            },
        );
    }
    let layout = state.into_layout();
    open_window_with_retained(cx, layout, retained)
}

/// Opens a window from an existing layout root, materializing every panel
/// through the registry and restoring retained panel states.
fn open_window_with_retained(
    cx: &mut App,
    layout: window::WindowLayout,
    retained: HashMap<PanelId, crate::shell::RetainedPanel>,
) -> WindowHandle<Shell> {
    let bounds = Bounds::centered(None, size(px(1024.0), px(768.0)), cx);
    let handle = cx
        .open_window(
            splitype_window_options(SharedString::new("Splitype"), bounds),
            move |_window, cx| {
                let mut leaf_ids = Vec::new();
                layout.tree.leaf_ids(&mut leaf_ids);
                let leaf_kinds: Vec<(NodeId, platform_contracts::PanelKind)> = leaf_ids
                    .iter()
                    .filter_map(|leaf_id| {
                        layout
                            .tree
                            .find_leaf_kind(*leaf_id)
                            .map(|kind| (*leaf_id, kind))
                    })
                    .collect();
                let mut panels = WindowPanels { layout };
                if panels.layout.active_leaf.is_none() {
                    if let Some((leaf_id, _)) = leaf_kinds
                        .iter()
                        .find(|(_, kind)| Shell::kind_is_document_panel(kind))
                    {
                        panels.layout.activate_leaf(*leaf_id);
                    } else {
                        panels.layout.activation_history.clear();
                    }
                }
                let shell = cx.new(move |cx| Shell {
                    panel_views: HashMap::new(),
                    retained_panel_states: HashMap::new(),
                    menu_bar: MenuBarState::default(),
                    panels,
                    has_rendered: false,
                    info_dialog: None,
                    focus_handle: cx.focus_handle(),
                    unsaved_dialog: None,
                    update_check_in_progress: false,
                    close_guard_installed: false,
                    about_bg_emojis: Vec::new(),
                });

                shell.update(cx, |shell, cx| {
                    for (panel_id, parked) in retained {
                        shell.restore_retained_view(panel_id, parked.kind, parked.state, cx);
                    }
                    for (leaf_id, kind) in leaf_kinds {
                        shell.ensure_registered_panel_view(PanelId(leaf_id), kind, cx);
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
