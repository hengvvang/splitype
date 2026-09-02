use std::path::Path;

use gpui::*;

use crate::state::ExplorerState;

use crate::ops::drag_and_drop::DraggedExplorerEntryView;
use crate::state::{
    DraggedExplorerSelection, EXPLORER_NODE_HEIGHT, ExplorerEntryKind, FOLDER_ICON,
    VisibleExplorerEntry, file_type_icon,
};
use platform_contracts::PanelId;
use theme::Theme;

impl ExplorerState {
    /// Render the root row: the folder name and chevron. Root rows are draggable
    /// to reorder worktrees (mirrors Zed).
    pub(crate) fn render_explorer_root_row(
        &self,
        entry: &VisibleExplorerEntry,
        panel_id: PanelId,
        drag_highlight: Option<&Path>,
        theme: &Theme,
        _cx: &mut App,
    ) -> AnyElement {
        let c = &theme.colors;
        let t = &theme.typography;
        let mark_selection = crate::state::SelectedEntry {
            worktree_id: entry.worktree_id,
            entry_id: entry.id,
        };
        let selected = self.selected == Some(mark_selection);
        let is_drag_target = drag_highlight
            .is_some_and(|highlight| entry.path == *highlight || entry.path.starts_with(highlight));
        let is_expanded = entry.is_expanded;
        let node_id = entry.id;
        let click_path = entry.path.clone();
        let right_click_path = entry.path.clone();
        let arrow_node_id = entry.id;
        let drag_entry_id = entry.id;
        let drag_label = entry.label.clone();
        let drag_payload = DraggedExplorerSelection {
            selections: vec![mark_selection],
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
        let weak = self.self_weak.clone();

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
                .on_mouse_down(MouseButton::Left, {
                    let weak = weak.clone();
                    move |_event, _window, cx| {
                        let _ = weak.update(cx, |state, cx| {
                            state.toggle_explorer_node(arrow_node_id, cx);
                        });
                        cx.stop_propagation();
                    }
                });
        }

        div()
            .id(ElementId::Name(
                format!("explorer-root-row-{panel_id}-{}", entry.worktree_id.0).into(),
            ))
            .relative()
            .h(px(EXPLORER_NODE_HEIGHT))
            .w_full()
            .overflow_hidden()
            .flex()
            .items_center()
            .gap(px(6.0))
            .pl(px(6.0))
            .pr(px(4.0))
            .bg(if is_drag_target {
                c.callout_tip_bg
            } else if selected {
                c.panel_row_hover
            } else {
                hsla(0.0, 0.0, 0.0, 0.0)
            })
            .hover(|this| this.bg(c.panel_row_hover))
            .cursor_pointer()
            .children(if selected {
                Some(
                    div()
                        .absolute()
                        .left_0()
                        .top(px(4.0))
                        .bottom(px(4.0))
                        .w(px(3.0))
                        .rounded_r(px(2.0))
                        .bg(c.focus_accent),
                )
            } else {
                None
            })
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
            .on_mouse_down(MouseButton::Right, {
                let right_click_selection = mark_selection;
                let weak = weak.clone();
                move |event, _window, cx| {
                    let path = right_click_path.clone();
                    let selection = right_click_selection;
                    let _ = weak.update(cx, |state, cx| {
                        // Right-click selects the row (indicator feedback,
                        // mirroring Zed's deploy_context_menu); marked
                        // entries are cleared when the target is not one of
                        // them, so menu actions never surprise multi-selects.
                        state.selected = Some(selection);
                        if !state.marked.contains(&selection) {
                            state.marked.clear();
                        }
                        state.open_explorer_file_context_menu(event.position, path, true, cx);
                        cx.refresh_windows();
                    });
                    cx.stop_propagation();
                }
            })
            .on_click({
                let weak = weak.clone();
                move |event, window, cx| {
                    let id = node_id;
                    let selection = mark_selection;
                    let shift = event.modifiers().shift;
                    let alt = event.modifiers().alt;
                    let secondary = event.modifiers().secondary();
                    let click_count = event.click_count();
                    let _ = weak.update(cx, |state, cx| {
                        if shift {
                            state.select_explorer_range(id, cx);
                            return;
                        }
                        if secondary {
                            state.toggle_explorer_mark(selection, cx);
                            return;
                        }
                        state.marked.clear();
                        if root_is_file {
                            // A file-rooted worktree opens its file on click.
                            state.open_explorer_file_click(
                                click_path.clone(),
                                click_count > 1,
                                window,
                                cx,
                            );
                        } else if alt {
                            state.toggle_explorer_subtree(id, cx);
                        } else {
                            state.toggle_explorer_node(id, cx);
                        }
                    });
                    // Rows must not let clicks bubble to the panel background
                    // (background click clears the selection).
                    cx.stop_propagation();
                }
            })
            // Drag & drop: the root row is a drop target like any directory
            // row, and dragging it reorders worktrees (see
            // on_explorer_drop_internal). Drag moves bubble up to the panel
            // background for cursor/scroll handling; drops stop propagation
            // so the background does not handle the same drop twice.
            .on_drag_move::<ExternalPaths>({
                let weak = weak.clone();
                move |event, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.explorer_drag_hover_entry_external(event, drag_entry_id, window, cx);
                    });
                }
            })
            .on_drop::<ExternalPaths>({
                let weak = weak.clone();
                move |paths, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_drop_external(paths.paths(), drag_entry_id, window, cx);
                    });
                    cx.stop_propagation();
                }
            })
            .on_drag_move::<DraggedExplorerSelection>({
                let weak = weak.clone();
                move |event, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.explorer_drag_hover_entry_internal(event, drag_entry_id, window, cx);
                    });
                }
            })
            .on_drop::<DraggedExplorerSelection>({
                let weak = weak.clone();
                move |payload, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_drop_internal(payload, drag_entry_id, window, cx);
                    });
                    cx.stop_propagation();
                }
            })
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
