//! Explorer rendering: the virtualized file-tree list and dispatcher.

pub(crate) mod edit_row;
pub(crate) mod empty_state;
pub(crate) mod entry_row;
pub(crate) mod root_row;

use std::ops::Range;

use gpui::*;

use crate::state::{DragExplorerTarget, DraggedExplorerSelection, ExplorerState};
use config::language::I18nStrings;
use platform_contracts::PanelId;
use theme::Theme;

/// Free-function entry point: renders the explorer body (file tree or
/// empty state) from the given panel state entity.
pub fn render_explorer_body(
    panel_id: PanelId,
    state: &Entity<ExplorerState>,
    theme: &Theme,
    strings: &I18nStrings,
    cx: &mut App,
) -> AnyElement {
    state.update(cx, |state, cx| {
        state.render_explorer_body(panel_id, theme, strings, cx)
    })
}

/// Free-function entry point: renders the explorer row context menu
/// overlay from the given panel state entity.
pub fn render_explorer_file_context_menu(
    state: &Entity<ExplorerState>,
    theme: &Theme,
    viewport: gpui::Size<gpui::Pixels>,
    cx: &mut App,
) -> Option<AnyElement> {
    state.update(cx, |state, cx| {
        state.render_explorer_file_context_menu(theme, viewport, cx)
    })
}

impl ExplorerState {
    pub(crate) fn render_explorer_files_tree(
        &mut self,
        panel_id: PanelId,
        theme: &Theme,
        strings: &I18nStrings,
        cx: &mut App,
    ) -> AnyElement {
        if self.worktrees.is_empty() {
            let recent_folders = self.recent_folders_cache.clone();
            let recent_files = self.recent_files_cache.clone();
            return self.render_explorer_empty_state(
                "Explorer is empty now",
                "",
                panel_id,
                theme,
                strings,
                &recent_folders,
                &recent_files,
                cx,
            );
        }

        if let Some(error) = self.file_error.as_ref() {
            let recent_folders = self.recent_folders_cache.clone();
            let recent_files = self.recent_files_cache.clone();
            return self.render_explorer_empty_state(
                "Explorer is empty now",
                error,
                panel_id,
                theme,
                strings,
                &recent_folders,
                &recent_files,
                cx,
            );
        }

        if self.snapshots.is_empty() {
            let recent_folders = self.recent_folders_cache.clone();
            let recent_files = self.recent_files_cache.clone();
            return self.render_explorer_empty_state(
                "Explorer is empty now",
                "",
                panel_id,
                theme,
                strings,
                &recent_folders,
                &recent_files,
                cx,
            );
        }

        let c = &theme.colors;
        let weak = self.self_weak.clone();

        // Virtualized row list: only the visible range is rendered. Rows
        // must be uniform height (EXPLORER_NODE_HEIGHT). The root row is
        // the first row; its title buttons live on the row itself (shown
        // only while the root is expanded, VSCode-style).
        let entries_len = self.entries.len();
        let scroll_handle = self.scroll_handle.clone();
        let row_theme = theme.clone();
        let list = uniform_list(("explorer-tree", panel_id.0), entries_len, {
            let weak = weak.clone();
            move |range: Range<usize>, _window, cx| {
                weak.update(cx, |state, cx| {
                    state.rendered_rows = range.len();
                    // The drag highlight extends to a directory and all of
                    // its descendants; resolve it once per frame.
                    let drag_highlight = state.explorer_drag_highlight_path();
                    let mut items = Vec::with_capacity(range.len());
                    for index in range {
                        if let Some(row) = state.entries.get(index) {
                            items.push(state.render_explorer_row(
                                row,
                                panel_id,
                                drag_highlight.as_deref(),
                                &row_theme,
                                cx,
                            ));
                        }
                    }
                    items
                })
                .unwrap_or_default()
            }
        })
        .track_scroll(&scroll_handle)
        .flex_1()
        .min_h(px(0.0))
        .py(px(4.0));

        div()
            .id(("explorer-root", panel_id.0))
            .key_context("ExplorerPanel")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .bg(
                if matches!(
                    &self.drag_target,
                    Some(DragExplorerTarget::Background)
                ) {
                    c.panel_row_hover
                } else {
                    hsla(0.0, 0.0, 0.0, 0.0)
                },
            )
            .on_action({
                let weak = weak.clone();
                move |action: &crate::ops::selection::SelectPrevious, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_select_previous(action, window, cx);
                    });
                }
            })
            .on_action({
                let weak = weak.clone();
                move |action: &crate::ops::selection::SelectNext, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_select_next(action, window, cx);
                    });
                }
            })
            .on_action({
                let weak = weak.clone();
                move |action: &crate::ops::selection::SelectParent, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_select_parent(action, window, cx);
                    });
                }
            })
            .on_action({
                let weak = weak.clone();
                move |action: &crate::ops::selection::SelectFirst, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_select_first(action, window, cx);
                    });
                }
            })
            .on_action({
                let weak = weak.clone();
                move |action: &crate::ops::selection::SelectLast, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_select_last(action, window, cx);
                    });
                }
            })
            .on_action({
                let weak = weak.clone();
                move |action: &crate::ops::selection::ExpandSelectedEntry, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_expand_selected(action, window, cx);
                    });
                }
            })
            .on_action({
                let weak = weak.clone();
                move |action: &crate::ops::selection::CollapseSelectedEntry, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_collapse_selected(action, window, cx);
                    });
                }
            })
            .on_action({
                let weak = weak.clone();
                move |action: &crate::ops::selection::ExpandSelectedEntryAndChildren, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_expand_selected_and_children(action, window, cx);
                    });
                }
            })
            .on_action({
                let weak = weak.clone();
                move |action: &crate::ops::selection::CollapseSelectedEntryAndChildren, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_collapse_selected_and_children(action, window, cx);
                    });
                }
            })
            .on_action({
                let weak = weak.clone();
                move |action: &crate::ops::selection::ExpandAllEntries, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_expand_all_entries(action, window, cx);
                    });
                }
            })
            .on_action({
                let weak = weak.clone();
                move |action: &crate::ops::selection::CollapseAllEntries, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_collapse_all_entries(action, window, cx);
                    });
                }
            })
            .on_action({
                let weak = weak.clone();
                move |action: &crate::ops::selection::OpenSelectedEntry, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_open_selected(action, window, cx);
                    });
                }
            })
            .on_action({
                let weak = weak.clone();
                move |action: &crate::ops::selection::RenameSelectedEntry, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_rename_selected(action, window, cx);
                    });
                }
            })
            .on_action({
                let weak = weak.clone();
                move |action: &crate::ops::selection::DeleteSelectedEntry, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_delete_selected(action, window, cx);
                    });
                }
            })
            .on_action({
                let weak = weak.clone();
                move |action: &crate::ops::selection::TrashSelectedEntry, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_trash_selected(action, window, cx);
                    });
                }
            })
            .on_action({
                let weak = weak.clone();
                move |action: &crate::ops::selection::NewFile, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_new_file(action, window, cx);
                    });
                }
            })
            .on_action({
                let weak = weak.clone();
                move |action: &crate::ops::selection::NewDirectory, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_new_directory(action, window, cx);
                    });
                }
            })
            .on_action({
                let weak = weak.clone();
                move |action: &crate::ops::selection::ScrollUp, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_scroll_up(action, window, cx);
                    });
                }
            })
            .on_action({
                let weak = weak.clone();
                move |action: &crate::ops::selection::ScrollDown, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_scroll_down(action, window, cx);
                    });
                }
            })
            .on_action({
                let weak = weak.clone();
                move |action: &crate::ops::selection::ScrollCursorCenter, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_scroll_cursor_center(action, window, cx);
                    });
                }
            })
            .on_action({
                let weak = weak.clone();
                move |action: &crate::ops::selection::ScrollCursorTop, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_scroll_cursor_top(action, window, cx);
                    });
                }
            })
            .on_action({
                let weak = weak.clone();
                move |action: &crate::ops::selection::ScrollCursorBottom, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_scroll_cursor_bottom(action, window, cx);
                    });
                }
            })
            // The drag cursor (move vs. copy) follows the copy modifier
            // while it is held (mirrors Zed).
            .on_modifiers_changed({
                let weak = weak.clone();
                move |event: &ModifiersChangedEvent, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.refresh_explorer_drag_cursor(&event.modifiers, window, cx);
                    });
                }
            })
            // Background click clears the selection; double-click creates a
            // new file at the root (mirrors Zed). Rows stop propagation, so
            // this only fires on the empty area.
            .on_click({
                let weak = weak.clone();
                move |event: &gpui::ClickEvent, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        if event.click_count() > 1 {
                            if let Some(root) = state.last_explorer_root_path() {
                                state.begin_explorer_create(root, false, window, cx);
                            }
                        } else {
                            state.selected = None;
                            state.marked.clear();
                        }
                    });
                }
            })
            // Right-click on blank space targets the explorer root
            // (mirrors Zed: right-clicking below the last entry is
            // equivalent to right-clicking the root directory).
            .on_mouse_down(
                MouseButton::Right,
                {
                    let weak = weak.clone();
                    move |event: &gpui::MouseDownEvent, _window, cx| {
                        let _ = weak.update(cx, |state, cx| {
                            // Right-clicking below the last entry targets the last
                            // worktree root (mirrors Zed: background right-click is
                            // equivalent to right-clicking the root directory).
                            if let Some((worktree_id, path, root_id)) = state.last_explorer_root() {
                                state.selected = Some(crate::state::SelectedEntry {
                                    worktree_id,
                                    entry_id: root_id,
                                });
                                state.open_explorer_file_context_menu(event.position, path, true, cx);
                            }
                        });
                    }
                },
            )
            // Panel-level drag handling: cursor style + edge auto-scroll +
            // out-of-bounds cleanup (mirrors Zed's handle_drag_move).
            .on_drag_move::<ExternalPaths>({
                let weak = weak.clone();
                move |event, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.explorer_drag_hover_background_external(event, window, cx);
                    });
                }
            })
            .on_drop::<ExternalPaths>({
                let weak = weak.clone();
                move |paths, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_drop_external_to_root(paths.paths(), window, cx);
                    });
                }
            })
            .on_drag_move::<DraggedExplorerSelection>({
                let weak = weak.clone();
                move |event, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.explorer_drag_hover_background_internal(event, window, cx);
                    });
                }
            })
            .on_drop::<DraggedExplorerSelection>({
                let weak = weak.clone();
                move |payload, window, cx| {
                    let _ = weak.update(cx, |state, cx| {
                        state.on_explorer_drop_internal_to_root(payload, window, cx);
                    });
                }
            })
            .child(list)
            .into_any_element()
    }

    pub(crate) fn render_explorer_body(
        &mut self,
        panel_id: PanelId,
        theme: &Theme,
        strings: &I18nStrings,
        cx: &mut App,
    ) -> AnyElement {
        if !self.worktrees.is_empty() {
            // Cheap selection follow (map lookups only). The flat row list
            // itself rebuilds exclusively on scan/expansion events — a
            // full tree rebuild every frame made the whole window lag.
            self.select_active_file_in_tree(false, cx);
        }
        self.render_explorer_files_tree(panel_id, theme, strings, cx)
    }
}
