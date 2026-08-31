//! Context and dropdown menus for split pane borders and view mode toggles.

use gpui::*;
use splitter::SplitAxis;
use ui::popover::menu_panel;

use crate::editor::Editor;
use core_contracts::PaneId;
use theme::Theme;

impl Editor {
    pub(crate) fn render_editor_pane_border_menu(
        &mut self,
        _theme: &Theme,
        _strings: &config::language::I18nStrings,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let border_menu = self.session.root.active_border_menu?;
        let editor = cx.entity().downgrade();
        let split_id = border_menu.split_id;
        let theme = cx.global::<theme::ThemeManager>().current().clone();
        let menu_style = workspace::border_menu_style(&theme);

        let split_h_ed = editor.clone();
        let split_h: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = split_h_ed.update(app, |ed, cx| {
                ed.split_pane_with_ratio(split_id, SplitAxis::Horizontal, 0.5);
                ed.session_mut().root.active_border_menu = None;
                cx.notify();
            });
        });
        let split_v_ed = editor.clone();
        let split_v: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = split_v_ed.update(app, |ed, cx| {
                ed.split_pane_with_ratio(split_id, SplitAxis::Vertical, 0.5);
                ed.session_mut().root.active_border_menu = None;
                cx.notify();
            });
        });
        let swap_ed = editor.clone();
        let swap: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = swap_ed.update(app, |ed, cx| {
                ed.swap_pane_split_sides(split_id);
                cx.notify();
            });
        });
        let close_ed = editor.clone();
        let close: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = close_ed.update(app, |ed, cx| {
                ed.close_pane(split_id);
                ed.session_mut().root.active_border_menu = None;
                cx.notify();
            });
        });
        let dismiss_ed = editor.clone();
        let dismiss: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = dismiss_ed.update(app, |ed, cx| {
                ed.session_mut().root.active_border_menu = None;
                cx.notify();
            });
        });

        Some(splitter::interaction::render_border_menu(
            border_menu.position,
            vec![
                splitter::interaction::BorderMenuItem {
                    label: "Split Horizontally",
                    on_activate: split_h,
                },
                splitter::interaction::BorderMenuItem {
                    label: "Split Vertically",
                    on_activate: split_v,
                },
                splitter::interaction::BorderMenuItem {
                    label: "Swap Panels",
                    on_activate: swap,
                },
                splitter::interaction::BorderMenuItem {
                    label: "Close Panel",
                    on_activate: close,
                },
            ],
            &menu_style,
            dismiss,
        ))
    }

    #[allow(dead_code)]
    pub(crate) fn render_editor_pane_dropdown_menu(
        &mut self,
        pane_id: impl Into<PaneId>,
        current_kind: crate::session::PaneKindId,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let pane_id = pane_id.into();
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let editor = cx.entity().downgrade();

        let available_descriptors =
            core_contracts::PaneRegistry::global().lock().unwrap().all_descriptors();

        menu_panel(c, d)
            .id(("inner-pane-dropdown-overlay", pane_id.0))
            .absolute()
            .occlude()
            .left(px(0.0))
            .bottom(px(0.0))
            .w(px(d.menu_panel_width))
            .children(available_descriptors.into_iter().enumerate().map(|(idx, descriptor)| {
                let kind = descriptor.kind();
                let name = descriptor.display_name();
                let is_current = kind == current_kind;
                let option_editor = editor.clone();
                div()
                    .id(("inner-pane-type-opt", idx))
                    .w_full()
                    .h(px(d.menu_item_height))
                    .px(px(d.menu_item_padding_x))
                    .flex()
                    .items_center()
                    .justify_between()
                    .rounded(px(d.menu_item_radius))
                    .bg(if is_current {
                        c.panel_row_selected
                    } else {
                        c.dialog_surface
                    })
                    .hover(|this| this.bg(c.panel_row_hover))
                    .cursor_pointer()
                    .text_size(px(d.menu_text_size))
                    .font_weight(t.dialog_body_weight.to_font_weight())
                    .text_color(c.dialog_secondary_button_text)
                    .child(div().child(name.to_string()))
                    .child(if is_current {
                        svg()
                            .path("icons/editor/bottombar/checkmark.svg")
                            .size(px(14.0))
                            .text_color(c.dialog_primary_button_bg)
                            .into_any_element()
                    } else {
                        div().w(px(13.0)).into_any_element()
                    })
                    .on_click(move |_event, _window, cx| {
                        let _ = option_editor.update(cx, |ed, cx| {
                            ed.change_pane_kind(pane_id, kind);
                            cx.notify();
                        });
                    })
            }))
            .into_any_element()
    }
}

