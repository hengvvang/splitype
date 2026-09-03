//! Context menu rendering and action dispatch for the WYSIWYG editor pane.
//!
//! Replicates the cascading context menu and table axis menu from splitype-old.

use gpui::*;
use theme::Theme;
use ui::menu_item::menu_item;
use ui::popover::{menu_panel, overlay};

use crate::model::block::state::InlineFormat;
use crate::pane::controller::{ContextSubmenu, WysiwygContextMenuState, WysiwygDocumentController};
use markdown_parser::block::table::{TableAxis, TableColumnAlignment};
use markdown_parser::parse::BlockKind;

type ContextMenuActionHandler =
    Box<dyn Fn(&mut WysiwygDocumentController, &mut Window, &mut Context<WysiwygDocumentController>)>;

pub fn render_wysiwyg_context_menu(
    controller: &WysiwygDocumentController,
    state: &WysiwygContextMenuState,
    origin: Point<Pixels>,
    pane_size: Size<Pixels>,
    theme: &Theme,
    _window: &mut Window,
    cx: &mut Context<WysiwygDocumentController>,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    let is_zh = cx.has_global::<config::language::I18nManager>()
        && cx
            .global::<config::language::I18nManager>()
            .current_language_id()
            .starts_with("zh");
    let tr = |zh: &'static str, en: &'static str| if is_zh { zh } else { en };

    let pane_width = f32::from(pane_size.width);
    let pane_height = f32::from(pane_size.height);

    let make_separator = || {
        div()
            .h(px(1.0))
            .bg(c.dialog_border)
            .my(px(2.0))
            .into_any_element()
    };

    let make_item_base = |id: &'static str,
                          label: &'static str,
                          shortcut: Option<&'static str>,
                          enabled: bool,
                          danger: bool,
                          closes_submenu: bool,
                          on_click: ContextMenuActionHandler| {
        if enabled {
            let mut el = menu_item(id, c, d)
                .justify_between()
                .child(
                    div()
                        .text_size(px(t.text_size * 0.85))
                        .text_color(if danger {
                            c.dialog_danger_button_bg
                        } else {
                            c.text_default
                        })
                        .child(label),
                )
                .children(shortcut.map(|s| {
                    div()
                        .text_size(px(t.text_size * 0.75))
                        .text_color(c.dialog_muted)
                        .child(s)
                }));
            if closes_submenu {
                el = el.on_hover(cx.listener(|this, hovered, _window, cx| {
                    if *hovered {
                        this.set_context_menu_submenu(None, cx);
                    }
                }));
            }
            el.on_mouse_down(MouseButton::Left, cx.listener(move |this, _event, window, cx| {
                cx.stop_propagation();
                on_click(this, window, cx);
                this.close_context_menu(cx);
            }))
            .into_any_element()
        } else {
            let mut el = div()
                .id(id)
                .h(px(d.menu_item_height))
                .px(px(d.menu_item_padding_x))
                .flex()
                .items_center()
                .justify_between()
                .rounded(px(d.menu_item_radius))
                .text_size(px(t.text_size * 0.85))
                .text_color(c.dialog_muted)
                .child(label)
                .children(shortcut.map(|s| {
                    div()
                        .text_size(px(t.text_size * 0.75))
                        .text_color(c.dialog_muted)
                        .child(s)
                }));
            if closes_submenu {
                el = el.on_hover(cx.listener(|this, hovered, _window, cx| {
                    if *hovered {
                        this.set_context_menu_submenu(None, cx);
                    }
                }));
            }
            el.into_any_element()
        }
    };

    let make_item = |id: &'static str,
                     label: &'static str,
                     shortcut: Option<&'static str>,
                     enabled: bool,
                     danger: bool,
                     on_click: ContextMenuActionHandler| {
        make_item_base(id, label, shortcut, enabled, danger, false, on_click)
    };

    let make_main_item = |id: &'static str,
                          label: &'static str,
                          shortcut: Option<&'static str>,
                          enabled: bool,
                          danger: bool,
                          on_click: ContextMenuActionHandler| {
        make_item_base(id, label, shortcut, enabled, danger, true, on_click)
    };

    let make_submenu_trigger = |id: &'static str,
                                label: &'static str,
                                submenu_kind: ContextSubmenu,
                                is_active: bool| {
        menu_item(id, c, d)
            .justify_between()
            .bg(if is_active {
                c.panel_row_hover
            } else {
                c.dialog_surface
            })
            .child(
                div()
                    .text_size(px(t.text_size * 0.85))
                    .text_color(c.text_default)
                    .child(label),
            )
            .child(
                svg()
                    .path("icons/editor/context_menu/chevron-right.svg")
                    .size(px(14.0))
                    .text_color(c.dialog_muted),
            )
            .on_hover(cx.listener(move |this, hovered, _window, cx| {
                if *hovered {
                    this.set_context_menu_submenu(Some(submenu_kind), cx);
                }
            }))
            .into_any_element()
    };

    let make_check_item = |id: &'static str,
                           label: &'static str,
                           shortcut: Option<&'static str>,
                           is_checked: bool,
                           on_click: ContextMenuActionHandler| {
        menu_item(id, c, d)
            .justify_between()
            .child(
                div()
                    .text_size(px(t.text_size * 0.85))
                    .text_color(c.text_default)
                    .child(label),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .children(shortcut.map(|s| {
                        div()
                            .text_size(px(t.text_size * 0.75))
                            .text_color(c.dialog_muted)
                            .child(s)
                    }))
                    .children(is_checked.then(|| {
                        svg()
                            .path("icons/titlebar/app_menu/checkmark.svg")
                            .size(px(14.0))
                            .text_color(c.dialog_primary_button_bg)
                    })),
            )
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _event, window, cx| {
                cx.stop_propagation();
                on_click(this, window, cx);
                this.close_context_menu(cx);
            }))
            .into_any_element()
    };

    match state {
        WysiwygContextMenuState::Edit {
            position,
            target_entity_id,
            active_submenu,
        } => {
            let target_id = *target_entity_id;
            let panel_x = position.x - origin.x;
            let panel_y = position.y - origin.y;
            let panel_width = 185.0_f32;
            let max_x = (pane_width - panel_width - 16.0).max(8.0);
            let max_y = (pane_height - 290.0 - 16.0).max(8.0);
            let panel_left = px(f32::from(panel_x).clamp(8.0, max_x));
            let panel_top = px(f32::from(panel_y).clamp(8.0, max_y));

            let has_selection = controller
                .active_entity
                .as_ref()
                .map(|b| !b.read(cx).selected_range.is_empty())
                .unwrap_or(false);
            let active_kind = target_id
                .and_then(|id| controller.document.as_ref()?.block_entity_by_id(id))
                .or_else(|| controller.active_entity.clone())
                .map(|b| b.read(cx).kind().clone());

            let is_h1 = matches!(active_kind, Some(BlockKind::Heading { level: 1 }));
            let is_h2 = matches!(active_kind, Some(BlockKind::Heading { level: 2 }));
            let is_h3 = matches!(active_kind, Some(BlockKind::Heading { level: 3 }));
            let is_h4 = matches!(active_kind, Some(BlockKind::Heading { level: 4 }));
            let is_h5 = matches!(active_kind, Some(BlockKind::Heading { level: 5 }));
            let is_h6 = matches!(active_kind, Some(BlockKind::Heading { level: 6 }));
            let is_p = matches!(active_kind, Some(BlockKind::Paragraph)) || active_kind.is_none();

            let target_entity = target_id.or_else(|| controller.active_entity.as_ref().map(|b| b.entity_id()));

            let submenu_rendered = active_submenu.map(|sub| {
                let (items, y_offset) = match sub {
                    ContextSubmenu::TextFormat => (
                        vec![
                            make_item(
                                "menu-fmt-bold",
                                tr("粗体", "Bold"),
                                Some("Ctrl+B"),
                                true,
                                false,
                                Box::new(|this, _window, cx| {
                                    this.toggle_active_format(InlineFormat::Bold, cx);
                                }),
                            ),
                            make_item(
                                "menu-fmt-italic",
                                tr("斜体", "Italic"),
                                Some("Ctrl+I"),
                                true,
                                false,
                                Box::new(|this, _window, cx| {
                                    this.toggle_active_format(InlineFormat::Italic, cx);
                                }),
                            ),
                            make_item(
                                "menu-fmt-strike",
                                tr("删除线", "Strikethrough"),
                                None,
                                true,
                                false,
                                Box::new(|this, _window, cx| {
                                    this.toggle_active_format(InlineFormat::Strikethrough, cx);
                                }),
                            ),
                            make_item(
                                "menu-fmt-highlight",
                                tr("高亮", "Highlight"),
                                None,
                                true,
                                false,
                                Box::new(|this, _window, cx| {
                                    this.wrap_active_selection("==", "==", cx);
                                }),
                            ),
                            make_separator(),
                            make_item(
                                "menu-fmt-code",
                                tr("行内代码", "Inline Code"),
                                Some("Ctrl+E"),
                                true,
                                false,
                                Box::new(|this, _window, cx| {
                                    this.toggle_active_format(InlineFormat::Code, cx);
                                }),
                            ),
                            make_item(
                                "menu-fmt-math",
                                tr("行内公式", "Inline Math"),
                                Some("Ctrl+M"),
                                true,
                                false,
                                Box::new(|this, _window, cx| {
                                    this.wrap_active_selection("$", "$", cx);
                                }),
                            ),
                            make_item(
                                "menu-fmt-comment",
                                tr("注释", "Comment"),
                                None,
                                true,
                                false,
                                Box::new(|this, _window, cx| {
                                    this.wrap_active_selection("<!-- ", " -->", cx);
                                }),
                            ),
                            make_separator(),
                            make_item(
                                "menu-fmt-clear",
                                tr("清除格式", "Clear Format"),
                                None,
                                has_selection,
                                false,
                                Box::new(|this, _window, cx| {
                                    this.clear_active_selection_format(cx);
                                }),
                            ),
                        ],
                        px(110.0),
                    ),
                    ContextSubmenu::ParagraphSettings => (
                        vec![
                            make_item(
                                "menu-para-bullet",
                                tr("无序列表", "Bullet List"),
                                None,
                                true,
                                false,
                                Box::new(move |this, _window, cx| {
                                    if let Some(id) = target_entity {
                                        this.convert_target_block(id, BlockKind::BulletListItem, cx);
                                    }
                                }),
                            ),
                            make_item(
                                "menu-para-numbered",
                                tr("有序列表", "Numbered List"),
                                None,
                                true,
                                false,
                                Box::new(move |this, _window, cx| {
                                    if let Some(id) = target_entity {
                                        this.convert_target_block(
                                            id,
                                            BlockKind::NumberedListItem,
                                            cx,
                                        );
                                    }
                                }),
                            ),
                            make_item(
                                "menu-para-task",
                                tr("任务列表", "Task List"),
                                None,
                                true,
                                false,
                                Box::new(move |this, _window, cx| {
                                    if let Some(id) = target_entity {
                                        this.convert_target_block(
                                            id,
                                            BlockKind::TaskListItem { checked: false },
                                            cx,
                                        );
                                    }
                                }),
                            ),
                            make_separator(),
                            make_check_item(
                                "menu-para-h1",
                                tr("标题 1", "Heading 1"),
                                Some("Ctrl+1"),
                                is_h1,
                                Box::new(move |this, _window, cx| {
                                    if let Some(id) = target_entity {
                                        this.convert_target_block(
                                            id,
                                            BlockKind::Heading { level: 1 },
                                            cx,
                                        );
                                    }
                                }),
                            ),
                            make_check_item(
                                "menu-para-h2",
                                tr("标题 2", "Heading 2"),
                                Some("Ctrl+2"),
                                is_h2,
                                Box::new(move |this, _window, cx| {
                                    if let Some(id) = target_entity {
                                        this.convert_target_block(
                                            id,
                                            BlockKind::Heading { level: 2 },
                                            cx,
                                        );
                                    }
                                }),
                            ),
                            make_check_item(
                                "menu-para-h3",
                                tr("标题 3", "Heading 3"),
                                Some("Ctrl+3"),
                                is_h3,
                                Box::new(move |this, _window, cx| {
                                    if let Some(id) = target_entity {
                                        this.convert_target_block(
                                            id,
                                            BlockKind::Heading { level: 3 },
                                            cx,
                                        );
                                    }
                                }),
                            ),
                            make_check_item(
                                "menu-para-h4",
                                tr("标题 4", "Heading 4"),
                                None,
                                is_h4,
                                Box::new(move |this, _window, cx| {
                                    if let Some(id) = target_entity {
                                        this.convert_target_block(
                                            id,
                                            BlockKind::Heading { level: 4 },
                                            cx,
                                        );
                                    }
                                }),
                            ),
                            make_check_item(
                                "menu-para-h5",
                                tr("标题 5", "Heading 5"),
                                None,
                                is_h5,
                                Box::new(move |this, _window, cx| {
                                    if let Some(id) = target_entity {
                                        this.convert_target_block(
                                            id,
                                            BlockKind::Heading { level: 5 },
                                            cx,
                                        );
                                    }
                                }),
                            ),
                            make_check_item(
                                "menu-para-h6",
                                tr("标题 6", "Heading 6"),
                                None,
                                is_h6,
                                Box::new(move |this, _window, cx| {
                                    if let Some(id) = target_entity {
                                        this.convert_target_block(
                                            id,
                                            BlockKind::Heading { level: 6 },
                                            cx,
                                        );
                                    }
                                }),
                            ),
                            make_check_item(
                                "menu-para-p",
                                tr("正文", "Paragraph"),
                                Some("Ctrl+0"),
                                is_p,
                                Box::new(move |this, _window, cx| {
                                    if let Some(id) = target_entity {
                                        this.convert_target_block(id, BlockKind::Paragraph, cx);
                                    }
                                }),
                            ),
                            make_separator(),
                            make_item(
                                "menu-para-quote",
                                tr("引用", "Quote"),
                                None,
                                true,
                                false,
                                Box::new(move |this, _window, cx| {
                                    if let Some(id) = target_entity {
                                        this.convert_target_block(id, BlockKind::Blockquote, cx);
                                    }
                                }),
                            ),
                        ],
                        px(140.0),
                    ),
                    ContextSubmenu::Insert => (
                        vec![
                            make_item(
                                "menu-ins-footnote",
                                tr("脚注", "Footnote"),
                                None,
                                true,
                                false,
                                Box::new(move |this, _window, cx| {
                                    if let Some(id) = target_entity {
                                        this.insert_footnote_after(id, cx);
                                    }
                                }),
                            ),
                            make_item(
                                "menu-ins-table",
                                tr("表格", "Table"),
                                None,
                                true,
                                false,
                                Box::new(move |this, _window, cx| {
                                    if let Some(id) = target_entity {
                                        this.insert_table_after(id, cx);
                                    }
                                }),
                            ),
                            make_item(
                                "menu-ins-callout",
                                tr("提示块", "Callout"),
                                None,
                                true,
                                false,
                                Box::new(move |this, _window, cx| {
                                    if let Some(id) = target_entity {
                                        this.insert_callout_after(id, cx);
                                    }
                                }),
                            ),
                            make_item(
                                "menu-ins-break",
                                tr("分割线", "Thematic Break"),
                                None,
                                true,
                                false,
                                Box::new(move |this, _window, cx| {
                                    if let Some(id) = target_entity {
                                        this.insert_divider_after(id, cx);
                                    }
                                }),
                            ),
                            make_separator(),
                            make_item(
                                "menu-ins-code",
                                tr("代码块", "Code Block"),
                                None,
                                true,
                                false,
                                Box::new(move |this, _window, cx| {
                                    if let Some(id) = target_entity {
                                        this.insert_code_block_after(id, cx);
                                    }
                                }),
                            ),
                            make_item(
                                "menu-ins-math",
                                tr("公式块", "Math Block"),
                                None,
                                true,
                                false,
                                Box::new(move |this, _window, cx| {
                                    if let Some(id) = target_entity {
                                        this.insert_math_block_after(id, cx);
                                    }
                                }),
                            ),
                            make_item(
                                "menu-ins-mermaid",
                                tr("Mermaid 图表", "Mermaid"),
                                None,
                                true,
                                false,
                                Box::new(move |this, _window, cx| {
                                    if let Some(id) = target_entity {
                                        this.insert_mermaid_after(id, cx);
                                    }
                                }),
                            ),
                        ],
                        px(168.0),
                    ),
                };

                let submenu_width = 175.0_f32;
                let is_overflowing_right =
                    f32::from(panel_left) + panel_width + submenu_width + 16.0 > pane_width;
                let submenu_left = if is_overflowing_right {
                    (panel_left - px(submenu_width) - px(d.context_menu_submenu_gap.max(4.0)))
                        .max(px(8.0))
                } else {
                    panel_left + px(panel_width) + px(d.context_menu_submenu_gap.max(4.0))
                };
                let submenu_top = (panel_top + y_offset)
                    .max(px(8.0))
                    .min(px((pane_height - 350.0).max(8.0)));

                let bridge_left = if is_overflowing_right {
                    panel_left - px(d.context_menu_submenu_gap.max(4.0) + 4.0)
                } else {
                    panel_left + px(panel_width) - px(4.0)
                };
                let bridge_top = (panel_top.min(submenu_top) - px(8.0)).max(px(0.0));
                let bridge_height =
                    (panel_top.max(submenu_top) - bridge_top + px(350.0)).max(px(320.0));
                let bridge_el = div()
                    .id("editor-context-menu-submenu-bridge")
                    .absolute()
                    .left(bridge_left)
                    .top(bridge_top)
                    .w(px(d.context_menu_submenu_gap.max(4.0) + 8.0))
                    .h(bridge_height)
                    .occlude()
                    .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                        cx.stop_propagation()
                    });

                let panel_el = menu_panel(c, d)
                    .id("editor-context-menu-submenu")
                    .absolute()
                    .left(submenu_left)
                    .top(submenu_top)
                    .w(px(submenu_width))
                    .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                        cx.stop_propagation()
                    })
                    .children(items);

                (panel_el, bridge_el)
            });

            let main_menu_items = vec![
                make_main_item(
                    "context-menu-cut",
                    tr("剪切", "Cut"),
                    Some("Ctrl+X"),
                    has_selection,
                    false,
                    Box::new(|this, _window, cx| {
                        this.cut_active_selection(cx);
                    }),
                ),
                make_main_item(
                    "context-menu-copy",
                    tr("复制", "Copy"),
                    Some("Ctrl+C"),
                    has_selection,
                    false,
                    Box::new(|this, _window, cx| {
                        this.copy_active_selection(cx);
                    }),
                ),
                make_main_item(
                    "context-menu-paste",
                    tr("粘贴", "Paste"),
                    Some("Ctrl+V"),
                    true,
                    false,
                    Box::new(|this, window, cx| {
                        this.paste_into_active(window, cx);
                    }),
                ),
                make_main_item(
                    "context-menu-paste-plain",
                    tr("纯文本粘贴", "Paste Plain"),
                    None,
                    true,
                    false,
                    Box::new(|this, window, cx| {
                        this.paste_plain_into_active(window, cx);
                    }),
                ),
                make_main_item(
                    "context-menu-select-all",
                    tr("全选", "Select All"),
                    Some("Ctrl+A"),
                    true,
                    false,
                    Box::new(|this, _window, cx| {
                        this.select_all(cx);
                    }),
                ),
                make_separator(),
                make_submenu_trigger(
                    "context-menu-text-format",
                    tr("文本格式", "Text Format"),
                    ContextSubmenu::TextFormat,
                    *active_submenu == Some(ContextSubmenu::TextFormat),
                ),
                make_submenu_trigger(
                    "context-menu-paragraph-settings",
                    tr("段落设置", "Paragraph Settings"),
                    ContextSubmenu::ParagraphSettings,
                    *active_submenu == Some(ContextSubmenu::ParagraphSettings),
                ),
                make_submenu_trigger(
                    "context-menu-insert",
                    tr("插入", "Insert"),
                    ContextSubmenu::Insert,
                    *active_submenu == Some(ContextSubmenu::Insert),
                ),
                make_separator(),
                make_main_item(
                    "context-menu-delete-block",
                    tr("删除块", "Delete Block"),
                    None,
                    true,
                    true,
                    Box::new(move |this, _window, cx| {
                        if let Some(id) = target_entity {
                            this.delete_target_block(id, cx);
                        }
                    }),
                ),
            ];

            let mut container = overlay()
                .id("editor-context-menu-overlay")
                .occlude()
                .on_mouse_down(MouseButton::Left, cx.listener(|this, _event, _window, cx| {
                    this.close_context_menu(cx);
                }))
                .on_mouse_down(MouseButton::Right, cx.listener(|this, _event, _window, cx| {
                    this.close_context_menu(cx);
                }))
                .child(
                    menu_panel(c, d)
                        .id("editor-context-menu-panel")
                        .absolute()
                        .left(panel_left)
                        .top(panel_top)
                        .w(px(panel_width))
                        .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                            cx.stop_propagation()
                        })
                        .children(main_menu_items),
                );

            if let Some((sub_panel, bridge)) = submenu_rendered {
                container = container.child(bridge).child(sub_panel);
            }

            container.into_any_element()
        }
        WysiwygContextMenuState::TableAxis {
            position,
            selection,
        } => {
            let panel_x = position.x - origin.x;
            let panel_y = position.y - origin.y;
            let panel_width = 175.0_f32;
            let panel_left = px(f32::from(panel_x)
                .clamp(8.0, (pane_width - panel_width - 16.0).max(8.0)));
            let panel_top = px(f32::from(panel_y)
                .clamp(8.0, (pane_height - 240.0 - 16.0).max(8.0)));

            let table_block_id = selection.table_block_id;
            let axis_index = selection.index;

            let items = match selection.kind {
                TableAxis::Column => vec![
                    make_item(
                        "table-axis-insert-column-left",
                        tr("向左插入列", "Insert Column Left"),
                        None,
                        true,
                        false,
                        Box::new(move |this, _window, cx| {
                            this.insert_table_column_at_index(table_block_id, axis_index, cx);
                        }),
                    ),
                    make_item(
                        "table-axis-insert-column-right",
                        tr("向右插入列", "Insert Column Right"),
                        None,
                        true,
                        false,
                        Box::new(move |this, _window, cx| {
                            this.insert_table_column_at_index(table_block_id, axis_index + 1, cx);
                        }),
                    ),
                    make_item(
                        "table-axis-duplicate-column",
                        tr("复制列", "Duplicate Column"),
                        None,
                        true,
                        false,
                        Box::new(move |this, _window, cx| {
                            this.duplicate_table_column_at_index(table_block_id, axis_index, cx);
                        }),
                    ),
                    make_separator(),
                    make_item(
                        "table-axis-align-column-left",
                        tr("左对齐", "Align Left"),
                        None,
                        true,
                        false,
                        Box::new(move |this, _window, cx| {
                            this.set_table_column_alignment_at_index(
                                table_block_id,
                                axis_index,
                                TableColumnAlignment::Default,
                                cx,
                            );
                        }),
                    ),
                    make_item(
                        "table-axis-align-column-center",
                        tr("居中对齐", "Align Center"),
                        None,
                        true,
                        false,
                        Box::new(move |this, _window, cx| {
                            this.set_table_column_alignment_at_index(
                                table_block_id,
                                axis_index,
                                TableColumnAlignment::Center,
                                cx,
                            );
                        }),
                    ),
                    make_item(
                        "table-axis-align-column-right",
                        tr("右对齐", "Align Right"),
                        None,
                        true,
                        false,
                        Box::new(move |this, _window, cx| {
                            this.set_table_column_alignment_at_index(
                                table_block_id,
                                axis_index,
                                TableColumnAlignment::Right,
                                cx,
                            );
                        }),
                    ),
                    make_separator(),
                    make_item(
                        "table-axis-delete-column",
                        tr("删除列", "Delete Column"),
                        None,
                        true,
                        true,
                        Box::new(move |this, _window, cx| {
                            this.delete_table_column_at_index(table_block_id, axis_index, cx);
                        }),
                    ),
                ],
                TableAxis::Row => vec![
                    make_item(
                        "table-axis-insert-row-above",
                        tr("向上插入行", "Insert Row Above"),
                        None,
                        true,
                        false,
                        Box::new(move |this, _window, cx| {
                            this.insert_table_row_at_index(table_block_id, axis_index, cx);
                        }),
                    ),
                    make_item(
                        "table-axis-insert-row-below",
                        tr("向下插入行", "Insert Row Below"),
                        None,
                        true,
                        false,
                        Box::new(move |this, _window, cx| {
                            this.insert_table_row_at_index(table_block_id, axis_index + 1, cx);
                        }),
                    ),
                    make_item(
                        "table-axis-duplicate-row",
                        tr("复制行", "Duplicate Row"),
                        None,
                        true,
                        false,
                        Box::new(move |this, _window, cx| {
                            this.duplicate_table_row_at_index(table_block_id, axis_index, cx);
                        }),
                    ),
                    make_separator(),
                    make_item(
                        "table-axis-delete-row",
                        tr("删除行", "Delete Row"),
                        None,
                        true,
                        true,
                        Box::new(move |this, _window, cx| {
                            this.delete_table_row_at_index(table_block_id, axis_index, cx);
                        }),
                    ),
                ],
            };

            overlay()
                .id("editor-table-axis-menu-overlay")
                .occlude()
                .on_mouse_down(MouseButton::Left, cx.listener(|this, _event, _window, cx| {
                    this.close_context_menu(cx);
                }))
                .on_mouse_down(MouseButton::Right, cx.listener(|this, _event, _window, cx| {
                    this.close_context_menu(cx);
                }))
                .child(
                    menu_panel(c, d)
                        .id("editor-table-axis-menu-panel")
                        .absolute()
                        .left(panel_left)
                        .top(panel_top)
                        .w(px(panel_width))
                        .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                            cx.stop_propagation()
                        })
                        .children(items),
                )
                .into_any_element()
        }
    }
}
