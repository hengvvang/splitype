//! Explorer rendering: the virtualized file-tree list, entry rows
//! (including the worktree root rows with their title buttons), the inline
//! edit row, the empty state (with drag-to-open support) and the outline.

use std::ops::Range;
use std::path::{Path, PathBuf};

use gpui::*;

use crate::infra::config::recent::{read_recent_files, read_recent_folders};
use crate::infra::i18n::I18nStrings;
use crate::theme::Theme;
use crate::ui::components::button::icon_chip_button;
use crate::ui::components::empty_state::empty_state_container;
use crate::windows::explorer::drag_and_drop::DraggedExplorerEntryView;
use crate::windows::explorer::filename_editor::ExplorerFilenameInputElement;
use crate::windows::explorer::state::*;

use crate::editor::controller::Editor;

impl Editor {
    pub(crate) fn render_explorer_files_tree(
        &mut self,
        area_id: usize,
        theme: &Theme,
        strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        // Recent files and folders give the empty state a quick-open entry
        // point; stale history entries are filtered out so clicks never fail.
        let recent_folders = read_recent_folders()
            .unwrap_or_default()
            .into_iter()
            .filter(|path| path.is_dir())
            .take(5)
            .collect::<Vec<_>>();
        let recent_files = read_recent_files()
            .unwrap_or_default()
            .into_iter()
            .filter(|path| path.is_file())
            .take(5)
            .collect::<Vec<_>>();

        if self.panels.explorer.worktrees.is_empty() {
            return self.render_explorer_empty_state(
                "Explorer is empty now",
                "",
                area_id,
                theme,
                strings,
                &recent_folders,
                &recent_files,
                cx,
            );
        }

        if let Some(error) = self.panels.explorer.file_error.as_ref() {
            return self.render_explorer_empty_state(
                "Explorer is empty now",
                error,
                area_id,
                theme,
                strings,
                &recent_folders,
                &recent_files,
                cx,
            );
        }

        if self.panels.explorer.trees_cache.is_empty() {
            return self.render_explorer_empty_state(
                "Explorer is empty now",
                "",
                area_id,
                theme,
                strings,
                &recent_folders,
                &recent_files,
                cx,
            );
        }

        let c = &theme.colors;

        // Virtualized row list: only the visible range is rendered. Rows
        // must be uniform height (`EXPLORER_NODE_HEIGHT`). The root row is
        // the first row; its title buttons live on the row itself (shown
        // only while the root is expanded, VSCode-style).
        let entries_len = self.panels.explorer.entries.len();
        let scroll_handle = self.panels.explorer.scroll_handle.clone();
        let row_theme = theme.clone();
        let row_editor = cx.entity().downgrade();
        let list = uniform_list(
            ("explorer-tree", area_id),
            entries_len,
            cx.processor(
                move |this: &mut Editor, range: Range<usize>, _window, cx| {
                    this.panels.explorer.rendered_rows = range.len();
                    // The drag highlight extends to a directory and all of
                    // its descendants; resolve it once per frame.
                    let drag_highlight = this.explorer_drag_highlight_path();
                    let mut items = Vec::with_capacity(range.len());
                    for index in range {
                        if let Some(row) = this.panels.explorer.entries.get(index) {
                            items.push(this.render_explorer_row(
                                row,
                                area_id,
                                drag_highlight.as_deref(),
                                &row_theme,
                                &row_editor,
                                cx,
                            ));
                        }
                    }
                    items
                },
            ),
        )
        .track_scroll(scroll_handle)
        .flex_1()
        .min_h(px(0.0))
        .py(px(4.0));

        div()
            .id(("explorer-root", area_id))
            .key_context("ExplorerPanel")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .bg(if matches!(
                &self.panels.explorer.drag_target,
                Some(DragExplorerTarget::Background)
            ) {
                c.dialog_secondary_button_hover
            } else {
                hsla(0.0, 0.0, 0.0, 0.0)
            })
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
                |editor, event: &ModifiersChangedEvent, window, cx| {
                    editor.refresh_explorer_drag_cursor(&event.modifiers, window, cx);
                },
            ))
            // Background click clears the selection; double-click creates a
            // new file at the root (mirrors Zed). Rows stop propagation, so
            // this only fires on the empty area.
            .on_click(cx.listener(
                |editor, event: &gpui::ClickEvent, window, cx| {
                    if event.click_count() > 1 {
                        if let Some(root) = editor.last_explorer_root_path() {
                            editor.begin_explorer_create(root, false, window, cx);
                        }
                    } else {
                        editor.panels.explorer.selected = None;
                        editor.panels.explorer.marked.clear();
                        cx.notify();
                    }
                },
            ))
            // Right-click on blank space targets the explorer root
            // (mirrors Zed: right-clicking below the last entry is
            // equivalent to right-clicking the root directory).
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|editor, event: &gpui::MouseDownEvent, _window, cx| {
                    // Right-clicking below the last entry targets the last
                    // worktree root (mirrors Zed: background right-click is
                    // equivalent to right-clicking the root directory).
                    if let Some((root, path, root_id)) = editor.last_explorer_root() {
                        editor.panels.explorer.selected = Some(ExplorerSelection::File {
                            root,
                            entry: root_id,
                        });
                        editor.open_explorer_file_context_menu(event.position, path, true, cx);
                        cx.notify();
                    }
                }),
            )
            // Panel-level drag handling: cursor style + edge auto-scroll +
            // out-of-bounds cleanup (mirrors Zed's `handle_drag_move`).
            .on_drag_move::<ExternalPaths>(cx.listener(|editor, event, window, cx| {
                editor.explorer_drag_hover_background_external(event, window, cx);
            }))
            .on_drop::<ExternalPaths>(cx.listener::<ExternalPaths>(
                |editor, paths, window, cx| {
                    editor.on_explorer_drop_external_to_root(paths.paths(), window, cx);
                },
            ))
            .on_drag_move::<DraggedExplorerSelection>(
                cx.listener(|editor, event, window, cx| {
                    editor.explorer_drag_hover_background_internal(event, window, cx);
                }),
            )
            .on_drop::<DraggedExplorerSelection>(
                cx.listener(|editor, payload, window, cx| {
                    editor.on_explorer_drop_internal_to_root(payload, window, cx);
                }),
            )
            .child(list)
            .into_any_element()
    }

    /// The path whose highlight extends to all of its descendants during a
    /// drag (the highlight target of the current drag, if any).
    pub(crate) fn explorer_drag_highlight_path(&self) -> Option<PathBuf> {
        self.panels
            .explorer
            .drag_target
            .and_then(|target| target.highlight_entry_id())
            .and_then(|id| self.explorer_entry_by_id(id))
            .map(|entry| entry.path.clone())
    }

    /// Render one file-tree row for the virtualized list: either a visible
    /// entry or the inline edit row. `drag_highlight` is the drag target's
    /// highlight path (its descendants are highlighted too).
    pub(crate) fn render_explorer_row(
        &self,
        row: &ExplorerRow,
        area_id: usize,
        drag_highlight: Option<&Path>,
        theme: &Theme,
        editor: &WeakEntity<Editor>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match row {
            ExplorerRow::Entry(entry) if entry.parent_id.is_none() => {
                self.render_explorer_root_row(entry, area_id, drag_highlight, theme, editor, cx)
            }
            ExplorerRow::Entry(entry) => {
                self.render_explorer_entry_row(entry, area_id, drag_highlight, theme, editor, cx)
            }
            ExplorerRow::Edit { .. } => self.render_explorer_edit_row(area_id, theme, editor, cx),
        }
    }

    /// Render the root row: the folder name plus the title buttons (new
    /// file / new folder / refresh / collapse all / toggle hidden). The
    /// buttons are shown only while the root is expanded (VSCode-style
    /// title row); collapsing the root hides them. Root rows are draggable
    /// to reorder worktrees (mirrors Zed).
    pub(crate) fn render_explorer_root_row(
        &self,
        entry: &VisibleExplorerEntry,
        area_id: usize,
        drag_highlight: Option<&Path>,
        theme: &Theme,
        editor: &WeakEntity<Editor>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let t = &theme.typography;
        let selected = matches!(
            &self.panels.explorer.selected,
            Some(ExplorerSelection::File { entry: entry_id, .. }) if *entry_id == entry.id
        );
        let is_drag_target = drag_highlight
            .is_some_and(|highlight| entry.path == *highlight || entry.path.starts_with(highlight));
        let is_expanded = entry.is_expanded;
        let node_id = entry.id;
        let click_editor = editor.clone();
        let right_click_editor = editor.clone();
        let right_click_path = entry.path.clone();
        let arrow_node_id = entry.id;
        let arrow_editor = editor.clone();
        let mark_selection = ExplorerSelection::File {
            root: entry.root,
            entry: entry.id,
        };
        let drag_entry_id = entry.id;
        let drag_label = entry.label.clone();
        let drag_payload = DraggedExplorerSelection {
            selections: vec![mark_selection.clone()],
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
                            "icon/panel/chevron-down.svg"
                        } else {
                            "icon/panel/chevron-right.svg"
                        })
                        .size(px(12.0))
                        .text_color(c.dialog_muted),
                )
                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                    let _ = arrow_editor.update(cx, |editor, cx| {
                        editor.toggle_explorer_node(arrow_node_id, cx);
                    });
                    cx.stop_propagation();
                });
        }

        let ed_open = editor.clone();
        let ed_refresh = editor.clone();
        let ed_collapse = editor.clone();

        // Title buttons: visible only while the root row is expanded. The
        // set matches the panel toolbar minus new file / new folder, which
        // live on the row context menu (Zed layout).
        let buttons = if is_expanded {
            div()
                .flex()
                .items_center()
                .gap(px(2.0))
                .child(
                    icon_chip_button(c, &theme.dimensions)
                        .id(("ws-tb-open", area_id))
                        .child(
                            svg()
                                .path("icon/explorer/folder-open.svg")
                                .size(px(13.0))
                                .text_color(c.text_default),
                        )
                        .on_click(move |_ev, window, cx| {
                            let _ = ed_open.update(cx, |ed, cx| {
                                ed.prompt_open_explorer_folder(window, cx);
                            });
                            cx.stop_propagation();
                        }),
                )
                .child(
                    icon_chip_button(c, &theme.dimensions)
                        .id(("ws-tb-refresh", area_id))
                        .child(
                            svg()
                                .path("icon/explorer/refresh.svg")
                                .size(px(13.0))
                                .text_color(c.text_default),
                        )
                        .on_click(move |_ev, _window, cx| {
                            let _ = ed_refresh.update(cx, |ed, cx| {
                                ed.refresh_explorer_tree(cx);
                            });
                            cx.stop_propagation();
                        }),
                )
                .child(
                    icon_chip_button(c, &theme.dimensions)
                        .id(("ws-tb-collapse", area_id))
                        .child(
                            svg()
                                .path("icon/explorer/collapse-all.svg")
                                .size(px(13.0))
                                .text_color(c.text_default),
                        )
                        .on_click(move |_ev, _window, cx| {
                            let _ = ed_collapse.update(cx, |ed, cx| {
                                ed.collapse_all_explorer_nodes(cx);
                            });
                            cx.stop_propagation();
                        }),
                )
                .into_any_element()
        } else {
            div().into_any_element()
        };

        div()
            .id(ElementId::Name(format!("explorer-root-row-{area_id}").into()))
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
                c.panel_row_selected
            } else {
                hsla(0.0, 0.0, 0.0, 0.0)
            })
            .hover(|this| this.bg(c.panel_row_hover))
            .cursor_pointer()
            .child(arrow_el)
            .child(
                svg()
                    .path(FOLDER_ICON)
                    .size(px(17.0))
                    .flex_shrink_0()
                    .text_color(c.text_default),
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
                    let _ = right_click_editor.update(cx, |editor, cx| {
                        // Right-click selects the row (indicator feedback,
                        // mirroring Zed's `deploy_context_menu`); marked
                        // entries are cleared when the target is not one of
                        // them, so menu actions never surprise multi-selects.
                        editor.panels.explorer.selected = Some(selection.clone());
                        if !editor.panels.explorer.marked.contains(&selection) {
                            editor.panels.explorer.marked.clear();
                        }
                        editor.open_explorer_file_context_menu(event.position, path, true, cx);
                        cx.notify();
                    });
                    cx.stop_propagation();
                }
            })
            .on_click(move |event, _window, cx| {
                let id = node_id;
                let selection = mark_selection.clone();
                let shift = event.modifiers().shift;
                let alt = event.modifiers().alt;
                let secondary = event.modifiers().secondary();
                let _ = click_editor.update(cx, |editor, cx| {
                    if shift {
                        editor.select_explorer_range(id, cx);
                        return;
                    }
                    if secondary {
                        editor.toggle_explorer_mark(selection, cx);
                        return;
                    }
                    editor.panels.explorer.marked.clear();
                    if alt {
                        editor.toggle_explorer_subtree(id, cx);
                    } else {
                        editor.toggle_explorer_node(id, cx);
                    }
                });
                // Rows must not let clicks bubble to the panel background
                // (background click clears the selection).
                cx.stop_propagation();
            })
            // Drag & drop: the root row is a drop target like any directory
            // row, and dragging it reorders worktrees (see
            // `on_explorer_drop_internal`). Drag moves bubble up to the panel
            // background for cursor/scroll handling; drops stop propagation
            // so the background does not handle the same drop twice.
            .on_drag_move::<ExternalPaths>(cx.listener(move |editor, event, window, cx| {
                editor.explorer_drag_hover_entry_external(event, drag_entry_id, window, cx);
            }))
            .on_drop::<ExternalPaths>(cx.listener::<ExternalPaths>(
                move |editor, paths, window, cx| {
                    editor.on_explorer_drop_external(paths.paths(), drag_entry_id, window, cx);
                    cx.stop_propagation();
                },
            ))
            .on_drag_move::<DraggedExplorerSelection>(
                cx.listener(move |editor, event, window, cx| {
                    editor.explorer_drag_hover_entry_internal(event, drag_entry_id, window, cx);
                }),
            )
            .on_drop::<DraggedExplorerSelection>(cx.listener(move |editor, payload, window, cx| {
                editor.on_explorer_drop_internal(payload, drag_entry_id, window, cx);
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

    /// Render one flat file-tree entry row.
    pub(crate) fn render_explorer_entry_row(
        &self,
        entry: &VisibleExplorerEntry,
        area_id: usize,
        drag_highlight: Option<&Path>,
        theme: &Theme,
        editor: &WeakEntity<Editor>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let t = &theme.typography;
        let selected = matches!(
            &self.panels.explorer.selected,
            Some(ExplorerSelection::File { entry: entry_id, .. }) if *entry_id == entry.id
        );
        let is_marked = self
            .panels
            .explorer
            .marked
            .contains(&ExplorerSelection::File {
                root: entry.root,
                entry: entry.id,
            });
        let is_drag_target = drag_highlight
            .is_some_and(|highlight| entry.path == *highlight || entry.path.starts_with(highlight));
        let node_id = entry.id;
        let click_editor = editor.clone();
        let click_kind = entry.kind;
        let click_path = entry.path.clone();
        let right_click_editor = editor.clone();
        let right_click_path = entry.path.clone();
        let right_click_is_dir = entry.kind == ExplorerEntryKind::Directory;
        let arrow_node_id = entry.id;
        let arrow_editor = editor.clone();
        let mark_selection = ExplorerSelection::File {
            root: entry.root,
            entry: entry.id,
        };
        // Drag payload: the row where the drag started first, then the
        // marked entries it carries along (mirrors Zed's `DraggedSelection`).
        let mut drag_selections = vec![mark_selection.clone()];
        for selection in &self.panels.explorer.marked {
            if !drag_selections.contains(selection) {
                drag_selections.push(selection.clone());
            }
        }
        let drag_payload = DraggedExplorerSelection {
            selections: drag_selections,
        };
        let drag_label = entry.label.clone();
        let drag_entry_id = entry.id;

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
                match ext.as_str() {
                    "md" => Some((MARKDOWN_ICON, c.text_default)),
                    _ => Some((FILE_ICON, c.text_default)),
                }
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
                            "icon/panel/chevron-down.svg"
                        } else {
                            "icon/panel/chevron-right.svg"
                        })
                        .size(px(12.0))
                        .text_color(c.dialog_muted),
                )
                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                    let _ = arrow_editor.update(cx, |editor, cx| {
                        editor.toggle_explorer_node(arrow_node_id, cx);
                    });
                    cx.stop_propagation();
                });
        }

        div()
            .id(ElementId::Name(
                format!("explorer-node-{area_id}-{}", node_id.0).into(),
            ))
            .h(px(EXPLORER_NODE_HEIGHT))
            .w_full()
            .overflow_hidden()
            .flex()
            .items_center()
            .gap(px(6.0))
            .pl(px(6.0 + entry.depth as f32 * EXPLORER_NODE_INDENT))
            .pr(px(8.0))
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
                    .size(px(17.0))
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
                let right_click_selection = mark_selection.clone();
                move |event, _window, cx| {
                    let path = right_click_path.clone();
                    let is_dir = right_click_is_dir;
                    let selection = right_click_selection.clone();
                    let _ = right_click_editor.update(cx, |editor, cx| {
                        // Right-click selects the row (indicator feedback,
                        // mirroring Zed's `deploy_context_menu`); marked
                        // entries are cleared when the target is not one of
                        // them, so menu actions never surprise multi-selects.
                        editor.panels.explorer.selected = Some(selection.clone());
                        if !editor.panels.explorer.marked.contains(&selection) {
                            editor.panels.explorer.marked.clear();
                        }
                        editor.open_explorer_file_context_menu(event.position, path, is_dir, cx);
                        cx.notify();
                    });
                    cx.stop_propagation();
                }
            })
            .on_click(move |event, window, cx| {
                let id = node_id;
                let kind = click_kind;
                let path = click_path.clone();
                let selection = mark_selection.clone();
                let click_count = event.click_count();
                let shift = event.modifiers().shift;
                let alt = event.modifiers().alt;
                let secondary = event.modifiers().secondary();
                let _ = click_editor.update(cx, |editor, cx| {
                    if shift {
                        editor.select_explorer_range(id, cx);
                        return;
                    }
                    if secondary {
                        if click_count > 1 {
                            // Ctrl/Cmd+double-click: open in a split area.
                            editor.split_explorer_file(path, window, cx);
                        } else {
                            editor.toggle_explorer_mark(selection, cx);
                        }
                        return;
                    }
                    editor.panels.explorer.marked.clear();
                    match kind {
                        ExplorerEntryKind::Directory => {
                            if alt {
                                editor.toggle_explorer_subtree(id, cx);
                            } else {
                                editor.toggle_explorer_node(id, cx);
                            }
                        }
                        ExplorerEntryKind::MarkdownFile | ExplorerEntryKind::File => {
                            editor.open_explorer_file_click(path, click_count > 1, window, cx);
                        }
                    }
                });
                // Rows must not let clicks bubble to the panel background
                // (background click clears the selection).
                cx.stop_propagation();
            })
            // Drag & drop: external files are copied; internal entries are
            // moved by default and copied with the copy modifier. Drag moves
            // bubble up to the panel background for cursor/scroll handling;
            // drops stop propagation so the background does not handle the
            // same drop twice.
            .on_drag_move::<ExternalPaths>(cx.listener(move |editor, event, window, cx| {
                editor.explorer_drag_hover_entry_external(event, drag_entry_id, window, cx);
            }))
            .on_drop::<ExternalPaths>(cx.listener::<ExternalPaths>(
                move |editor, paths, window, cx| {
                    editor.on_explorer_drop_external(paths.paths(), drag_entry_id, window, cx);
                    cx.stop_propagation();
                },
            ))
            .on_drag_move::<DraggedExplorerSelection>(
                cx.listener(move |editor, event, window, cx| {
                    editor.explorer_drag_hover_entry_internal(event, drag_entry_id, window, cx);
                }),
            )
            .on_drop::<DraggedExplorerSelection>(cx.listener(move |editor, payload, window, cx| {
                editor.on_explorer_drop_internal(payload, drag_entry_id, window, cx);
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

    /// Render the inline create/rename row: a filename input with keyboard
    /// handling, IME bridge, and live validation feedback.
    pub(crate) fn render_explorer_edit_row(
        &self,
        area_id: usize,
        theme: &Theme,
        _editor: &WeakEntity<Editor>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(edit) = self.panels.explorer.edit.as_ref() else {
            return div().into_any_element();
        };
        let c = &theme.colors;
        let t = &theme.typography;
        let depth = edit.depth;
        let is_dir = edit.is_dir;
        let validation = edit.validation.clone();
        let focus_handle = edit.filename.focus_handle.clone().unwrap();

        let icon = if is_dir {
            (FOLDER_ICON, c.text_default)
        } else {
            (MARKDOWN_ICON, c.text_default)
        };

        let validation_label = match validation {
            Some(ExplorerValidation::Warning(message)) => Some((
                message,
                c.callout_warning_border,
            )),
            Some(ExplorerValidation::Error(message)) => Some((message, c.callout_caution_border)),
            None => None,
        };

        div()
            .id(ElementId::Name(format!("explorer-edit-{area_id}").into()))
            .h(px(EXPLORER_NODE_HEIGHT))
            .w_full()
            .overflow_hidden()
            .flex()
            .items_center()
            .gap(px(6.0))
            .pl(px(6.0 + depth as f32 * EXPLORER_NODE_INDENT))
            .pr(px(8.0))
            .bg(c.dialog_secondary_button_hover)
            // Clicks inside the edit row must not reach the panel
            // background (double-click there would create a new file).
            .on_click(|_ev, _window, cx| cx.stop_propagation())
            // Arrow placeholder keeps the row aligned with siblings.
            .child(
                div()
                    .w(px(14.0))
                    .h(px(18.0))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(svg().size(px(12.0))),
            )
            .child(
                svg()
                    .path(icon.0)
                    .size(px(17.0))
                    .flex_shrink_0()
                    .text_color(icon.1),
            )
            .child(
                div()
                    .id(("explorer-filename-input-box", area_id))
                    .key_context("ExplorerFilenameInput")
                    .track_focus(&focus_handle)
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .items_center()
                    .on_key_down(cx.listener(Self::on_explorer_filename_key_down))
                    // The global keymap binds escape to `DismissTransientUi`;
                    // GPUI dispatches matched actions BEFORE raw key
                    // listeners, so Esc must be handled as an action here
                    // (the focused node runs first) — `on_key_down` would
                    // never see it.
                    .on_action(cx.listener(Self::on_explorer_escape))
                    .on_action(cx.listener(Self::on_explorer_filename_copy))
                    .on_action(cx.listener(Self::on_explorer_filename_cut))
                    .on_action(cx.listener(Self::on_explorer_filename_paste))
                    .child(ExplorerFilenameInputElement {
                        editor: cx.entity(),
                    }),
            )
            .children(validation_label.map(|(message, color)| {
                div()
                    .max_w(px(160.0))
                    .truncate()
                    .text_size(px(t.text_size * 0.72))
                    .text_color(color)
                    .child(message)
                    .into_any_element()
            }))
            .into_any_element()
    }

    pub(crate) fn render_explorer_empty_state(
        &self,
        title: &str,
        message: &str,
        area_id: usize,
        theme: &Theme,
        strings: &I18nStrings,
        recent_folders: &[PathBuf],
        recent_files: &[PathBuf],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let click_editor = cx.entity().downgrade();
        let drop_target_bg = c.dialog_secondary_button_hover;

        let display_title = if title.is_empty() {
            "Explorer is empty now"
        } else {
            title
        };

        // An empty message means the empty state has no hint line at all;
        // non-empty messages (e.g. scan errors) are still rendered.
        let has_message = !message.is_empty();

        empty_state_container()
            .gap(px(10.0))
            .px(px(24.0))
            // Dropping folders onto the empty state opens them as worktrees
            // (mirrors Zed's empty-state drop-to-open).
            .drag_over::<ExternalPaths>(move |this, _, _, _| this.bg(drop_target_bg))
            .on_drop::<ExternalPaths>(cx.listener::<ExternalPaths>(
                |editor, paths, window, cx| {
                    for path in paths.paths() {
                        if path.is_dir() {
                            editor.open_explorer_folder_path(path.clone(), cx);
                        } else {
                            editor.open_explorer_file(path.clone(), window, cx);
                        }
                    }
                },
            ))
            .child(
                svg()
                    .path("icon/explorer/folder-open.svg")
                    .size(px(36.0))
                    .text_color(c.dialog_muted),
            )
            .child(
                div()
                    .text_size(px(13.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(c.text_default)
                    .child(display_title.to_string()),
            )
            .child(if has_message {
                div()
                    .max_w(px(230.0))
                    .text_size(px(t.text_size * 0.78))
                    .line_height(px(t.text_size * t.text_line_height * 0.90))
                    .text_color(c.dialog_muted)
                    .child(message.to_string())
            } else {
                div()
            })
            .child(
                div()
                    .id(("explorer-empty-open-btn", area_id))
                    .cursor_pointer()
                    .mt(px(4.0))
                    .h(px(28.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.0))
                    .rounded(px(d.menu_item_radius))
                    .border_1()
                    .border_color(c.dialog_border)
                    .bg(c.dialog_secondary_button_bg)
                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                    .active(|this| this.opacity(0.92))
                    .child(
                        svg()
                            .path("icon/explorer/folder-open.svg")
                            .size(px(14.0))
                            .text_color(c.dialog_secondary_button_text),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(c.dialog_secondary_button_text)
                            .child("Open Folder"),
                    )
                    .on_click(move |_ev, window, cx| {
                        let _ = click_editor.update(cx, |ed, cx| {
                            ed.prompt_open_explorer_folder(window, cx);
                        });
                    }),
            )
            .child(
                // Recent folders and files quick-open list under the button;
                // hidden when both histories are empty or the state carries
                // an error message.
                if (recent_folders.is_empty() && recent_files.is_empty()) || has_message {
                    div()
                } else {
                    div()
                        .mt(px(16.0))
                        .w_full()
                        .flex()
                        .flex_col()
                        .items_start()
                        .gap(px(2.0))
                        .child(
                            div()
                                .ml(px(10.0))
                                .text_size(px(13.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(c.dialog_muted)
                                .child(strings.explorer_recent_title.clone()),
                        )
                        .children(recent_folders.iter().map(|path| {
                            let folder_name = path
                                .file_name()
                                .map(|name| name.to_string_lossy().to_string())
                                .unwrap_or_else(|| path.to_string_lossy().to_string());
                            let ed = cx.entity().downgrade();
                            let path = path.clone();
                            div()
                                .id(ElementId::Name(
                                    format!(
                                        "explorer-recent-folder-{}-{}",
                                        area_id,
                                        path.display()
                                    )
                                    .into(),
                                ))
                                .cursor_pointer()
                                .px(px(10.0))
                                .py(px(2.0))
                                .rounded(px(d.menu_item_radius))
                                .hover(|this| this.bg(c.panel_row_hover))
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(
                                    svg()
                                        .path("icon/explorer/folder.svg")
                                        .size(px(12.0))
                                        .text_color(c.dialog_muted),
                                )
                                .child(
                                    div()
                                        .max_w(px(190.0))
                                        .truncate()
                                        .text_size(px(12.0))
                                        .text_color(c.dialog_muted)
                                        .hover(|this| this.text_color(c.text_default))
                                        .child(folder_name),
                                )
                                .on_click(move |_, _window, cx| {
                                    let _ = ed.update(cx, |editor, cx| {
                                        editor.open_explorer_folder_path(path.clone(), cx);
                                    });
                                })
                        }))
                        .children(recent_files.iter().map(|path| {
                            let file_name = path
                                .file_name()
                                .map(|name| name.to_string_lossy().to_string())
                                .unwrap_or_else(|| path.to_string_lossy().to_string());
                            let ed = cx.entity().downgrade();
                            let path = path.clone();
                            div()
                                .id(ElementId::Name(
                                    format!("explorer-recent-{}-{}", area_id, path.display()).into(),
                                ))
                                .cursor_pointer()
                                .px(px(10.0))
                                .py(px(2.0))
                                .rounded(px(d.menu_item_radius))
                                .hover(|this| this.bg(c.panel_row_hover))
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(
                                    svg()
                                        .path("icon/explorer/markdown.svg")
                                        .size(px(12.0))
                                        .text_color(c.dialog_muted),
                                )
                                .child(
                                    div()
                                        .max_w(px(190.0))
                                        .truncate()
                                        .text_size(px(12.0))
                                        .text_color(c.dialog_muted)
                                        .hover(|this| this.text_color(c.text_default))
                                        .child(file_name),
                                )
                                .on_click(move |_, window, cx| {
                                    let _ = ed.update(cx, |editor, cx| {
                                        editor.open_explorer_file(path.clone(), window, cx);
                                    });
                                })
                        }))
                },
            )
            .into_any_element()
    }

    // ── Outline rendering (non-virtualized; heading trees are small) ────

    pub(crate) fn render_explorer_nodes(
        &self,
        nodes: &[ExplorerNode],
        depth: usize,
        area_id: usize,
        theme: &Theme,
        editor: &WeakEntity<Editor>,
    ) -> Vec<AnyElement> {
        let mut elements = Vec::new();
        for node in nodes {
            elements.push(self.render_explorer_node(node, depth, area_id, theme, editor));
            if !node.children.is_empty()
                && self.panels.explorer.expanded_outline.contains(&node.id)
            {
                elements.extend(self.render_explorer_nodes(
                    &node.children,
                    depth + 1,
                    area_id,
                    theme,
                    editor,
                ));
            }
        }
        elements
    }
    pub(crate) fn render_explorer_node(
        &self,
        node: &ExplorerNode,
        depth: usize,
        area_id: usize,
        theme: &Theme,
        editor: &WeakEntity<Editor>,
    ) -> AnyElement {
        let c = &theme.colors;
        let t = &theme.typography;
        let is_expanded = self.panels.explorer.expanded_outline.contains(&node.id);
        let has_children = !node.children.is_empty();
        let selected = matches!(
            &self.panels.explorer.selected,
            Some(ExplorerSelection::Outline(id)) if id == &node.id
        );
        let node_id = node.id.clone();
        let click_editor = editor.clone();
        let click_kind = node.kind.clone();
        let arrow_node_id = node.id.clone();
        let arrow_editor = editor.clone();

        let heading_badge = match &node.kind {
            ExplorerNodeKind::Heading { level, .. } => {
                let badge_color = match level {
                    1 => c.callout_note_border,
                    2 => c.callout_tip_border,
                    3 => c.callout_important_border,
                    4 => c.callout_warning_border,
                    5 => c.callout_caution_border,
                    _ => c.dialog_muted,
                };
                Some(
                    div()
                        .px(px(4.0))
                        .py(px(1.0))
                        .rounded(px(3.0))
                        .text_size(px(10.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(badge_color)
                        .bg(badge_color.opacity(0.12))
                        .child(format!("H{level}")),
                )
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

        if has_children {
            arrow_el = arrow_el
                .cursor_pointer()
                .child(
                    svg()
                        .path(if is_expanded {
                            "icon/panel/chevron-down.svg"
                        } else {
                            "icon/panel/chevron-right.svg"
                        })
                        .size(px(12.0))
                        .text_color(c.dialog_muted),
                )
                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                    let _ = arrow_editor.update(cx, |editor, cx| {
                        editor.toggle_outline_node(&arrow_node_id, cx);
                    });
                    cx.stop_propagation();
                });
        }

        div()
            .id(ElementId::Name(
                format!("explorer-node-{area_id}-{}", stable_node_hash(&node.id)).into(),
            ))
            .h(px(EXPLORER_NODE_HEIGHT))
            .w_full()
            .overflow_hidden()
            .flex()
            .items_center()
            .gap(px(6.0))
            .pl(px(6.0 + depth as f32 * EXPLORER_NODE_INDENT))
            .pr(px(8.0))
            .bg(if selected {
                c.panel_row_selected
            } else {
                hsla(0.0, 0.0, 0.0, 0.0)
            })
            .hover(|this| this.bg(c.panel_row_hover))
            .cursor_pointer()
            .child(arrow_el)
            .children(heading_badge)
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .truncate()
                    .text_size(px(t.text_size * 0.9))
                    .line_height(px(t.text_size * t.text_line_height))
                    .text_color(label_color)
                    .child(node.label.clone()),
            )
            .on_click(move |_event, _window, cx| {
                let node_id = node_id.clone();
                let click_kind = click_kind.clone();
                let _ = click_editor.update(cx, |editor, cx| match click_kind {
                    ExplorerNodeKind::Heading { .. } => {
                        editor.select_outline_node(node_id, cx);
                    }
                });
            })
            .into_any_element()
    }
    pub(crate) fn render_explorer_panel(
        &mut self,
        area_id: usize,
        theme: &Theme,
        strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.sync_explorer_models(cx);
        self.render_explorer_files_tree(area_id, theme, strings, cx)
    }
}
