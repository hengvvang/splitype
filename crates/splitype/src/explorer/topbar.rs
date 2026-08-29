use gpui::*;

use crate::app::shell::Shell;
use workspace::PanelId;
use workspace::panel_topbar_icon;
use theme::Theme;
use splitter::SplitAxis;
use ui::button::{icon_chip_button, small_pill_button, toolbar_icon_size};
use ui::topbar::topbar_container;

impl Shell {
    /// Top bar of an Explorer area: type selector and split/close controls.
    pub(crate) fn render_explorer_topbar(
        &self,
        panel_id: PanelId,
        kind: workspace::WindowPanelKind,
        theme: &Theme,
        leaf_count: usize,
        is_maximized: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let shell = cx.entity().downgrade();

        let type_shell = shell.clone();
        let type_button = small_pill_button(c, d)
            .id(("panel-topbar-type", panel_id.0))
            .text_size(px(12.0))
            .text_color(c.text_default)
            .child(kind.name().to_string())
            .on_click(move |_event, _window, cx| {
                let _ = type_shell.update(cx, |shell, cx| {
                    shell.panels.layout.toggle_dropdown(panel_id.0);
                    cx.notify();
                });
            });

        let btn_icon_size = toolbar_icon_size(d.topbar_height);

        let split_h_shell = shell.clone();
        let split_h_button = icon_chip_button(c, d)
            .id(("panel-topbar-split-h", panel_id.0))
            .child(
                svg()
                    .path(panel_topbar_icon(kind, "split-h"))
                    .size(px(btn_icon_size))
                    .text_color(c.dialog_muted),
            )
            .on_click(move |_event, _window, cx| {
                let _ = split_h_shell.update(cx, |shell, cx| {
                    // Same-kind split; Editor panels deep-copy their tabs.
                    shell.split_panel(panel_id, SplitAxis::Horizontal, 0.5, true, cx);
                    cx.notify();
                });
            });

        let split_v_shell = shell.clone();
        let split_v_button = icon_chip_button(c, d)
            .id(("panel-topbar-split-v", panel_id.0))
            .child(
                svg()
                    .path(panel_topbar_icon(kind, "split-v"))
                    .size(px(btn_icon_size))
                    .text_color(c.dialog_muted),
            )
            .on_click(move |_event, _window, cx| {
                let _ = split_v_shell.update(cx, |shell, cx| {
                    // Same-kind split; Editor panels deep-copy their tabs.
                    shell.split_panel(panel_id, SplitAxis::Vertical, 0.5, true, cx);
                    cx.notify();
                });
            });

        let mut actions = div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .child(split_v_button)
            .child(split_h_button);

        if leaf_count > 1 {
            let max_shell = shell.clone();
            let max_button = icon_chip_button(c, d)
                .id(("panel-topbar-max", panel_id.0))
                .child(
                    svg()
                        .path(if is_maximized {
                            panel_topbar_icon(kind, "restore")
                        } else {
                            panel_topbar_icon(kind, "maximize")
                        })
                        .size(px(btn_icon_size))
                        .text_color(c.dialog_muted),
                )
                .on_click(move |_event, _window, cx| {
                    let _ = max_shell.update(cx, |shell, cx| {
                        shell.panels.layout.toggle_maximize(panel_id.0);
                        cx.notify();
                    });
                });

            let close_shell = shell.clone();
            let close_button = icon_chip_button(c, d)
                .id(("panel-topbar-close", panel_id.0))
                .child(
                    svg()
                        .path(panel_topbar_icon(kind, "close"))
                        .size(px(btn_icon_size))
                        .text_color(c.dialog_muted),
                )
                .on_click(move |_event, _window, cx| {
                    let _ = close_shell.update(cx, |shell, cx| {
                        shell.close_panel(panel_id, cx);
                        cx.notify();
                    });
                });

            actions = actions.child(max_button).child(close_button);
        }

        topbar_container(c, d.topbar_height, 8.0)
            .id(("panel-topbar", panel_id.0))
            .child(div().flex().items_center().gap(px(8.0)).child(type_button))
            .child(div().flex().items_center().gap(px(6.0)).child(actions))
            .into_any_element()
    }
}
