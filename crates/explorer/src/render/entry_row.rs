use std::path::{Path, PathBuf};

use gpui::*;

use crate::ExplorerState;

use crate::ops::drag_and_drop::DraggedExplorerEntryView;
use crate::state::state::{
    DraggedExplorerSelection, EXPLORER_NODE_HEIGHT, EXPLORER_NODE_INDENT, ExplorerEntryKind,
    ExplorerRow, FOLDER_ICON, MARKDOWN_ICON, VisibleExplorerEntry, file_type_icon,
};
use core_contracts::PanelId;
use theme::Theme;

impl ExplorerState {
    /// The path whose highlight extends to all of its descendants during a
    /// drag (the highlight target of the current drag, if any).
    pub(crate) fn explorer_drag_highlight_path(&self) -> Option<PathBuf> {
        self.drag_target
            .and_then(|target| target.highlight_entry_id())
            .and_then(|id| self.explorer_entry_by_id(id))
            .map(|entry| entry.path.clone())
    }

    /// Render one file-tree row for the virtualized list: either a visible
    /// entry or the inline edit row. drag_highlight is the drag target's
    /// highlight path (its descendants are highlighted too).
    pub(crate) fn render_explorer_row(
        &self,
        row: &ExplorerRow,
        panel_id: PanelId,
        drag_highlight: Option<&Path>,
        theme: &Theme,
        cx: &mut App,
    ) -> AnyElement {
        match row {
            ExplorerRow::Entry(entry) if entry.parent_id.is_none() => {
                self.render_explorer_root_row(entry, panel_id, drag_highlight, theme, cx)
            }
            ExplorerRow::Entry(entry) => {
                self.render_explorer_entry_row(entry, panel_id, drag_highlight, theme, cx)
            }
            ExplorerRow::Edit { .. } => self.render_explorer_edit_row(panel_id, theme, cx),
        }
    }

    /// Render one flat file-tree entry row.
    pub(crate) fn render_explorer_entry_row(
        &self,
        entry: &VisibleExplorerEntry,
        panel_id: PanelId,
        drag_highlight: Option<&Path>,
        theme: &Theme,
        _cx: &mut App,
    ) -> AnyElement {
        let c = &theme.colors;
        let t = &theme.typography;
        let mark_selection = crate::state::state::SelectedEntry {
            worktree_id: entry.worktree_id,
            entry_id: entry.id,
        };
        let selected = self.selected == Some(mark_selection);
        let is_marked = self.marked.contains(&mark_selection);
        let is_drag_target = drag_highlight
            .is_some_and(|highlight| entry.path == *highlight || entry.path.starts_with(highlight));
        let node_id = entry.id;
        let click_kind = entry.kind;
        let click_path = entry.path.clone();
        let click_worktree_id = entry.worktree_id;
        let right_click_path = entry.path.clone();
        let right_click_is_dir = entry.kind == ExplorerEntryKind::Directory;
        let arrow_node_id = entry.id;
        // Drag payload: the row where the drag started first, then the
        // marked entries it carries along (mirrors Zed's DraggedSelection).
        let mut drag_selections = vec![mark_selection];
        for selection in &self.marked {
            if !drag_selections.contains(selection) {
                drag_selections.push(*selection);
            }
        }
        let drag_payload = DraggedExplorerSelection {
            selections: drag_selections,
        };
        let drag_label = entry.label.clone();
        let drag_entry_id = entry.id;
        let weak = self.self_weak.clone();

        let icon = match entry.kind {
            ExplorerEntryKind::Directory => Some((FOLDER_ICON, c.text_default)),
            ExplorerEntryKind::MarkdownFile => Some((MARKDOWN_ICON, c.text_default)),
            ExplorerEntryKind::File => {
                let ext = entry
                    .path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                Some((file_type_icon(&ext), c.text_default))
            }
        };

        let label_color = if selected {
            c.text_default
        } else {
            c.dialog_muted
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
                        .path(if entry.is_expanded {
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
                format!("explorer-node-{panel_id}-{}", node_id.0).into(),
            ))
            .h(px(EXPLORER_NODE_HEIGHT))
            .w_full()
            .overflow_hidden()
            .flex()
            .items_center()
            .gap(px(6.0))
            .pl(px(6.0 + entry.depth as f32 * EXPLORER_NODE_INDENT))
            .pr(px(8.0))
            .rounded(px(theme.dimensions.tree_item_radius))
            .bg(if is_drag_target {
                c.callout_tip_bg
            } else if is_marked {
                c.callout_note_bg
            } else if selected {
                c.panel_row_selected
            } else {
                hsla(0.0, 0.0, 0.0, 0.0)
            })
            .hover(|this| this.bg(c.panel_row_hover))
            .cursor_pointer()
            .child(arrow_el)
            .children(icon.map(|(path, color)| {
                svg()
                    .path(path)
                    .size(px(19.0))
                    .flex_shrink_0()
                    .text_color(color)
                    .into_any_element()
            }))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .truncate()
                    .text_size(px(t.text_size * 0.9))
                    .line_height(px(t.text_size * t.text_line_height))
                    .text_color(label_color)
                    .child(entry.label.clone()),
            )
            .on_mouse_down(MouseButton::Right, {
                let right_click_selection = mark_selection;
                let weak = weak.clone();
                move |event, _window, cx| {
                    let path = right_click_path.clone();
                    let is_dir = right_click_is_dir;
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
                        state.open_explorer_file_context_menu(event.position, path, is_dir, cx);
                        cx.refresh_windows();
                    });
                    cx.stop_propagation();
                }
            })
            .on_click({
                let weak = weak.clone();
                move |event, window, cx| {
                    let id = node_id;
                    let kind = click_kind;
                    let path = click_path.clone();
                    let selection = mark_selection;
                    let click_count = event.click_count();
                    let shift = event.modifiers().shift;
                    let alt = event.modifiers().alt;
                    let secondary = event.modifiers().secondary();
                    let _ = weak.update(cx, |state, cx| {
                        if shift {
                            state.select_explorer_range(id, cx);
                            return;
                        }
                        if secondary {
                            if click_count > 1 {
                                // Ctrl/Cmd+double-click: open in a split area.
                                state.split_explorer_file(path, window, cx);
                            } else {
                                state.toggle_explorer_mark(selection, cx);
                            }
                            return;
                        }
                        state.marked.clear();
                        match kind {
                            ExplorerEntryKind::Directory => {
                                // Select the directory so a click always gives
                                // feedback, even when it is empty and there is
                                // nothing to expand.
                                state.selected = Some(crate::state::state::SelectedEntry {
                                    worktree_id: click_worktree_id,
                                    entry_id: id,
                                });
                                if alt {
                                    state.toggle_explorer_subtree(id, cx);
                                } else {
                                    state.toggle_explorer_node(id, cx);
                                }
                            }
                            ExplorerEntryKind::MarkdownFile | ExplorerEntryKind::File => {
                                state.open_explorer_file_click(path, click_count > 1, window, cx);
                            }
                        }
                    });
                    // Rows must not let clicks bubble to the panel background
                    // (background click clears the selection).
                    cx.stop_propagation();
                }
            })
            // Drag & drop: external files are copied; internal entries are
            // moved by default and copied with the copy modifier. Drag moves
            // bubble up to the panel background for cursor/scroll handling;
            // drops stop propagation so the background does not handle the
            // same drop twice.
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
