//! Top bar of a Settings area: the area type selector and split/close
//! controls.

use gpui::*;

use crate::app::shell::Shell;

use crate::editor::window_layout::panel_topbar_icon;
use crate::infra::theme::Theme;
use crate::splitter::Axis;
use crate::ui::button::{icon_chip_button, small_pill_button};
use crate::ui::topbar::topbar_container;

impl Shell {
    /// Top bar of a Settings area: type selector and split/close controls.
    pub(crate) fn render_settings_topbar(
        &self,
        leaf_id: usize,
        kind: crate::app::window_panels::WindowPanelKind,
        theme: &Theme,
        leaf_count: usize,
        is_maximized: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let editor = cx.entity().downgrade();

        let type_editor = editor.clone();
        let type_button = small_pill_button(c, d)
            .id(("area-topbar-type", leaf_id))
            .text_size(px(12.0))
            .text_color(c.text_default)
            .child(kind.name().to_string())
            .on_click(move |_event, _window, cx| {
                let _ = type_editor.update(cx, |ed, cx| {
                    ed.panels.layout.toggle_dropdown(leaf_id);
                    cx.notify();
                });
            });

        let split_h_editor = editor.clone();
        let split_h_button = icon_chip_button(c, d)
            .id(("area-topbar-split-h", leaf_id))
            .child(
                svg()
                    .path(panel_topbar_icon(kind, "split-h"))
                    .size(px(d.topbar_height * 0.5 + 2.0))
                    .text_color(c.dialog_muted),
            )
            .on_click(move |_event, _window, cx| {
                let _ = split_h_editor.update(cx, |ed, cx| {
                    // Same-kind split; Editor panels deep-copy their tabs.
                    ed.split_panel(leaf_id, Axis::Horizontal, 0.5, true, cx);
                    cx.notify();
                });
            });

        let split_v_editor = editor.clone();
        let split_v_button = icon_chip_button(c, d)
            .id(("area-topbar-split-v", leaf_id))
            .child(
                svg()
                    .path(panel_topbar_icon(kind, "split-v"))
                    .size(px(d.topbar_height * 0.5 + 2.0))
                    .text_color(c.dialog_muted),
            )
            .on_click(move |_event, _window, cx| {
                let _ = split_v_editor.update(cx, |ed, cx| {
                    // Same-kind split; Editor panels deep-copy their tabs.
                    ed.split_panel(leaf_id, Axis::Vertical, 0.5, true, cx);
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
            let max_editor = editor.clone();
            let max_button = icon_chip_button(c, d)
                .id(("area-topbar-max", leaf_id))
                .child(
                    svg()
                        .path(if is_maximized {
                            panel_topbar_icon(kind, "restore")
                        } else {
                            panel_topbar_icon(kind, "maximize")
                        })
                        .size(px(d.topbar_height * 0.5 - 2.0))
                        .text_color(c.dialog_muted),
                )
                .on_click(move |_event, _window, cx| {
                    let _ = max_editor.update(cx, |ed, cx| {
                        ed.panels.layout.toggle_maximize(leaf_id);
                        cx.notify();
                    });
                });

            let close_editor = editor.clone();
            let close_button = icon_chip_button(c, d)
                .id(("area-topbar-close", leaf_id))
                .child(
                    svg()
                        .path(panel_topbar_icon(kind, "close"))
                        .size(px(d.topbar_height * 0.5 - 2.0))
                        .text_color(c.dialog_muted),
                )
                .on_click(move |_event, _window, cx| {
                    let _ = close_editor.update(cx, |ed, cx| {
                        ed.close_panel(leaf_id, cx);
                        cx.notify();
                    });
                });

            actions = actions.child(max_button).child(close_button);
        }

        topbar_container(c, d.topbar_height, 8.0)
            .id(("area-topbar", leaf_id))
            .child(div().flex().items_center().gap(px(8.0)).child(type_button))
            .child(div().flex().items_center().gap(px(6.0)).child(actions))
            .into_any_element()
    }
}
