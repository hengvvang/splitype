//! Panel tile card and panel type selector dropdown menu.

use gpui::*;

use crate::shell::Shell;
use config::language::I18nStrings;
use theme::Theme;
use splitter::tree::NodeId;
use ui::menu_item::menu_item;
use ui::popover::menu_panel;
use window::panel_topbar_icon;

impl Shell {
    pub(crate) fn render_window_panel_tile(
        &mut self,
        leaf_id: NodeId,
        kind: window::PanelKind,
        theme: &Theme,
        strings: &I18nStrings,
        leaf_count: usize,
        is_maximized: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let gap = d.panel_tile_gap;
        let radius = d.panel_tile_radius;

        let panel_id = window::PanelId(leaf_id);
        let render_ctx = window::PanelRenderContext {
            panel_id,
            leaf_count,
            is_maximized,
            theme,
            strings,
        };

        if !self.panel_views.contains_key(&panel_id) {
            self.sync_panel_kind(panel_id, kind == window::PanelKind::new("editor"), cx);
        }

        let panel_card: AnyElement = if let Some(view) = self.panel_views.get_mut(&panel_id) {
            let rendered = view.render(&render_ctx, _window, cx);
            if view.kind() == window::PanelKind::new("editor") {
                rendered
            } else {
                let card = div()
                    .id(("panel-card", leaf_id))
                    .w_full()
                    .h_full()
                    .flex()
                    .flex_col()
                    .relative()
                    .overflow_hidden()
                    .rounded(px(radius))
                    .bg(c.dialog_surface)
                    .border(px(d.dialog_border_width))
                    .border_color(c.dialog_border)
                    .shadow_lg()
                    .child(rendered);
                card.into_any_element()
            }
        } else {
            div().into_any_element()
        };

        let tile_focus = cx.entity().downgrade();
        let mut wrapped = div()
            .id(("panel-wrapper", leaf_id))
            .w_full()
            .h_full()
            .min_w(px(0.0))
            .min_h(px(0.0))
            .p(px(gap))
            .relative()
            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                let _ = tile_focus.update(cx, |shell, cx| {
                    shell.panels.layout.focused_leaf = Some(leaf_id);
                    if kind == window::PanelKind::new("editor") {
                        shell.panels.layout.activate_leaf(leaf_id);
                    }
                    cx.notify();
                });
            })
            .child(panel_card);

        let titlebar_height = ui::custom_titlebar::custom_titlebar_height_for_target_os(
            std::env::consts::OS,
            Decorations::Server,
            &theme.dimensions,
        );

        let shell_corner = cx.entity().downgrade();
        let corner_handles = splitter::interaction::corner_drag_handles(
            "panel-corner",
            leaf_id,
            gap,
            20.0,
            false,
            false,
            move |modifier, pos, cx| {
                let _ = shell_corner.update(cx, |shell, cx| {
                    let body_pos = point(pos.x, px((f32::from(pos.y) - titlebar_height).max(0.0)));
                    shell
                        .panels
                        .layout
                        .start_corner_drag(leaf_id, body_pos, modifier);
                    cx.notify();
                });
            },
        );
        wrapped = wrapped.child(corner_handles);

        let dropdown_open = self
            .panels
            .layout
            .tree
            .find_leaf(leaf_id)
            .is_some_and(|p| p.open_dropdown);
        if dropdown_open {
            let menu = self.render_panel_type_dropdown_menu(leaf_id, kind, theme, cx);
            wrapped = wrapped.child(menu);
        }

        wrapped.into_any_element()
    }

    pub(crate) fn render_panel_type_dropdown_menu(
        &mut self,
        leaf_id: NodeId,
        current_kind: window::PanelKind,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let shell = cx.entity().downgrade();

        let registry = window::PanelRegistry::global().lock().unwrap();
        let available_descriptors = registry.all_descriptors();

        menu_panel(c, d)
            .id(("panel-dropdown-overlay", leaf_id))
            .absolute()
            .occlude()
            .top(px(28.0))
            .left(px(8.0))
            .w(px(d.menu_panel_width))
            .children(available_descriptors.into_iter().enumerate().map(|(idx, desc)| {
                let kind_id = desc.kind();
                let is_current = kind_id == current_kind;
                let option_shell = shell.clone();
                let display_name = desc.display_name();
                menu_item(("panel-type-opt", idx), c, d)
                    .w_full()
                    .justify_between()
                    .bg(if is_current {
                        c.panel_row_selected
                    } else {
                        c.dialog_surface
                    })
                    .text_size(px(d.menu_text_size))
                    .font_weight(t.dialog_button_weight.to_font_weight())
                    .text_color(c.dialog_secondary_button_text)
                    .child(display_name)
                    .child(if is_current {
                        svg()
                            .path(panel_topbar_icon(current_kind, "check"))
                            .size(px(13.0))
                            .text_color(c.dialog_primary_button_bg)
                            .into_any_element()
                    } else {
                        div().w(px(13.0)).into_any_element()
                    })
                    .on_click(move |_event, _window, cx| {
                        let _ = option_shell.update(cx, |shell, cx| {
                            shell.change_panel_kind(leaf_id, kind_id, cx);
                            cx.notify();
                        });
                    })
                    .into_any_element()
            }))
            .into_any_element()
    }
}
