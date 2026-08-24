//! Explorer rendering: the virtualized file-tree list and dispatcher.

pub(crate) mod edit_row;
pub(crate) mod empty_state;
pub(crate) mod entry_row;
pub(crate) mod root_row;

use std::ops::Range;

use gpui::*;

use crate::app::shell::Shell;
use crate::explorer::state::state::{
    DragExplorerTarget, DraggedExplorerSelection, ExplorerSelection,
};
use crate::infra::i18n::I18nStrings;
use crate::infra::theme::Theme;

impl Shell {
    pub(crate) fn render_explorer_files_tree(
        &mut self,
        panel_id: usize,
        theme: &Theme,
        strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.panels.explorer.worktrees.is_empty() {
            let recent_folders = self.panels.explorer.recent_folders_cache.clone();
            let recent_files = self.panels.explorer.recent_files_cache.clone();
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

        if let Some(error) = self.panels.explorer.file_error.as_ref() {
            let recent_folders = self.panels.explorer.recent_folders_cache.clone();
            let recent_files = self.panels.explorer.recent_files_cache.clone();
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

        if self.panels.explorer.trees_cache.is_empty() {
            let recent_folders = self.panels.explorer.recent_folders_cache.clone();
            let recent_files = self.panels.explorer.recent_files_cache.clone();
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

        // Virtualized row list: only the visible range is rendered. Rows
        // must be uniform height (EXPLORER_NODE_HEIGHT). The root row is
        // the first row; its title buttons live on the row itself (shown
        // only while the root is expanded, VSCode-style).
        let entries_len = self.panels.explorer.entries.len();
        let scroll_handle = self.panels.explorer.scroll_handle.clone();
        let row_theme = theme.clone();
        let row_shell = cx.entity().downgrade();
        let list = uniform_list(
            ("explorer-tree", panel_id),
            entries_len,
            cx.processor(move |this: &mut Shell, range: Range<usize>, _window, cx| {
                this.panels.explorer.rendered_rows = range.len();
                // The drag highlight extends to a directory and all of
                // its descendants; resolve it once per frame.
                let drag_highlight = this.explorer_drag_highlight_path();
                let mut items = Vec::with_capacity(range.len());
                for index in range {
                    if let Some(row) = this.panels.explorer.entries.get(index) {
                        items.push(this.render_explorer_row(
                            row,
                            panel_id,
                            drag_highlight.as_deref(),
                            &row_theme,
                            &row_shell,
                            cx,
                        ));
                    }
                }
                items
            }),
        )
        .track_scroll(&scroll_handle)
        .flex_1()
        .min_h(px(0.0))
        .py(px(4.0));

        div()
            .id(("explorer-root", panel_id))
            .key_context("ExplorerPanel")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .bg(
                if matches!(
                    &self.panels.explorer.drag_target,
                    Some(DragExplorerTarget::Background)
                ) {
                    c.dialog_secondary_button_hover
                } else {
                    hsla(0.0, 0.0, 0.0, 0.0)
                },
            )
            .on_action(cx.listener(Self::on_explorer_select_previous))
            .on_action(cx.listener(Self::on_explorer_select_next))
            .on_action(cx.listener(Self::on_explorer_select_parent))
            .on_action(cx.listener(Self::on_explorer_select_first))
            .on_action(cx.listener(Self::on_explorer_select_last))
            .on_action(cx.listener(Self::on_explorer_scroll_up))
            .on_action(cx.listener(Self::on_explorer_scroll_down))
            .on_action(cx.listener(Self::on_explorer_scroll_cursor_center))
            .on_action(cx.listener(Self::on_explorer_scroll_cursor_top))
            .on_action(cx.listener(Self::on_explorer_scroll_cursor_bottom))
            // The drag cursor (move vs. copy) follows the copy modifier
            // while it is held (mirrors Zed).
            .on_modifiers_changed(cx.listener(
                |shell, event: &ModifiersChangedEvent, window, cx| {
                    shell.refresh_explorer_drag_cursor(&event.modifiers, window, cx);
                },
            ))
            // Background click clears the selection; double-click creates a
            // new file at the root (mirrors Zed). Rows stop propagation, so
            // this only fires on the empty area.
            .on_click(cx.listener(|shell, event: &gpui::ClickEvent, window, cx| {
                if event.click_count() > 1 {
                    if let Some(root) = shell.last_explorer_root_path() {
                        shell.begin_explorer_create(root, false, window, cx);
                    }
                } else {
                    shell.panels.explorer.selected = None;
                    shell.panels.explorer.marked.clear();
                    cx.notify();
                }
            }))
            // Right-click on blank space targets the explorer root
            // (mirrors Zed: right-clicking below the last entry is
            // equivalent to right-clicking the root directory).
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|shell, event: &gpui::MouseDownEvent, _window, cx| {
                    // Right-clicking below the last entry targets the last
                    // worktree root (mirrors Zed: background right-click is
                    // equivalent to right-clicking the root directory).
                    if let Some((root, path, root_id)) = shell.last_explorer_root() {
                        shell.panels.explorer.selected = Some(ExplorerSelection::Entry {
                            root,
                            entry: root_id,
                        });
                        shell.open_explorer_file_context_menu(event.position, path, true, cx);
                        cx.notify();
                    }
                }),
            )
            // Panel-level drag handling: cursor style + edge auto-scroll +
            // out-of-bounds cleanup (mirrors Zed's handle_drag_move).
            .on_drag_move::<ExternalPaths>(cx.listener(|shell, event, window, cx| {
                shell.explorer_drag_hover_background_external(event, window, cx);
            }))
            .on_drop::<ExternalPaths>(cx.listener::<ExternalPaths>(|shell, paths, window, cx| {
                shell.on_explorer_drop_external_to_root(paths.paths(), window, cx);
            }))
            .on_drag_move::<DraggedExplorerSelection>(cx.listener(|shell, event, window, cx| {
                shell.explorer_drag_hover_background_internal(event, window, cx);
            }))
            .on_drop::<DraggedExplorerSelection>(cx.listener(|shell, payload, window, cx| {
                shell.on_explorer_drop_internal_to_root(payload, window, cx);
            }))
            .child(list)
            .into_any_element()
    }

    pub(crate) fn render_explorer_body(
        &mut self,
        panel_id: usize,
        theme: &Theme,
        strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if !self.panels.explorer.worktrees.is_empty() {
            self.sync_explorer_models(cx);
        }
        self.render_explorer_files_tree(panel_id, theme, strings, cx)
    }
}
