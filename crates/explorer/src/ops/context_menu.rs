//! Explorer row context menu — a window-level overlay owned by the Shell.
//!
//! The menu is triggered from the explorer sidebar but must float over the
//! whole window (its position arrives in window coordinates), so the Shell
//! renders it at the window root instead of inside an editor tile. State
//! lives on `Shell::explorer_file_menu`; every action operates on the
//! explorer model directly.

use gpui::*;

use crate::state::state::{ExplorerFileMenuState, ExplorerState};

use i18n::I18nManager;
use theme::Theme;
use ui::menu_item::{menu_item, menu_item_row};
use ui::popover::overlay;

impl ExplorerState {
    /// Open the row context menu at `position` (window coordinates); the
    /// shell renders the overlay at the window root.
    pub(crate) fn open_explorer_file_context_menu(
        &mut self,
        position: Point<Pixels>,
        path: std::path::PathBuf,
        is_dir: bool,
        cx: &mut App,
    ) {
        self.file_menu = Some(ExplorerFileMenuState {
            position,
            path,
            is_dir,
        });
        cx.refresh_windows();
    }

    /// Close the row context menu, if open.
    pub(crate) fn close_explorer_file_menu(&mut self, cx: &mut App) {
        self.file_menu = None;
        cx.refresh_windows();
    }

    pub(crate) fn on_dismiss_explorer_file_menu(
        &mut self,
        _event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut App,
    ) {
        self.close_explorer_file_menu(cx);
    }

    /// Renders the explorer row context menu at its stored window position.
    pub(crate) fn render_explorer_file_context_menu(
        &self,
        theme: &Theme,
        viewport: Size<Pixels>,
        cx: &App,
    ) -> Option<AnyElement> {
        let state = self.file_menu.as_ref()?;
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let s = cx.global::<I18nManager>().strings().clone();

        let (panel_x, panel_y) = if viewport.width > px(0.0) {
            let max_x = (viewport.width - px(260.0)).max(px(0.0));
            let max_y = (viewport.height - px(400.0)).max(px(0.0));
            (state.position.x.min(max_x), state.position.y.min(max_y))
        } else {
            (state.position.x, state.position.y)
        };
        let path = state.path.clone();
        let is_dir = state.is_dir;

        let is_root = self
            .worktrees
            .iter()
            .any(|wt| wt.read(cx).root() == path.as_path());
        let can_undo = self.undo_history.can_undo();
        let can_redo = self.undo_history.can_redo();
        let has_pasteable = self.clipboard.is_some();
        let entry_id = self.explorer_id_for_path(&path);

        type ContextMenuItemHandler =
            Box<dyn Fn(&mut ExplorerState, &mut Window, &mut App) + 'static>;

        // Build a menu row: label only (no icons, no keybindings — matching
        // Zed's context menu), optionally disabled, with a danger variant
        // for destructive actions.
        let make_item = |id: &'static str,
                         label: String,
                         color: Hsla,
                         enabled: bool,
                         handler: ContextMenuItemHandler|
         -> AnyElement {
            if enabled {
                menu_item(id, c, d)
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(t.text_size * 0.8))
                            .text_color(color)
                            .child(label),
                    )
                    .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                        let handler = &handler;
                        ExplorerState::update(cx, |state, cx| {
                            handler(state, window, cx);
                        });
                        cx.stop_propagation();
                    })
                    .into_any_element()
            } else {
                menu_item_row(c, d)
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(t.text_size * 0.8))
                            .text_color(c.dialog_muted)
                            .child(label),
                    )
                    .into_any_element()
            }
        };
        let separator = || {
            div()
                .mx(px(d.menu_separator_margin_x))
                .my(px(d.menu_separator_margin_y))
                .h(px(d.menu_separator_height))
                .bg(c.dialog_border)
                .into_any_element()
        };

        let mut items = Vec::new();

        // New File / New Folder (directories create inside; files create in parent directory).
        {
            let target_parent = if is_dir {
                path.clone()
            } else {
                path.parent().map(std::path::Path::to_path_buf).unwrap_or_else(|| path.clone())
            };
            let p = target_parent.clone();
            items.push(make_item(
                "explorer-ctx-new-file",
                s.explorer_new_file.clone(),
                c.text_default,
                true,
                Box::new(move |state, window, cx| {
                    let p = p.clone();
                    state.close_explorer_file_menu(cx);
                    state.begin_inline_create_file(p, window, cx);
                }),
            ));
            let p = target_parent;
            items.push(make_item(
                "explorer-ctx-new-folder",
                s.explorer_new_folder.clone(),
                c.text_default,
                true,
                Box::new(move |state, window, cx| {
                    let p = p.clone();
                    state.close_explorer_file_menu(cx);
                    state.begin_inline_create_folder(p, window, cx);
                }),
            ));
            items.push(separator());
        }

        // Open in Split (files only).
        if !is_dir {
            let p = path.clone();
            items.push(make_item(
                "explorer-ctx-open-split",
                s.explorer_open_in_split.clone(),
                c.text_default,
                true,
                Box::new(move |state, window, cx| {
                    let p = p.clone();
                    state.close_explorer_file_menu(cx);
                    state.split_explorer_file(p, window, cx);
                }),
            ));
        }

        // Reveal / Open with system / Open in terminal.
        let p = path.clone();
        items.push(make_item(
            "explorer-ctx-reveal",
            s.explorer_reveal_in_file_manager.clone(),
            c.text_default,
            true,
            Box::new(move |state, _window, cx| {
                let p = p.clone();
                state.close_explorer_file_menu(cx);
                state.reveal_in_file_explorer(&p);
            }),
        ));
        let p = path.clone();
        items.push(make_item(
            "explorer-ctx-open-default",
            s.explorer_open_in_default_app.clone(),
            c.text_default,
            true,
            Box::new(move |state, _window, cx| {
                let p = p.clone();
                state.close_explorer_file_menu(cx);
                state.open_explorer_with_system(&p);
            }),
        ));
        let p = path.clone();
        items.push(make_item(
            "explorer-ctx-open-terminal",
            s.explorer_open_in_terminal.clone(),
            c.text_default,
            true,
            Box::new(move |state, _window, cx| {
                let p = p.clone();
                state.close_explorer_file_menu(cx);
                state.open_in_terminal(&p);
            }),
        ));
        items.push(separator());

        // Cut / Copy / Duplicate / Paste (paste disabled without clipboard
        // content), then Undo / Redo (disabled when the corresponding stack
        // is empty) — matching Zed's order.
        items.push(make_item(
            "explorer-ctx-cut",
            s.explorer_cut.clone(),
            c.text_default,
            true,
            Box::new(|state, _window, cx| {
                state.close_explorer_file_menu(cx);
                state.explorer_cut(cx);
            }),
        ));
        items.push(make_item(
            "explorer-ctx-copy",
            s.explorer_copy.clone(),
            c.text_default,
            true,
            Box::new(|state, _window, cx| {
                state.close_explorer_file_menu(cx);
                state.explorer_copy(cx);
            }),
        ));
        items.push(make_item(
            "explorer-ctx-duplicate",
            s.explorer_duplicate.clone(),
            c.text_default,
            true,
            Box::new(|state, window, cx| {
                state.close_explorer_file_menu(cx);
                state.explorer_duplicate(window, cx);
            }),
        ));
        items.push(make_item(
            "explorer-ctx-paste",
            s.explorer_paste.clone(),
            c.text_default,
            has_pasteable,
            Box::new(|state, window, cx| {
                state.close_explorer_file_menu(cx);
                state.explorer_paste(window, cx);
            }),
        ));
        items.push(make_item(
            "explorer-ctx-undo",
            s.explorer_undo.clone(),
            c.text_default,
            can_undo,
            Box::new(|state, window, cx| {
                state.close_explorer_file_menu(cx);
                state.explorer_undo(window, cx);
            }),
        ));
        items.push(make_item(
            "explorer-ctx-redo",
            s.explorer_redo.clone(),
            c.text_default,
            can_redo,
            Box::new(|state, window, cx| {
                state.close_explorer_file_menu(cx);
                state.explorer_redo(window, cx);
            }),
        ));
        items.push(separator());

        // Copy Path / Copy Relative Path.
        let p = path.clone();
        items.push(make_item(
            "explorer-ctx-copy-path",
            s.explorer_copy_path.clone(),
            c.text_default,
            true,
            Box::new(move |state, _window, cx| {
                let p = p.clone();
                state.close_explorer_file_menu(cx);
                state.copy_path_to_clipboard(&p, cx);
            }),
        ));
        let p = path.clone();
        items.push(make_item(
            "explorer-ctx-copy-relative-path",
            s.explorer_copy_relative_path.clone(),
            c.text_default,
            true,
            Box::new(move |state, _window, cx| {
                let p = p.clone();
                state.close_explorer_file_menu(cx);
                state.copy_explorer_relative_path(&p, cx);
            }),
        ));

        // Rename (hidden for the root, mirroring Zed) and Delete.
        if !is_root {
            items.push(separator());
            let p = path.clone();
            items.push(make_item(
                "explorer-ctx-rename",
                s.explorer_rename.clone(),
                c.text_default,
                true,
                Box::new(move |state, window, cx| {
                    let p = p.clone();
                    state.close_explorer_file_menu(cx);
                    state.begin_inline_rename(p, window, cx);
                }),
            ));
            items.push(make_item(
                "explorer-ctx-trash",
                s.explorer_trash.clone(),
                c.text_default,
                true,
                Box::new(|state, window, cx| {
                    state.close_explorer_file_menu(cx);
                    state.trash_explorer_selections(window, cx);
                }),
            ));
            items.push(make_item(
                "explorer-ctx-delete",
                s.explorer_delete.clone(),
                c.dialog_danger_button_bg,
                true,
                Box::new(|state, window, cx| {
                    state.close_explorer_file_menu(cx);
                    state.delete_explorer_selections(window, cx);
                }),
            ));
        } else {
            // Worktree roots: add another folder to the explorer or remove
            // this one (mirrors Zed's "Add Folders to Project…" / "Remove
            // from Project").
            items.push(separator());
            items.push(make_item(
                "explorer-ctx-add-folder",
                s.explorer_add_folder.clone(),
                c.text_default,
                true,
                Box::new(|state, window, cx| {
                    state.close_explorer_file_menu(cx);
                    state.prompt_open_explorer_folder(window, cx);
                }),
            ));
            let p = path.clone();
            items.push(make_item(
                "explorer-ctx-remove-folder",
                s.explorer_remove_folder.clone(),
                c.text_default,
                true,
                Box::new(move |state, _window, cx| {
                    let p = p.clone();
                    state.close_explorer_file_menu(cx);
                    if let Some(index) = state
                        .worktrees
                        .iter()
                        .position(|worktree| worktree.read(cx).root() == p.as_path())
                    {
                        state.remove_explorer_worktree(index, cx);
                    }
                }),
            ));
        }

        // Expand All / Collapse All (directories; root uses the whole-tree
        // variants, mirroring Zed).
        if is_dir {
            items.push(separator());
            items.push(make_item(
                "explorer-ctx-expand-all",
                s.explorer_expand_all.clone(),
                c.text_default,
                true,
                Box::new(move |state, _window, cx| {
                    state.close_explorer_file_menu(cx);
                    match entry_id {
                        Some(sel) if !is_root => {
                            state.expand_all_explorer_for_entry(sel.entry_id, cx);
                        }
                        _ => state.expand_all_explorer_nodes(cx),
                    }
                }),
            ));
            items.push(make_item(
                "explorer-ctx-collapse-all",
                s.explorer_collapse_all.clone(),
                c.text_default,
                true,
                Box::new(move |state, _window, cx| {
                    state.close_explorer_file_menu(cx);
                    match entry_id {
                        Some(sel) if !is_root => {
                            state.collapse_all_explorer_for_entry(sel.entry_id, cx);
                        }
                        _ => state.collapse_all_explorer_nodes(cx),
                    }
                }),
            ));
        }

        Some(
            overlay()
                .id("explorer-file-context-menu-overlay")
                .occlude()
                .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                    ExplorerState::update(cx, |state, cx| {
                        state.on_dismiss_explorer_file_menu(event, window, cx);
                    });
                })
                .child(
                    div()
                        .id("explorer-file-context-menu-panel")
                        .absolute()
                        .left(panel_x)
                        .top(panel_y)
                        .w(px(250.0))
                        .p(px(d.menu_panel_padding))
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .bg(c.dialog_surface)
                        .border(px(d.dialog_border_width))
                        .border_color(c.dialog_border)
                        .rounded(px(d.menu_panel_radius))
                        .shadow_lg()
                        .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                            cx.stop_propagation()
                        })
                        .children(items),
                )
                .into_any_element(),
        )
    }
}
