//! Worktree root row rendering with actions toolbar and collapse/expand controls.

use std::path::Path;

use gpui::*;

use crate::app::shell::Shell;
use crate::explorer::drag_and_drop::DraggedExplorerEntryView;
use crate::explorer::state::state::{
    DraggedExplorerSelection, EXPLORER_NODE_HEIGHT, ExplorerEntryKind, ExplorerSelection,
    FOLDER_ICON, VisibleExplorerEntry, file_type_icon,
};
use crate::infra::theme::Theme;
use crate::ui::button::icon_chip_button;

impl Shell {
    /// Render the root row: the folder name plus the title buttons (new
    /// file / new folder / refresh / collapse all / toggle hidden). The
    /// buttons are shown only while the root is expanded (VSCode-style
    /// title row); collapsing the root hides them. Root rows are draggable
    /// to reorder worktrees (mirrors Zed).
    pub(crate) fn render_explorer_root_row(
        &self,
        entry: &VisibleExplorerEntry,
        panel_id: usize,
        drag_highlight: Option<&Path>,
        theme: &Theme,
        shell: &WeakEntity<Shell>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let t = &theme.typography;
        let selected = matches!(
            &self.panels.explorer.selected,
            Some(ExplorerSelection::Entry { entry: entry_id, .. }) if *entry_id == entry.id
        );
        let is_drag_target = drag_highlight
            .is_some_and(|highlight| entry.path == *highlight || entry.path.starts_with(highlight));
        let is_expanded = entry.is_expanded;
        let node_id = entry.id;
        let click_shell = shell.clone();
        let click_path = entry.path.clone();
        let right_click_shell = shell.clone();
        let right_click_path = entry.path.clone();
        let arrow_node_id = entry.id;
        let arrow_shell = shell.clone();
        let mark_selection = ExplorerSelection::Entry {
            root: entry.root,
            entry: entry.id,
        };
        let drag_entry_id = entry.id;
        let drag_label = entry.label.clone();
        let drag_payload = DraggedExplorerSelection {
            selections: vec![mark_selection.clone()],
        };
        // A worktree rooted at a single file renders as a file row: its own
        // icon (markdown for .md) and a click that opens the file instead of
        // toggling a folder.
        let root_is_file = entry.kind != ExplorerEntryKind::Directory;
        let root_icon = if root_is_file {
            let ext = entry
                .path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            file_type_icon(&ext)
        } else {
            FOLDER_ICON
        };
        let root_icon_color = if root_is_file {
            c.dialog_muted
        } else {
            c.text_default
        };

        let mut arrow_el = div()
            .w(px(14.0))
            .h(px(18.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center();

        if entry.has_children {
            arrow_el = arrow_el
                .cursor_pointer()
                .child(
                    svg()
                        .path(if is_expanded {
                            "icons/explorer/worktree/chevron-down.svg"
                        } else {
                            "icons/explorer/worktree/chevron-right.svg"
                        })
                        .size(px(14.0))
                        .text_color(c.dialog_muted),
                )
                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                    let _ = arrow_shell.update(cx, |shell, cx| {
                        shell.toggle_explorer_node(arrow_node_id, cx);
                    });
                    cx.stop_propagation();
                });
        }

        let shell_open = shell.clone();
        let shell_refresh = shell.clone();
        let shell_collapse = shell.clone();
        let shell_hidden = shell.clone();
        let root_index = entry.root;
        let hide_hidden =
            crate::infra::config::settings::ExplorerSettingsStore::settings(cx).hide_hidden;

        // Title buttons: visible only while the root row is expanded. The
        // set mirrors the panel toolbar: replace folder, toggle hidden
        // files, refresh, collapse all.
        let buttons = if is_expanded {
            div()
                .flex()
                .items_center()
                .gap(px(2.0))
                .child(
                    icon_chip_button(c, &theme.dimensions)
                        .id(("explorer-tb-replace", panel_id))
                        .child(
                            svg()
                                .path("icons/explorer/worktree/replace_folder.svg")
                                .size(px(14.0))
                                .text_color(c.text_default),
                        )
                        .on_click(move |_event, window, cx| {
                            let _ = shell_open.update(cx, |shell, cx| {
                                shell.replace_explorer_worktree(root_index, window, cx);
                            });
                            cx.stop_propagation();
                        }),
                )
                .child(
                    icon_chip_button(c, &theme.dimensions)
                        .id(("explorer-tb-hidden", panel_id))
                        .child(
                            svg()
                                .path(if hide_hidden {
                                    "icons/explorer/worktree/hide.svg"
                                } else {
                                    "icons/explorer/worktree/view.svg"
                                })
                                .size(px(14.0))
                                .text_color(if hide_hidden {
                                    c.text_default
                                } else {
                                    c.dialog_muted
                                }),
                        )
                        .on_click(move |_event, _window, cx| {
                            let _ = shell_hidden.update(cx, |shell, cx| {
                                shell.toggle_explorer_hidden(cx);
                            });
                            cx.stop_propagation();
                        }),
                )
                .child(
                    icon_chip_button(c, &theme.dimensions)
                        .id(("explorer-tb-refresh", panel_id))
                        .child(
                            svg()
                                .path("icons/explorer/worktree/sync_folder.svg")
                                .size(px(14.0))
                                .text_color(c.text_default),
                        )
                        .on_click(move |_event, _window, cx| {
                            let _ = shell_refresh.update(cx, |shell, cx| {
                                shell.rescan_and_sync_explorer(cx);
                            });
                            cx.stop_propagation();
                        }),
                )
                .child(
                    icon_chip_button(c, &theme.dimensions)
                        .id(("explorer-tb-collapse", panel_id))
                        .child(
                            svg()
                                .path("icons/explorer/worktree/collapse-all.svg")
                                .size(px(14.0))
                                .text_color(c.text_default),
                        )
                        .on_click(move |_event, _window, cx| {
                            let _ = shell_collapse.update(cx, |shell, cx| {
                                shell.collapse_all_explorer_nodes(cx);
                            });
                            cx.stop_propagation();
                        }),
                )
                .into_any_element()
        } else {
            div().into_any_element()
        };

        div()
            .id(ElementId::Name(
                format!("explorer-root-row-{panel_id}-{}", entry.root).into(),
            ))
            .h(px(EXPLORER_NODE_HEIGHT))
            .w_full()
            .overflow_hidden()
            .flex()
            .items_center()
            .gap(px(6.0))
            .pl(px(6.0))
            .pr(px(4.0))
            .rounded(px(theme.dimensions.tree_item_radius))
            .bg(if is_drag_target {
                c.callout_tip_bg
            } else if selected {
                c.panel_row_selected
            } else {
                hsla(0.0, 0.0, 0.0, 0.0)
            })
            .hover(|this| this.bg(c.panel_row_hover))
            .cursor_pointer()
            .child(arrow_el)
            .child(
                svg()
                    .path(root_icon)
                    .size(px(19.0))
                    .flex_shrink_0()
                    .text_color(root_icon_color),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .truncate()
                    .text_size(px(t.text_size * 0.9))
                    .font_weight(FontWeight::BOLD)
                    .line_height(px(t.text_size * t.text_line_height))
                    .text_color(if selected {
                        c.text_default
                    } else {
                        c.dialog_muted
                    })
                    .child(entry.label.clone()),
            )
            .child(buttons)
            .on_mouse_down(MouseButton::Right, {
                let right_click_selection = mark_selection.clone();
                move |event, _window, cx| {
                    let path = right_click_path.clone();
                    let selection = right_click_selection.clone();
                    let _ = right_click_shell.update(cx, |shell, cx| {
                        // Right-click selects the row (indicator feedback,
                        // mirroring Zed's deploy_context_menu); marked
                        // entries are cleared when the target is not one of
                        // them, so menu actions never surprise multi-selects.
                        shell.panels.explorer.selected = Some(selection.clone());
                        if !shell.panels.explorer.marked.contains(&selection) {
                            shell.panels.explorer.marked.clear();
                        }
                        shell.open_explorer_file_context_menu(event.position, path, true, cx);
                        cx.notify();
                    });
                    cx.stop_propagation();
                }
            })
            .on_click(move |event, window, cx| {
                let id = node_id;
                let selection = mark_selection.clone();
                let shift = event.modifiers().shift;
                let alt = event.modifiers().alt;
                let secondary = event.modifiers().secondary();
                let _ = click_shell.update(cx, |shell, cx| {
                    if shift {
                        shell.select_explorer_range(id, cx);
                        return;
                    }
                    if secondary {
                        shell.toggle_explorer_mark(selection, cx);
                        return;
                    }
                    shell.panels.explorer.marked.clear();
                    if root_is_file {
                        // A file-rooted worktree opens its file on click.
                        shell.open_explorer_file_click(click_path.clone(), false, window, cx);
                    } else if alt {
                        shell.toggle_explorer_subtree(id, cx);
                    } else {
                        shell.toggle_explorer_node(id, cx);
                    }
                });
                // Rows must not let clicks bubble to the panel background
                // (background click clears the selection).
                cx.stop_propagation();
            })
            // Drag & drop: the root row is a drop target like any directory
            // row, and dragging it reorders worktrees (see
            // on_explorer_drop_internal). Drag moves bubble up to the panel
            // background for cursor/scroll handling; drops stop propagation
            // so the background does not handle the same drop twice.
            .on_drag_move::<ExternalPaths>(cx.listener(move |shell, event, window, cx| {
                shell.explorer_drag_hover_entry_external(event, drag_entry_id, window, cx);
            }))
            .on_drop::<ExternalPaths>(cx.listener::<ExternalPaths>(
                move |shell, paths, window, cx| {
                    shell.on_explorer_drop_external(paths.paths(), drag_entry_id, window, cx);
                    cx.stop_propagation();
                },
            ))
            .on_drag_move::<DraggedExplorerSelection>(cx.listener(
                move |shell, event, window, cx| {
                    shell.explorer_drag_hover_entry_internal(event, drag_entry_id, window, cx);
                },
            ))
            .on_drop::<DraggedExplorerSelection>(cx.listener(move |shell, payload, window, cx| {
                shell.on_explorer_drop_internal(payload, drag_entry_id, window, cx);
                cx.stop_propagation();
            }))
            .on_drag(drag_payload, move |payload, click_offset, _window, cx| {
                let label = drag_label.clone();
                let count = payload.selections.len();
                cx.new(|_| DraggedExplorerEntryView {
                    label,
                    count,
                    click_offset,
                })
            })
            .into_any_element()
    }
}
