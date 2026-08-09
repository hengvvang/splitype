//! Top bar of an Explorer area: the area type selector and split/close
//! controls.

use gpui::*;

use crate::layout::{AreaSplitMode, Axis};
use crate::infra::theme::Theme;
use crate::ui::components::button::{icon_chip_button, small_pill_button};
use crate::ui::components::topbar::topbar_container;
use crate::editor::window_layout::area_topbar_icon;

impl crate::editor::controller::Editor {
    /// Top bar of an Explorer area: type selector and split/close controls.
    pub(crate) fn render_explorer_topbar(
        &self,
        leaf_id: usize,
        kind: crate::layout::WindowAreaKind,
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
                    ed.panels.layout.toggle_window_area_dropdown(leaf_id);
                    cx.notify();
                });
            });

        let split_h_editor = editor.clone();
        let split_h_button = icon_chip_button(c, d)
            .id(("area-topbar-split-h", leaf_id))
            .child(
                svg()
                    .path(area_topbar_icon(kind, "split-h"))
                    .size(px(d.topbar_height * 0.5 + 2.0))
                    .text_color(c.dialog_muted),
            )
            .on_click(move |_event, _window, cx| {
                let _ = split_h_editor.update(cx, |ed, cx| {
                    // Same-kind split; Editor areas deep-copy their tabs.
                    ed.split_area(leaf_id, Axis::Horizontal, 0.5, AreaSplitMode::Copy, cx);
                    cx.notify();
                });
            });

        let split_v_editor = editor.clone();
        let split_v_button = icon_chip_button(c, d)
            .id(("area-topbar-split-v", leaf_id))
            .child(
                svg()
                    .path(area_topbar_icon(kind, "split-v"))
                    .size(px(d.topbar_height * 0.5 + 2.0))
                    .text_color(c.dialog_muted),
            )
            .on_click(move |_event, _window, cx| {
                let _ = split_v_editor.update(cx, |ed, cx| {
                    // Same-kind split; Editor areas deep-copy their tabs.
                    ed.split_area(leaf_id, Axis::Vertical, 0.5, AreaSplitMode::Copy, cx);
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
                            area_topbar_icon(kind, "restore")
                        } else {
                            area_topbar_icon(kind, "maximize")
                        })
                        .size(px(d.topbar_height * 0.5 - 2.0))
                        .text_color(c.dialog_muted),
                )
                .on_click(move |_event, _window, cx| {
                    let _ = max_editor.update(cx, |ed, cx| {
                        ed.panels.layout.toggle_window_area_maximize(leaf_id);
                        cx.notify();
                    });
                });

            let close_editor = editor.clone();
            let close_button = icon_chip_button(c, d)
                .id(("area-topbar-close", leaf_id))
                .child(
                    svg()
                        .path(area_topbar_icon(kind, "close"))
                        .size(px(d.topbar_height * 0.5 - 2.0))
                        .text_color(c.dialog_muted),
                )
                .on_click(move |_event, _window, cx| {
                    let _ = close_editor.update(cx, |ed, cx| {
                        ed.panels.layout.close_window_area(leaf_id);
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
