//! Rendering of the editor's context menus: the axis menu items and the
//! overlay panel. The table-insert dialog moved to `dialogs.rs`.

use splitype_ui::menu_item::menu_item;
use splitype_ui::popover::menu_panel;
use splitype_ui::popover::overlay;

use gpui::*;

use crate::editor::engine::controller::Editor;
use crate::editor::panes::document_pane::context_menu::ContextMenuState;
use splitype_infra::i18n::I18nManager;
use splitype_infra::theme::Theme;
use splitype_model::block::table::TableAxis;
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

    /// Renders the footnote content tooltip floating near the pointer when a
    /// footnote reference or definition header is hovered.
    pub(crate) fn render_footnote_tooltip(
        &self,
        theme: &Theme,
        window: &Window,
        _cx: &App,
    ) -> Option<AnyElement> {
        let tooltip = self.footnote_tooltip.as_ref()?;
        let c = &theme.colors;
        let origin = self.panel_rect.map(|rect| rect.origin).unwrap_or_default();
        let top = (tooltip.position.y - origin.y + px(4.0)).max(px(0.0));

        let viewport_width = f32::from(window.viewport_size().width.max(px(1.0)));
        let panel_width = self
            .panel_rect
            .map(|r| f32::from(r.size.width))
            .unwrap_or(viewport_width);
        let max_width = 420.0_f32;
        let mut left_f32 = f32::from(tooltip.position.x - origin.x);
        if left_f32 + 200.0 > panel_width {
            left_f32 = (panel_width - max_width.min(panel_width) - 16.0).max(8.0);
        }
        let left = px(left_f32.max(8.0));

        Some(
            div()
                .absolute()
                .occlude()
                .left(left)
                .top(top)
                .max_w(px(max_width))
                .px(px(10.0))
                .py(px(6.0))
                .rounded(px(5.0))
                .bg(c.dialog_surface)
                .border(px(1.0))
                .border_color(c.dialog_border)
                .shadow_md()
                .text_size(px(13.0))
                .text_color(c.dialog_muted)
                .line_height(relative(1.5))
                .child(tooltip.content.clone())
                .into_any_element(),
        )
    }


    pub(crate) fn render_menu_item_row(
        theme: &Theme,
        id: &'static str,
        label: String,
        enabled: bool,
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
                .text_color(c.dialog_secondary_button_text)
                .child(label)
                .on_click(cx.listener(on_click))
                .on_hover(cx.listener(|this, hovered, _window, cx| {
                    if *hovered {
                        this.set_context_menu_submenu_hover(None, false, cx);
                    }
                }))
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
                .text_color(c.dialog_muted)
                .child(label)
                .into_any_element()
        }
    }

    pub(crate) fn render_menu_check_row(
        theme: &Theme,
        id: &'static str,
        label: String,
        is_checked: bool,
        on_click: fn(&mut Editor, &ClickEvent, &mut Window, &mut Context<Editor>),
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        menu_item(id, c, d)
            .justify_between()
            .text_size(px(d.menu_text_size))
            .font_weight(t.dialog_body_weight.to_font_weight())
            .text_color(c.dialog_secondary_button_text)
            .child(label)
            .children(is_checked.then(|| {
                svg()
                    .path("icons/titlebar/app_menu/checkmark.svg")
                    .size(px(14.0))
                    .text_color(c.dialog_primary_button_bg)
            }))
            .on_click(cx.listener(on_click))
            .into_any_element()
    }

    pub(crate) fn render_menu_separator(theme: &Theme) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        div()
            .mx(px(d.menu_separator_margin_x))
            .my(px(d.menu_separator_margin_y))
            .h(px(d.menu_separator_height))
            .bg(c.dialog_border)
            .into_any_element()
    }

    pub(crate) fn render_submenu_trigger(
        theme: &Theme,
        id: &'static str,
        label: String,
        is_active: bool,
        submenu_kind: crate::editor::panes::document_pane::context_menu::ContextSubmenu,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        menu_item(id, c, d)
            .justify_between()
            .bg(if is_active {
                c.panel_row_selected
            } else {
                c.dialog_surface
            })
            .text_size(px(d.menu_text_size))
            .font_weight(t.dialog_body_weight.to_font_weight())
            .text_color(c.dialog_secondary_button_text)
            .child(label)
            .child(
                svg()
                    .path("icons/editor/context_menu/chevron-right.svg")
                    .size(px(14.0))
                    .text_color(c.dialog_muted),
            )
            .on_click(cx.listener(move |this, _event, _window, cx| {
                this.set_context_menu_submenu_hover(Some(submenu_kind), false, cx);
            }))
            .on_hover(cx.listener(move |this, hovered, _window, cx| {
                if *hovered {
                    this.set_context_menu_submenu_hover(Some(submenu_kind), false, cx);
                } else {
                    this.set_context_menu_submenu_hover(None, false, cx);
                }
            }))
            .into_any_element()
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
        let origin = self.panel_rect.map(|rect| rect.origin).unwrap_or_default();

        match menu {
            ContextMenuState::Edit {
                position,
                target_entity,
                active_submenu,
                ..
            } => {
                let panel_x = position.x - origin.x;
                let panel_y = position.y - origin.y;
                let panel_width = px(d.context_menu_panel_width.max(180.0));
                let has_selection = self.context_menu_has_selection(cx);

                let pane_id = self.active_pane_id();
                let active_kind = target_entity
                    .and_then(|id| self.focusable_entity_by_id(id))
                    .or_else(|| {
                        self.pane_state_ref(pane_id)
                            .and_then(|p| p.as_wysiwyg())
                            .and_then(|w| w.focus.active_entity)
                            .and_then(|id| self.focusable_entity_by_id(id))
                    })
                    .map(|b| b.read(cx).kind().clone());

                let (is_h1, is_h2, is_h3, is_h4, is_h5, is_h6, is_p) = if self.is_source_code() {
                    let source_line = self
                        .pane_state_ref(pane_id)
                        .and_then(|p| p.as_source_code())
                        .map(|source| {
                            let (cur_line, _) = source.line_and_column(source.cursor);
                            let start = source.line_start_offset(cur_line);
                            let end = source.line_end_offset(cur_line);
                            source.text[start..end].trim_start().to_string()
                        })
                        .unwrap_or_default();

                    let h1 = source_line.starts_with("# ");
                    let h2 = source_line.starts_with("## ");
                    let h3 = source_line.starts_with("### ");
                    let h4 = source_line.starts_with("#### ");
                    let h5 = source_line.starts_with("##### ");
                    let h6 = source_line.starts_with("###### ");
                    let p = !h1 && !h2 && !h3 && !h4 && !h5 && !h6;
                    (h1, h2, h3, h4, h5, h6, p)
                } else {
                    let h1 = matches!(active_kind, Some(splitype_model::parse::BlockKind::Heading { level: 1 }));
                    let h2 = matches!(active_kind, Some(splitype_model::parse::BlockKind::Heading { level: 2 }));
                    let h3 = matches!(active_kind, Some(splitype_model::parse::BlockKind::Heading { level: 3 }));
                    let h4 = matches!(active_kind, Some(splitype_model::parse::BlockKind::Heading { level: 4 }));
                    let h5 = matches!(active_kind, Some(splitype_model::parse::BlockKind::Heading { level: 5 }));
                    let h6 = matches!(active_kind, Some(splitype_model::parse::BlockKind::Heading { level: 6 }));
                    let p = matches!(active_kind, Some(splitype_model::parse::BlockKind::Paragraph)) || active_kind.is_none();
                    (h1, h2, h3, h4, h5, h6, p)
                };

                let submenu = active_submenu.map(|sub| {
                    use crate::editor::panes::document_pane::context_menu::ContextSubmenu;
                    let (submenu_items, y_offset) = match sub {
                        ContextSubmenu::TextFormat => (
                            vec![
                                Self::render_menu_item_row(theme, "menu-fmt-bold", s.context_menu_bold.clone(), true, Self::on_format_bold, cx),
                                Self::render_menu_item_row(theme, "menu-fmt-italic", s.context_menu_italic.clone(), true, Self::on_format_italic, cx),
                                Self::render_menu_item_row(theme, "menu-fmt-strikethrough", s.context_menu_strikethrough.clone(), true, Self::on_format_strikethrough, cx),
                                Self::render_menu_item_row(theme, "menu-fmt-highlight", s.context_menu_highlight.clone(), true, Self::on_format_highlight, cx),
                                Self::render_menu_separator(theme),
                                Self::render_menu_item_row(theme, "menu-fmt-inline-code", s.context_menu_inline_code.clone(), true, Self::on_format_inline_code, cx),
                                Self::render_menu_item_row(theme, "menu-fmt-inline-math", s.context_menu_inline_math.clone(), true, Self::on_format_inline_math, cx),
                                Self::render_menu_item_row(theme, "menu-fmt-comment", s.context_menu_comment.clone(), true, Self::on_format_comment, cx),
                                Self::render_menu_separator(theme),
                                Self::render_menu_item_row(theme, "menu-fmt-clear", s.context_menu_clear_format.clone(), has_selection, Self::on_format_clear, cx),
                            ],
                            px(140.0),
                        ),
                        ContextSubmenu::ParagraphSettings => (
                            vec![
                                Self::render_menu_item_row(theme, "menu-para-bullet", s.context_menu_bullet_list.clone(), true, Self::on_set_bullet_list, cx),
                                Self::render_menu_item_row(theme, "menu-para-numbered", s.context_menu_numbered_list.clone(), true, Self::on_set_numbered_list, cx),
                                Self::render_menu_item_row(theme, "menu-para-task", s.context_menu_task_list.clone(), true, Self::on_set_task_list, cx),
                                Self::render_menu_separator(theme),
                                Self::render_menu_check_row(theme, "menu-para-h1", s.context_menu_heading_1.clone(), is_h1, Self::on_set_heading_1, cx),
                                Self::render_menu_check_row(theme, "menu-para-h2", s.context_menu_heading_2.clone(), is_h2, Self::on_set_heading_2, cx),
                                Self::render_menu_check_row(theme, "menu-para-h3", s.context_menu_heading_3.clone(), is_h3, Self::on_set_heading_3, cx),
                                Self::render_menu_check_row(theme, "menu-para-h4", s.context_menu_heading_4.clone(), is_h4, Self::on_set_heading_4, cx),
                                Self::render_menu_check_row(theme, "menu-para-h5", s.context_menu_heading_5.clone(), is_h5, Self::on_set_heading_5, cx),
                                Self::render_menu_check_row(theme, "menu-para-h6", s.context_menu_heading_6.clone(), is_h6, Self::on_set_heading_6, cx),
                                Self::render_menu_check_row(theme, "menu-para-p", s.context_menu_paragraph.clone(), is_p, Self::on_set_paragraph, cx),
                                Self::render_menu_separator(theme),
                                Self::render_menu_item_row(theme, "menu-para-quote", s.context_menu_quote.clone(), true, Self::on_set_quote, cx),
                            ],
                            px(168.0),
                        ),
                        ContextSubmenu::Insert => (
                            vec![
                                Self::render_menu_item_row(theme, "menu-ins-footnote", s.context_menu_footnote.clone(), true, Self::on_insert_footnote, cx),
                                Self::render_menu_item_row(theme, "menu-ins-table", s.context_menu_table.clone(), true, Self::on_open_table_insert_dialog, cx),
                                Self::render_menu_item_row(theme, "menu-ins-callout", s.context_menu_callout.clone(), true, Self::on_insert_callout, cx),
                                Self::render_menu_item_row(theme, "menu-ins-break", s.context_menu_thematic_break.clone(), true, Self::on_insert_thematic_break, cx),
                                Self::render_menu_separator(theme),
                                Self::render_menu_item_row(theme, "menu-ins-code", s.context_menu_code_block.clone(), true, Self::on_insert_code_block, cx),
                                Self::render_menu_item_row(theme, "menu-ins-math", s.context_menu_math_block.clone(), true, Self::on_insert_math_block, cx),
                                Self::render_menu_item_row(theme, "menu-ins-mermaid", s.context_menu_mermaid.clone(), true, Self::on_insert_mermaid, cx),
                            ],
                            px(196.0),
                        ),
                    };

                    let submenu_panel_el = menu_panel(c, d)
                        .id("editor-context-menu-submenu")
                        .absolute()
                        .left(panel_x + panel_width + px(d.context_menu_submenu_gap))
                        .top(panel_y + y_offset)
                        .w(px(d.context_menu_submenu_width.max(160.0)))
                        .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                            cx.stop_propagation()
                        })
                        .on_hover(cx.listener(move |this, hovered, _window, cx| {
                            if *hovered {
                                this.set_context_menu_submenu_hover(Some(sub), true, cx);
                            } else {
                                this.set_context_menu_submenu_hover(None, true, cx);
                            }
                        }))
                        .children(submenu_items);

                    let bridge_el = div()
                        .id("editor-context-menu-submenu-bridge")
                        .absolute()
                        .left(panel_x + panel_width - px(4.0))
                        .top(panel_y)
                        .w(px(d.context_menu_submenu_gap + 8.0))
                        .h(px(320.0))
                        .on_hover(cx.listener(move |this, hovered, _window, cx| {
                            if *hovered {
                                this.set_context_menu_submenu_hover(Some(sub), true, cx);
                            }
                        }));

                    (submenu_panel_el, bridge_el)
                });

                let mut overlay = overlay()
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
                            .child(Self::render_menu_item_row(
                                theme,
                                "editor-context-menu-cut",
                                s.context_menu_cut.clone(),
                                has_selection,
                                Self::on_context_menu_cut,
                                cx,
                            ))
                            .child(Self::render_menu_item_row(
                                theme,
                                "editor-context-menu-copy",
                                s.context_menu_copy.clone(),
                                has_selection,
                                Self::on_context_menu_copy,
                                cx,
                            ))
                            .child(Self::render_menu_item_row(
                                theme,
                                "editor-context-menu-paste",
                                s.context_menu_paste.clone(),
                                true,
                                Self::on_context_menu_paste,
                                cx,
                            ))
                            .child(Self::render_menu_item_row(
                                theme,
                                "editor-context-menu-paste-plain",
                                s.context_menu_paste_plain.clone(),
                                true,
                                Self::on_context_menu_paste_plain,
                                cx,
                            ))
                            .child(Self::render_menu_item_row(
                                theme,
                                "editor-context-menu-select-all",
                                s.context_menu_select_all.clone(),
                                true,
                                Self::on_context_menu_select_all,
                                cx,
                            ))
                            .child(Self::render_menu_separator(theme))
                            .child(Self::render_submenu_trigger(
                                theme,
                                "editor-context-menu-text-format",
                                s.context_menu_text_format.clone(),
                                *active_submenu == Some(crate::editor::panes::document_pane::context_menu::ContextSubmenu::TextFormat),
                                crate::editor::panes::document_pane::context_menu::ContextSubmenu::TextFormat,
                                cx,
                            ))
                            .child(Self::render_submenu_trigger(
                                theme,
                                "editor-context-menu-paragraph-settings",
                                s.context_menu_paragraph_settings.clone(),
                                *active_submenu == Some(crate::editor::panes::document_pane::context_menu::ContextSubmenu::ParagraphSettings),
                                crate::editor::panes::document_pane::context_menu::ContextSubmenu::ParagraphSettings,
                                cx,
                            ))
                            .child(Self::render_submenu_trigger(
                                theme,
                                "editor-context-menu-insert",
                                s.context_menu_insert.clone(),
                                *active_submenu == Some(crate::editor::panes::document_pane::context_menu::ContextSubmenu::Insert),
                                crate::editor::panes::document_pane::context_menu::ContextSubmenu::Insert,
                                cx,
                            )),
                    );

                if let Some((submenu_panel_el, bridge_el)) = submenu {
                    overlay = overlay.child(bridge_el).child(submenu_panel_el);
                }

                Some(deferred(overlay.into_any_element()).into_any_element())
            }
            ContextMenuState::TableAxis {
                position,
                selection,
            } => {
                let panel_x = position.x - origin.x;
                let panel_y = position.y - origin.y;
                let Some(table_block) = self.table_block_by_id(selection.table_block_id, cx) else {
                    return None;
                };
                let table = table_block.read(cx).data.table.clone()?;
                let items = match selection.kind {
                    TableAxis::Column => vec![
                        Self::render_axis_menu_item(
                            theme,
                            "table-axis-insert-column-left",
                            s.table_axis_insert_column_left.clone(),
                            true,
                            false,
                            Self::on_insert_table_column_left,
                            cx,
                        ),
                        Self::render_axis_menu_item(
                            theme,
                            "table-axis-insert-column-right",
                            s.table_axis_insert_column_right.clone(),
                            true,
                            false,
                            Self::on_insert_table_column_right,
                            cx,
                        ),
                        Self::render_axis_menu_item(
                            theme,
                            "table-axis-duplicate-column",
                            s.table_axis_duplicate_column.clone(),
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
                    TableAxis::Row => {
                        let mut items: Vec<AnyElement> = vec![
                            Self::render_axis_menu_item(
                                theme,
                                "table-axis-insert-row-above",
                                s.table_axis_insert_row_above.clone(),
                                true,
                                false,
                                Self::on_insert_table_row_above,
                                cx,
                            ),
                            Self::render_axis_menu_item(
                                theme,
                                "table-axis-insert-row-below",
                                s.table_axis_insert_row_below.clone(),
                                true,
                                false,
                                Self::on_insert_table_row_below,
                                cx,
                            ),
                            Self::render_axis_menu_item(
                                theme,
                                "table-axis-duplicate-row",
                                s.table_axis_duplicate_row.clone(),
                                true,
                                false,
                                Self::on_duplicate_table_row,
                                cx,
                            ),
                            div()
                                .mx(px(d.menu_separator_margin_x))
                                .my(px(d.menu_separator_margin_y))
                                .h(px(d.menu_separator_height))
                                .bg(c.dialog_border)
                                .into_any_element(),
                        ];
                        // The header row (visual index 0) shares the normal row
                        // menu, with its Header Row styling toggle added on top.
                        if selection.index == 0 {
                            let headers_shown =
                                splitype_infra::config::settings::SettingsStore::get(cx)
                                    .markdown
                                    .show_table_headers;
                            items.push(
                                menu_item("table-header-toggle", c, d)
                                    .justify_between()
                                    .gap(px(d.menu_item_padding_x))
                                    .text_size(px(d.menu_text_size))
                                    .font_weight(t.dialog_body_weight.to_font_weight())
                                    .text_color(c.dialog_secondary_button_text)
                                    .child(s.table_header_row.clone())
                                    .children(headers_shown.then(|| {
                                        svg()
                                            .path("icons/titlebar/app_menu/checkmark.svg")
                                            .size(px(14.0))
                                            .text_color(c.dialog_primary_button_bg)
                                    }))
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
                    deferred(
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
                                    .left(panel_x)
                                    .top(panel_y)
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
                    .into_any_element(),
                )
            }
        }
    }

    /// Renders the interactive Table Size Matrix Picker popup.
    pub(crate) fn render_table_size_picker_overlay(
        &self,
        theme: &Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let picker = self.table_size_picker.as_ref()?;
        let c = &theme.colors;
        let d = &theme.dimensions;

        let table_block_id = picker.table_block_id;
        let origin = self.panel_rect.map(|rect| rect.origin).unwrap_or_default();
        let pos_x = picker.position.x - origin.x;
        let pos_y = picker.position.y - origin.y;

        let panel_width = 176.0_f32;
        let panel_height = 240.0_f32;

        let viewport = window.viewport_size();
        let max_x = f32::from(viewport.width) - panel_width - 16.0;
        let max_y = f32::from(viewport.height) - panel_height - 16.0;

        let panel_x = px(f32::from(pos_x - px(panel_width)).clamp(8.0, max_x.max(8.0)));
        let panel_y = px(f32::from(pos_y + px(10.0)).clamp(8.0, max_y.max(8.0)));

        let max_matrix_rows = 8usize;
        let max_matrix_cols = 6usize;

        let current_rows = picker.current_rows;
        let current_cols = picker.current_cols;

        let display_rows = picker
            .hovered_rows
            .unwrap_or(picker.current_rows)
            .clamp(1, max_matrix_rows);
        let display_cols = picker
            .hovered_cols
            .unwrap_or(picker.current_cols)
            .clamp(1, max_matrix_cols);

        use splitype_ui::table_matrix_picker::{render_matrix_dimension_indicator, MatrixCellColors};
        let colors = MatrixCellColors::from_theme(theme);
        let top_indicator = render_matrix_dimension_indicator(display_rows, display_cols, "Row", "Column", theme);

        let mut grid_rows = Vec::with_capacity(max_matrix_rows);
        for r in 0..max_matrix_rows {
            let row_num = r + 1;
            let mut row_cells = Vec::with_capacity(max_matrix_cols);
            for col in 0..max_matrix_cols {
                let col_num = col + 1;
                let is_in_current = r < current_rows && col < current_cols;
                let is_in_hovered = if let (Some(h_r), Some(h_c)) = (picker.hovered_rows, picker.hovered_cols) {
                    r < h_r && col < h_c
                } else {
                    false
                };

                let cell_bg = match (is_in_current, is_in_hovered) {
                    (true, true) => colors.overlap,
                    (false, true) => colors.hover_only,
                    (true, false) => colors.current_only,
                    (false, false) => colors.inactive,
                };

                let cell = div()
                    .id(ElementId::Name(format!("table-size-cell-{}-{}", r, col).into()))
                    .size(px(20.0))
                    .rounded(px(3.0))
                    .bg(cell_bg)
                    .cursor_pointer()
                    .on_hover(cx.listener(move |editor, hovered: &bool, _window, cx| {
                        if *hovered {
                            editor.set_table_size_picker_hover(Some(row_num), Some(col_num), cx);
                        }
                    }))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |editor, _event, _window, cx| {
                            editor.resize_table(table_block_id, row_num, col_num, cx);
                        }),
                    );
                row_cells.push(cell);
            }
            grid_rows.push(
                div()
                    .flex()
                    .gap(px(3.0))
                    .children(row_cells),
            );
        }

        let matrix_grid = div()
            .id("table-size-matrix-grid")
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(3.0))
            .pt(px(6.0))
            .on_hover(cx.listener(|editor, hovered: &bool, _window, cx| {
                if !*hovered {
                    editor.set_table_size_picker_hover(None, None, cx);
                }
            }))
            .children(grid_rows);

        Some(
            deferred(
                overlay()
                    .id("table-size-picker-overlay")
                    .occlude()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|editor, _event, _window, cx| {
                            editor.close_table_size_picker(cx);
                        }),
                    )
                    .child(
                        div()
                            .id("table-size-picker-panel")
                            .absolute()
                            .left(panel_x)
                            .top(panel_y)
                            .p(px(12.0))
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .bg(c.dialog_surface)
                            .border(px(d.dialog_border_width))
                            .border_color(c.dialog_border)
                            .rounded(px(d.menu_panel_radius))
                            .shadow_lg()
                            .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                                cx.stop_propagation();
                            })
                            .child(top_indicator)
                            .child(matrix_grid),
                    )
                    .into_any_element(),
            )
            .into_any_element(),
        )
    }
}
