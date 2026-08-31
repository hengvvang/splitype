//! Top bar of a Settings area: type selector and split/close controls.
//!
//! All mutations dispatch `workspace` layout actions; the shell handles
//! them against its window layout tree.

use gpui::*;

use core_contracts::PanelId;
use splitter::SplitAxis;
use theme::Theme;
use ui::button::{icon_chip_button, small_pill_button, toolbar_icon_size};
use ui::topbar::topbar_container;
use window::actions::{ClosePanel, SplitPanel, ToggleKindDropdown, TogglePanelMaximized};
use window::panel_topbar_icon;

/// Top bar of a Settings area: type selector and split/close controls.
pub fn render_settings_topbar(
    panel_id: PanelId,
    icon_prefix: &'static str,
    theme: &Theme,
    leaf_count: usize,
    is_maximized: bool,
    _cx: &mut App,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;

    let type_button = small_pill_button(c, d)
        .id(("panel-topbar-type", panel_id.0))
        .text_size(px(12.0))
        .text_color(c.text_default)
        .child("Settings")
        .on_click(move |_event, window, cx| {
            window.dispatch_action(Box::new(ToggleKindDropdown { panel: panel_id.0 }), cx);
        });

    let btn_icon_size = toolbar_icon_size(d.topbar_height);

    let split_h_button = icon_chip_button(c, d)
        .id(("panel-topbar-split-h", panel_id.0))
        .child(
            svg()
                .path(panel_topbar_icon(icon_prefix, "split-h"))
                .size(px(btn_icon_size))
                .text_color(c.dialog_muted),
        )
        .on_click(move |_event, window, cx| {
            window.dispatch_action(
                Box::new(SplitPanel {
                    panel: panel_id.0,
                    axis: SplitAxis::Horizontal,
                }),
                cx,
            );
        });

    let split_v_button = icon_chip_button(c, d)
        .id(("panel-topbar-split-v", panel_id.0))
        .child(
            svg()
                .path(panel_topbar_icon(icon_prefix, "split-v"))
                .size(px(btn_icon_size))
                .text_color(c.dialog_muted),
        )
        .on_click(move |_event, window, cx| {
            window.dispatch_action(
                Box::new(SplitPanel {
                    panel: panel_id.0,
                    axis: SplitAxis::Vertical,
                }),
                cx,
            );
        });

    let mut actions = div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .child(split_v_button)
        .child(split_h_button);

    if leaf_count > 1 {
        let max_button = icon_chip_button(c, d)
            .id(("panel-topbar-max", panel_id.0))
            .child(
                svg()
                    .path(if is_maximized {
                        panel_topbar_icon(icon_prefix, "restore")
                    } else {
                        panel_topbar_icon(icon_prefix, "maximize")
                    })
                    .size(px(btn_icon_size))
                    .text_color(c.dialog_muted),
            )
            .on_click(move |_event, window, cx| {
                window.dispatch_action(Box::new(TogglePanelMaximized { panel: panel_id.0 }), cx);
            });

        let close_button = icon_chip_button(c, d)
            .id(("panel-topbar-close", panel_id.0))
            .child(
                svg()
                    .path(panel_topbar_icon(icon_prefix, "close"))
                    .size(px(btn_icon_size))
                    .text_color(c.dialog_muted),
            )
            .on_click(move |_event, window, cx| {
                window.dispatch_action(Box::new(ClosePanel { panel: panel_id.0 }), cx);
            });

        actions = actions.child(max_button).child(close_button);
    }

    topbar_container(c, d.topbar_height, 8.0)
        .id(("panel-topbar", panel_id.0))
        .child(div().flex().items_center().gap(px(8.0)).child(type_button))
        .child(div().flex().items_center().gap(px(6.0)).child(actions))
        .into_any_element()
}
