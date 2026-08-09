//! Rendering of the editor's context menus and the table insert dialog:
//! the axis menu items, the overlay panel, and the insert-size dialog.

use crate::ui::button::{primary_button, secondary_button};
use crate::ui::dialog::dialog_card;
use crate::ui::menu_item::{menu_item, menu_item_row};
use crate::ui::popover::menu_panel;
use crate::ui::popover::overlay;

use gpui::*;

use crate::editor::controller::Editor;
use crate::editor::window::context_menu::ContextMenuState;
use crate::infra::i18n::I18nManager;
use crate::infra::theme::Theme;
use crate::model::syntax::table::TableAxisKind;
impl Editor {
    pub(crate) fn render_axis_menu_item(
        theme: &Theme,
        id: &'static str,
        label: String,
        enabled: bool,
        danger: bool,
        on_click: fn(&mut Editor, &ClickEvent, &mut Window, &mut Context<Editor>),
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        if enabled {
            menu_item(id, c, d)
                .text_size(px(d.menu_text_size))
                .font_weight(t.dialog_body_weight.to_font_weight())
                .text_color(if danger {
                    c.dialog_danger_button_bg
                } else {
                    c.dialog_secondary_button_text
                })
                .child(label)
                .on_click(cx.listener(on_click))
                .into_any_element()
        } else {
            div()
                .id(id)
                .h(px(d.menu_item_height))
                .px(px(d.menu_item_padding_x))
                .flex()
                .items_center()
                .rounded(px(d.menu_item_radius))
                .bg(c.dialog_surface)
                .text_size(px(d.menu_text_size))
                .font_weight(t.dialog_body_weight.to_font_weight())
                .text_color(if danger {
                    c.dialog_danger_button_bg
                } else {
                    c.dialog_muted
                })
                .child(label)
                .into_any_element()
        }
    }

    pub(crate) fn render_context_menu_overlay(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let menu = self.context_menu.as_ref()?;
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let s = cx.global::<I18nManager>().strings().clone();

        match menu {
            ContextMenuState::Insert {
                position,
                submenu_open,
                ..
            } => {
                let panel_x = position.x;
                let panel_y = position.y;
                let panel_width = px(d.context_menu_panel_width);

                let submenu = submenu_open.then(|| {
                    menu_panel(c, d)
                        .id("editor-context-menu-submenu")
                        .absolute()
                        .left(panel_x + panel_width + px(d.context_menu_submenu_gap))
                        .top(panel_y)
                        .w(px(d.context_menu_submenu_width))
                        .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                            cx.stop_propagation()
                        })
                        .on_hover(cx.listener(Self::on_context_menu_submenu_hover))
                        .child(
                            menu_item("editor-context-menu-insert-table", c, d)
                                .text_size(px(d.menu_text_size))
                                .font_weight(t.dialog_body_weight.to_font_weight())
                                .text_color(c.dialog_secondary_button_text)
                                .child(s.context_menu_table.clone())
                                .on_click(cx.listener(Self::on_open_table_insert_dialog)),
                        )
                });

                let overlay = overlay()
                    .id("editor-context-menu-overlay")
                    .occlude()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(Self::on_dismiss_context_menu_overlay),
                    )
                    .child(
                        menu_panel(c, d)
                            .id("editor-context-menu-panel")
                            .absolute()
                            .left(panel_x)
                            .top(panel_y)
                            .w(panel_width)
                            .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                                cx.stop_propagation()
                            })
                            .child(
                                menu_item("editor-context-menu-insert", c, d)
                                    .justify_between()
                                    .bg(if *submenu_open {
                                        c.panel_row_selected
                                    } else {
                                        c.dialog_surface
                                    })
                                    .text_size(px(d.menu_text_size))
                                    .font_weight(t.dialog_body_weight.to_font_weight())
                                    .text_color(c.dialog_secondary_button_text)
                                    .child(s.context_menu_insert.clone())
                                    .child(
                                        svg()
                                            .path("icons/editor/context_menu/chevron-right.svg")
                                            .size(px(14.0))
                                            .text_color(c.dialog_secondary_button_text),
                                    )
                                    .on_hover(cx.listener(Self::on_context_menu_insert_hover)),
                            ),
                    );

                Some(if let Some(submenu) = submenu {
                    overlay.child(submenu).into_any_element()
                } else {
                    overlay.into_any_element()
                })
            }
            ContextMenuState::TableAxis {
                position,
                selection,
            } => {
                let Some(table_block) = self.table_block_by_id(selection.table_block_id, cx) else {
                    return None;
                };
                let table = table_block.read(cx).record.table.clone()?;
                let items = match selection.kind {
                    TableAxisKind::Column => vec![
                        Self::render_axis_menu_item(
                            theme,
                            "table-axis-insert-column-left",
                            "Insert Column Left".to_string(),
                            true,
                            false,
                            Self::on_insert_table_column_left,
                            cx,
                        ),
                        Self::render_axis_menu_item(
                            theme,
                            "table-axis-insert-column-right",
                            "Insert Column Right".to_string(),
                            true,
                            false,
                            Self::on_insert_table_column_right,
                            cx,
                        ),
                        Self::render_axis_menu_item(
                            theme,
                            "table-axis-duplicate-column",
                            "Duplicate Column".to_string(),
                            true,
                            false,
                            Self::on_duplicate_table_column,
                            cx,
                        ),
                        div()
                            .mx(px(d.menu_separator_margin_x))
                            .my(px(d.menu_separator_margin_y))
                            .h(px(d.menu_separator_height))
                            .bg(c.dialog_border)
                            .into_any_element(),
                        Self::render_axis_menu_item(
                            theme,
                            "table-axis-align-column-left",
                            s.table_axis_align_column_left.clone(),
                            true,
                            false,
                            Self::on_align_table_column_left,
                            cx,
                        ),
                        Self::render_axis_menu_item(
                            theme,
                            "table-axis-align-column-center",
                            s.table_axis_align_column_center.clone(),
                            true,
                            false,
                            Self::on_align_table_column_center,
                            cx,
                        ),
                        Self::render_axis_menu_item(
                            theme,
                            "table-axis-align-column-right",
                            s.table_axis_align_column_right.clone(),
                            true,
                            false,
                            Self::on_align_table_column_right,
                            cx,
                        ),
                        div()
                            .mx(px(d.menu_separator_margin_x))
                            .my(px(d.menu_separator_margin_y))
                            .h(px(d.menu_separator_height))
                            .bg(c.dialog_border)
                            .into_any_element(),
                        Self::render_axis_menu_item(
                            theme,
                            "table-axis-move-column-left",
                            s.table_axis_move_column_left.clone(),
                            selection.index > 0,
                            false,
                            Self::on_move_table_column_left,
                            cx,
                        ),
                        Self::render_axis_menu_item(
                            theme,
                            "table-axis-move-column-right",
                            s.table_axis_move_column_right.clone(),
                            selection.index + 1 < table.column_count(),
                            false,
                            Self::on_move_table_column_right,
                            cx,
                        ),
                        div()
                            .mx(px(d.menu_separator_margin_x))
                            .my(px(d.menu_separator_margin_y))
                            .h(px(d.menu_separator_height))
                            .bg(c.dialog_border)
                            .into_any_element(),
                        Self::render_axis_menu_item(
                            theme,
                            "table-axis-delete-column",
                            s.table_axis_delete_column.clone(),
                            // Always enabled: deleting the last column removes the
                            // whole table.
                            true,
                            true,
                            Self::on_delete_table_column,
                            cx,
                        ),
                    ],
                    TableAxisKind::Row => {
                        let mut items: Vec<AnyElement> = Vec::new();
                        items.push(Self::render_axis_menu_item(
                            theme,
                            "table-axis-insert-row-above",
                            "Insert Row Above".to_string(),
                            true,
                            false,
                            Self::on_insert_table_row_above,
                            cx,
                        ));
                        items.push(Self::render_axis_menu_item(
                            theme,
                            "table-axis-insert-row-below",
                            "Insert Row Below".to_string(),
                            true,
                            false,
                            Self::on_insert_table_row_below,
                            cx,
                        ));
                        items.push(Self::render_axis_menu_item(
                            theme,
                            "table-axis-duplicate-row",
                            "Duplicate Row".to_string(),
                            true,
                            false,
                            Self::on_duplicate_table_row,
                            cx,
                        ));
                        items.push(
                            div()
                                .mx(px(d.menu_separator_margin_x))
                                .my(px(d.menu_separator_margin_y))
                                .h(px(d.menu_separator_height))
                                .bg(c.dialog_border)
                                .into_any_element(),
                        );
                        // The header row (visual index 0) shares the normal row
                        // menu, with its Header Row styling toggle added on top.
                        if selection.index == 0 {
                            let headers_shown =
                                crate::infra::config::settings::EditorSettings::show_table_headers(
                                    cx,
                                );
                            items.push(
                                menu_item("table-header-toggle", c, d)
                                    .justify_between()
                                    .gap(px(d.menu_item_padding_x))
                                    .text_size(px(d.menu_text_size))
                                    .font_weight(t.dialog_body_weight.to_font_weight())
                                    .text_color(c.dialog_secondary_button_text)
                                    .child(s.table_header_row.clone())
                                    .child(if headers_shown { "✓" } else { "" })
                                    .on_click(cx.listener(Self::on_toggle_table_headers))
                                    .into_any_element(),
                            );
                            items.push(
                                div()
                                    .mx(px(d.menu_separator_margin_x))
                                    .my(px(d.menu_separator_margin_y))
                                    .h(px(d.menu_separator_height))
                                    .bg(c.dialog_border)
                                    .into_any_element(),
                            );
                        }
                        items.push(Self::render_axis_menu_item(
                            theme,
                            "table-axis-move-row-up",
                            s.table_axis_move_row_up.clone(),
                            selection.index > 0,
                            false,
                            Self::on_move_table_row_up,
                            cx,
                        ));
                        items.push(Self::render_axis_menu_item(
                            theme,
                            "table-axis-move-row-down",
                            s.table_axis_move_row_down.clone(),
                            selection.index < table.rows.len(),
                            false,
                            Self::on_move_table_row_down,
                            cx,
                        ));
                        items.push(
                            div()
                                .mx(px(d.menu_separator_margin_x))
                                .my(px(d.menu_separator_margin_y))
                                .h(px(d.menu_separator_height))
                                .bg(c.dialog_border)
                                .into_any_element(),
                        );
                        // Always enabled: deleting the header promotes the first
                        // body row, and deleting the last remaining row removes
                        // the whole table.
                        items.push(Self::render_axis_menu_item(
                            theme,
                            "table-axis-delete-row",
                            s.table_axis_delete_row.clone(),
                            true,
                            true,
                            Self::on_delete_table_row,
                            cx,
                        ));
                        items
                    }
                };

                Some(
                    overlay()
                        .id("table-axis-context-menu-overlay")
                        .occlude()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(Self::on_dismiss_context_menu_overlay),
                        )
                        .child(
                            div()
                                .id("table-axis-context-menu-panel")
                                .absolute()
                                .left(position.x)
                                .top(position.y)
                                .w(px(d.context_menu_axis_panel_width))
                                .p(px(d.menu_panel_padding))
                                .flex()
                                .flex_col()
                                .gap(px(d.menu_panel_gap))
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
            ContextMenuState::ExplorerFile {
                position,
                path,
                is_dir,
            } => {
                let panel_x = position.x;
                let panel_y = position.y;
                let path = path.clone();
                let is_dir = *is_dir;
                let strings = cx.global::<I18nManager>().strings().clone();
                let is_root = self
                    .panels
                    .explorer
                    .trees_cache
                    .iter()
                    .any(|tree| tree.path == path);
                let can_undo = self.panels.explorer.undo_history.can_undo();
                let can_redo = self.panels.explorer.undo_history.can_redo();
                let has_pasteable = self.panels.explorer.clipboard.is_some();
                let entry_id = self.explorer_id_for_path(&path);

                // Build a menu row: label only (no icons, no keybindings —
                // matching Zed's context menu), optionally disabled, with a
                // danger variant for destructive actions.
                let make_item = |id: &'static str,
                                 label: String,
                                 color: Hsla,
                                 enabled: bool,
                                 editor: WeakEntity<Editor>,
                                 handler: Box<
                    dyn Fn(&mut Editor, &mut Window, &mut Context<Editor>) + 'static,
                >|
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
                            .on_mouse_down(MouseButton::Left, move |_ev, window, cx| {
                                let handler = &handler;
                                let _ = editor.update(cx, move |ed, cx| {
                                    handler(ed, window, cx);
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

                // New File / New Folder (directories only).
                if is_dir {
                    let p = path.clone();
                    items.push(make_item(
                        "ws-ctx-new-file",
                        strings.explorer_new_file.clone(),
                        c.text_default,
                        true,
                        cx.entity().downgrade(),
                        Box::new(move |ed, window, cx| {
                            let p = p.clone();
                            ed.dismiss_contextual_overlays(cx);
                            ed.start_inline_create_file(p, window, cx);
                        }),
                    ));
                    let p = path.clone();
                    items.push(make_item(
                        "ws-ctx-new-folder",
                        strings.explorer_new_folder.clone(),
                        c.text_default,
                        true,
                        cx.entity().downgrade(),
                        Box::new(move |ed, window, cx| {
                            let p = p.clone();
                            ed.dismiss_contextual_overlays(cx);
                            ed.start_inline_create_folder(p, window, cx);
                        }),
                    ));
                    items.push(separator());
                }

                // Reveal / Open with system / Open in terminal.
                let p = path.clone();
                items.push(make_item(
                    "ws-ctx-reveal",
                    strings.explorer_reveal_in_file_manager.clone(),
                    c.text_default,
                    true,
                    cx.entity().downgrade(),
                    Box::new(move |ed, _window, cx| {
                        let p = p.clone();
                        ed.dismiss_contextual_overlays(cx);
                        ed.reveal_in_file_explorer(&p);
                    }),
                ));
                let p = path.clone();
                items.push(make_item(
                    "ws-ctx-open-default",
                    strings.explorer_open_in_default_app.clone(),
                    c.text_default,
                    true,
                    cx.entity().downgrade(),
                    Box::new(move |ed, _window, cx| {
                        let p = p.clone();
                        ed.dismiss_contextual_overlays(cx);
                        ed.open_explorer_with_system(&p);
                    }),
                ));
                items.push(separator());

                // Cut / Copy / Duplicate / Paste (paste disabled without
                // clipboard content), then Undo / Redo (disabled when the
                // corresponding stack is empty) — matching Zed's order.
                items.push(make_item(
                    "ws-ctx-cut",
                    strings.explorer_cut.clone(),
                    c.text_default,
                    true,
                    cx.entity().downgrade(),
                    Box::new(|ed, _window, cx| {
                        ed.dismiss_contextual_overlays(cx);
                        ed.explorer_cut(cx);
                    }),
                ));
                items.push(make_item(
                    "ws-ctx-copy",
                    strings.explorer_copy.clone(),
                    c.text_default,
                    true,
                    cx.entity().downgrade(),
                    Box::new(|ed, _window, cx| {
                        ed.dismiss_contextual_overlays(cx);
                        ed.explorer_copy(cx);
                    }),
                ));
                items.push(make_item(
                    "ws-ctx-duplicate",
                    strings.explorer_duplicate.clone(),
                    c.text_default,
                    true,
                    cx.entity().downgrade(),
                    Box::new(|ed, window, cx| {
                        ed.dismiss_contextual_overlays(cx);
                        ed.explorer_duplicate(window, cx);
                    }),
                ));
                items.push(make_item(
                    "ws-ctx-paste",
                    strings.explorer_paste.clone(),
                    c.text_default,
                    has_pasteable,
                    cx.entity().downgrade(),
                    Box::new(|ed, window, cx| {
                        ed.dismiss_contextual_overlays(cx);
                        ed.explorer_paste(window, cx);
                    }),
                ));
                items.push(make_item(
                    "ws-ctx-undo",
                    strings.explorer_undo.clone(),
                    c.text_default,
                    can_undo,
                    cx.entity().downgrade(),
                    Box::new(|ed, window, cx| {
                        ed.dismiss_contextual_overlays(cx);
                        ed.explorer_undo(window, cx);
                    }),
                ));
                items.push(make_item(
                    "ws-ctx-redo",
                    strings.explorer_redo.clone(),
                    c.text_default,
                    can_redo,
                    cx.entity().downgrade(),
                    Box::new(|ed, window, cx| {
                        ed.dismiss_contextual_overlays(cx);
                        ed.explorer_redo(window, cx);
                    }),
                ));
                items.push(separator());

                // Copy Path / Copy Relative Path.
                let p = path.clone();
                items.push(make_item(
                    "ws-ctx-copy-path",
                    strings.explorer_copy_path.clone(),
                    c.text_default,
                    true,
                    cx.entity().downgrade(),
                    Box::new(move |ed, _window, cx| {
                        let p = p.clone();
                        ed.dismiss_contextual_overlays(cx);
                        ed.copy_path_to_clipboard(&p, cx);
                    }),
                ));
                let p = path.clone();
                items.push(make_item(
                    "ws-ctx-copy-relative-path",
                    strings.explorer_copy_relative_path.clone(),
                    c.text_default,
                    true,
                    cx.entity().downgrade(),
                    Box::new(move |ed, _window, cx| {
                        let p = p.clone();
                        ed.dismiss_contextual_overlays(cx);
                        ed.copy_explorer_relative_path(&p, cx);
                    }),
                ));

                // Rename (hidden for the root, mirroring Zed) and Delete.
                if !is_root {
                    items.push(separator());
                    let p = path.clone();
                    items.push(make_item(
                        "ws-ctx-rename",
                        strings.explorer_rename.clone(),
                        c.text_default,
                        true,
                        cx.entity().downgrade(),
                        Box::new(move |ed, window, cx| {
                            let p = p.clone();
                            ed.dismiss_contextual_overlays(cx);
                            ed.start_inline_rename(p, window, cx);
                        }),
                    ));
                    items.push(make_item(
                        "ws-ctx-trash",
                        strings.explorer_trash.clone(),
                        c.text_default,
                        true,
                        cx.entity().downgrade(),
                        Box::new(|ed, window, cx| {
                            ed.dismiss_contextual_overlays(cx);
                            ed.trash_explorer_selections(window, cx);
                        }),
                    ));
                    items.push(make_item(
                        "ws-ctx-delete",
                        strings.explorer_delete.clone(),
                        c.dialog_danger_button_bg,
                        true,
                        cx.entity().downgrade(),
                        Box::new(|ed, window, cx| {
                            ed.dismiss_contextual_overlays(cx);
                            ed.delete_explorer_selections(window, cx);
                        }),
                    ));
                } else {
                    // Worktree roots: add another folder to the explorer or
                    // remove this one (mirrors Zed's "Add Folders to
                    // Project…" / "Remove from Project").
                    items.push(separator());
                    items.push(make_item(
                        "ws-ctx-add-folder",
                        strings.explorer_add_folder.clone(),
                        c.text_default,
                        true,
                        cx.entity().downgrade(),
                        Box::new(|ed, window, cx| {
                            ed.dismiss_contextual_overlays(cx);
                            ed.prompt_open_explorer_folder(window, cx);
                        }),
                    ));
                    items.push(make_item(
                        "ws-ctx-remove-folder",
                        strings.explorer_remove_folder.clone(),
                        c.text_default,
                        true,
                        cx.entity().downgrade(),
                        Box::new(move |ed, _window, cx| {
                            let p = path.clone();
                            ed.dismiss_contextual_overlays(cx);
                            if let Some(index) = ed
                                .panels
                                .explorer
                                .worktrees
                                .iter()
                                .position(|worktree| worktree.read(cx).root() == p.as_path())
                            {
                                ed.remove_explorer_worktree(index, cx);
                            }
                        }),
                    ));
                }

                // Expand All / Collapse All (directories; root uses the
                // whole-tree variants, mirroring Zed).
                if is_dir {
                    items.push(separator());
                    let entry_id = entry_id;
                    items.push(make_item(
                        "ws-ctx-expand-all",
                        strings.explorer_expand_all.clone(),
                        c.text_default,
                        true,
                        cx.entity().downgrade(),
                        Box::new(move |ed, _window, cx| {
                            ed.dismiss_contextual_overlays(cx);
                            match entry_id {
                                Some((_, id)) if !is_root => {
                                    ed.expand_all_explorer_for_entry(id, cx);
                                }
                                _ => ed.expand_all_explorer_nodes(cx),
                            }
                        }),
                    ));
                    let entry_id = entry_id;
                    items.push(make_item(
                        "ws-ctx-collapse-all",
                        strings.explorer_collapse_all.clone(),
                        c.text_default,
                        true,
                        cx.entity().downgrade(),
                        Box::new(move |ed, _window, cx| {
                            ed.dismiss_contextual_overlays(cx);
                            match entry_id {
                                Some((_, id)) if !is_root => {
                                    ed.collapse_all_explorer_for_entry(id, cx);
                                }
                                _ => ed.collapse_all_explorer_nodes(cx),
                            }
                        }),
                    ));
                }

                Some(
                    overlay()
                        .id("explorer-file-context-menu-overlay")
                        .occlude()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(Self::on_dismiss_context_menu_overlay),
                        )
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
                                .on_mouse_down(MouseButton::Left, |_ev, _window, cx| {
                                    cx.stop_propagation()
                                })
                                .children(items),
                        )
                        .into_any_element(),
                )
            }
        }
    }

    pub(crate) fn render_table_insert_dialog_overlay(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let dialog = self.table_insert_dialog.as_ref()?;
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let s = cx.global::<I18nManager>().strings().clone();

        let stepper =
            |id_prefix: &'static str,
             label: String,
             value: usize,
             on_dec: fn(&mut Editor, &ClickEvent, &mut Window, &mut Context<Editor>),
             on_inc: fn(&mut Editor, &ClickEvent, &mut Window, &mut Context<Editor>)| {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(d.table_insert_stepper_gap))
                    .child(
                        div()
                            .text_size(px(t.dialog_body_size))
                            .font_weight(t.dialog_button_weight.to_font_weight())
                            .text_color(c.dialog_body)
                            .child(label),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(d.table_insert_stepper_gap))
                            .child(
                                div()
                                    .id((id_prefix, 0usize))
                                    .size(px(d.table_insert_stepper_button_size))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(d.table_insert_stepper_radius))
                                    .border(px(d.dialog_border_width))
                                    .border_color(c.dialog_border)
                                    .bg(c.dialog_secondary_button_bg)
                                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                                    .cursor_pointer()
                                    .text_color(c.dialog_secondary_button_text)
                                    .on_click(cx.listener(on_dec))
                                    .child(
                                        svg()
                                            .path("icons/editor/context_menu/minus.svg")
                                            .size(px(12.0))
                                            .text_color(c.dialog_secondary_button_text),
                                    ),
                            )
                            .child(
                                div()
                                    .min_w(px(d.table_insert_stepper_value_min_width))
                                    .h(px(d.table_insert_stepper_button_size))
                                    .px(px(d.table_insert_stepper_value_padding_x))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(d.table_insert_stepper_radius))
                                    .border(px(d.dialog_border_width))
                                    .border_color(c.dialog_border)
                                    .bg(c.dialog_surface)
                                    .text_size(px(t.dialog_body_size))
                                    .text_color(c.dialog_title)
                                    .child(value.to_string()),
                            )
                            .child(
                                div()
                                    .id((id_prefix, 1usize))
                                    .size(px(d.table_insert_stepper_button_size))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(d.table_insert_stepper_radius))
                                    .border(px(d.dialog_border_width))
                                    .border_color(c.dialog_border)
                                    .bg(c.dialog_secondary_button_bg)
                                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                                    .cursor_pointer()
                                    .text_color(c.dialog_secondary_button_text)
                                    .on_click(cx.listener(on_inc))
                                    .child(
                                        svg()
                                            .path("icons/editor/context_menu/plus.svg")
                                            .size(px(12.0))
                                            .text_color(c.dialog_secondary_button_text),
                                    ),
                            ),
                    )
            };

        Some(
            overlay()
                .id("table-insert-dialog-overlay")
                .occlude()
                .flex()
                .items_center()
                .justify_center()
                .bg(c.dialog_backdrop)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(Self::on_dismiss_context_menu_overlay),
                )
                .child(
                    div()
                        .w_full()
                        .px(px(d.editor_padding))
                        .flex()
                        .justify_center()
                        .child(
                            dialog_card(c, d)
                                .id("table-insert-dialog")
                                .w(px(d.dialog_width.min(d.table_insert_dialog_width)))
                                .border(px(d.dialog_border_width))
                                .border_color(c.dialog_border)
                                .rounded(px(d.dialog_radius))
                                .shadow_lg()
                                .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                                    cx.stop_propagation()
                                })
                                .child(
                                    div()
                                        .text_size(px(t.dialog_title_size))
                                        .font_weight(t.dialog_title_weight.to_font_weight())
                                        .text_color(c.dialog_title)
                                        .child(s.table_insert_title.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(t.dialog_body_size))
                                        .font_weight(t.dialog_body_weight.to_font_weight())
                                        .text_color(c.dialog_body)
                                        .child(s.table_insert_description.clone()),
                                )
                                .child(stepper(
                                    "table-body-rows",
                                    s.table_insert_body_rows.clone(),
                                    dialog.body_rows,
                                    Self::on_table_rows_decrement,
                                    Self::on_table_rows_increment,
                                ))
                                .child(stepper(
                                    "table-columns",
                                    s.table_insert_columns.clone(),
                                    dialog.columns,
                                    Self::on_table_columns_decrement,
                                    Self::on_table_columns_increment,
                                ))
                                .child(
                                    div()
                                        .flex()
                                        .justify_end()
                                        .gap(px(d.dialog_button_gap))
                                        .child(
                                            secondary_button("cancel-table-insert-dialog", c, d)
                                                .text_size(px(t.dialog_button_size))
                                                .font_weight(
                                                    t.dialog_button_weight.to_font_weight(),
                                                )
                                                .text_color(c.dialog_secondary_button_text)
                                                .on_click(
                                                    cx.listener(
                                                        Self::on_cancel_table_insert_dialog,
                                                    ),
                                                )
                                                .child(s.table_insert_cancel.clone()),
                                        )
                                        .child(
                                            primary_button("confirm-table-insert-dialog", c, d)
                                                .text_size(px(t.dialog_button_size))
                                                .font_weight(
                                                    t.dialog_button_weight.to_font_weight(),
                                                )
                                                .text_color(c.dialog_primary_button_text)
                                                .on_click(
                                                    cx.listener(
                                                        Self::on_confirm_table_insert_dialog,
                                                    ),
                                                )
                                                .child(s.table_insert_confirm.clone()),
                                        ),
                                ),
                        ),
                )
                .into_any_element(),
        )
    }
}
