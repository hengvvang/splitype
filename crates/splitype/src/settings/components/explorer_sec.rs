//! Explorer settings section component: hidden files, sorting mode, sort order, and auto reveal.

use gpui::*;

use splitype_infra::config::settings::{ExplorerSortMode, ExplorerSortOrder};
use splitype_infra::theme::{ThemeColors, ThemeDimensions};
use crate::settings::ui_helpers::{SettingsClickHandler, make_row, make_section};
use splitype_ui::select::{select_option, select_panel, select_trigger};
use splitype_ui::switch::Switch;

pub(crate) struct ExplorerProps {
    pub hide_hidden: bool,
    pub on_toggle_hide_hidden: SettingsClickHandler,

    pub sort_mode: ExplorerSortMode,
    pub is_sort_mode_open: bool,
    pub on_toggle_sort_mode: SettingsClickHandler,
    pub on_select_sort_mode: Box<dyn Fn(ExplorerSortMode) -> SettingsClickHandler>,

    pub sort_order: ExplorerSortOrder,
    pub is_sort_order_open: bool,
    pub on_toggle_sort_order: SettingsClickHandler,
    pub on_select_sort_order: Box<dyn Fn(ExplorerSortOrder) -> SettingsClickHandler>,

    pub auto_reveal: bool,
    pub on_toggle_auto_reveal: SettingsClickHandler,
}

pub(crate) fn render_explorer_section(
    c: &ThemeColors,
    d: &ThemeDimensions,
    id: impl Into<ElementId>,
    expanded: bool,
    toggle_fn: SettingsClickHandler,
    props: ExplorerProps,
) -> AnyElement {
    let mut inner_border_color = c.dialog_border;
    inner_border_color.a *= 0.4;

    let mut rows = Vec::new();

    if expanded {
        // 1. Hide Hidden Files Toggle
        let ctrl_hidden = Switch::new("switch-exp-hidden")
            .checked(props.hide_hidden)
            .on_click(props.on_toggle_hide_hidden)
            .into_any_element();

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "Filter Dotfiles & Hidden Items",
            "Hide files and directories starting with a dot from the tree",
            ctrl_hidden,
        ));

        // 2. Sort Mode Selector
        let sort_mode_options = ExplorerSortMode::all();
        let current_sort_mode_label = props.sort_mode.display_name();

        let mut sort_mode_btn_wrap = div().relative().child(
            select_trigger("pref-btn-exp-sort-mode", c, d)
                .text_size(px(12.0))
                .text_color(c.text_default)
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .truncate()
                        .child(current_sort_mode_label),
                )
                .child(
                    div().flex_shrink_0().pl(px(4.0)).child(
                        svg()
                            .path("icons/settings/select-chevron.svg")
                            .size(px(16.0))
                            .text_color(c.dialog_muted),
                    ),
                )
                .on_click(props.on_toggle_sort_mode),
        );

        if props.is_sort_mode_open {
            let mut menu_items = Vec::new();
            for mode in sort_mode_options {
                let is_selected = *mode == props.sort_mode;
                menu_items.push(
                    select_option(
                        ElementId::Name(format!("exp-sort-mode-{}", mode.as_str()).into()),
                        c,
                        d,
                    )
                    .bg(if is_selected {
                        c.panel_row_selected
                    } else {
                        c.dialog_surface
                    })
                    .text_size(px(12.0))
                    .text_color(c.text_default)
                    .child(mode.display_name())
                    .child(if is_selected {
                        svg()
                            .path("icons/settings/checkmark.svg")
                            .size(px(15.0))
                            .text_color(c.dialog_primary_button_bg)
                            .into_any_element()
                    } else {
                        div().w(px(13.0)).into_any_element()
                    })
                    .on_click((props.on_select_sort_mode)(*mode))
                    .into_any_element(),
                );
            }

            sort_mode_btn_wrap =
                sort_mode_btn_wrap.child(gpui::deferred(select_panel(c, d).children(menu_items)));
        }

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "File & Directory Sorting Rule",
            "How directories and files are grouped and ordered in explorer",
            sort_mode_btn_wrap.into_any_element(),
        ));

        // 3. Sort Order Selector
        let sort_order_options = ExplorerSortOrder::all();
        let current_sort_order_label = props.sort_order.display_name();

        let mut sort_order_btn_wrap = div().relative().child(
            select_trigger("pref-btn-exp-sort-order", c, d)
                .text_size(px(12.0))
                .text_color(c.text_default)
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .truncate()
                        .child(current_sort_order_label),
                )
                .child(
                    div().flex_shrink_0().pl(px(4.0)).child(
                        svg()
                            .path("icons/settings/select-chevron.svg")
                            .size(px(16.0))
                            .text_color(c.dialog_muted),
                    ),
                )
                .on_click(props.on_toggle_sort_order),
        );

        if props.is_sort_order_open {
            let mut menu_items = Vec::new();
            for order in sort_order_options {
                let is_selected = *order == props.sort_order;
                menu_items.push(
                    select_option(
                        ElementId::Name(format!("exp-sort-order-{}", order.as_str()).into()),
                        c,
                        d,
                    )
                    .bg(if is_selected {
                        c.panel_row_selected
                    } else {
                        c.dialog_surface
                    })
                    .text_size(px(12.0))
                    .text_color(c.text_default)
                    .child(order.display_name())
                    .child(if is_selected {
                        svg()
                            .path("icons/settings/checkmark.svg")
                            .size(px(15.0))
                            .text_color(c.dialog_primary_button_bg)
                            .into_any_element()
                    } else {
                        div().w(px(13.0)).into_any_element()
                    })
                    .on_click((props.on_select_sort_order)(*order))
                    .into_any_element(),
                );
            }

            sort_order_btn_wrap =
                sort_order_btn_wrap.child(gpui::deferred(select_panel(c, d).children(menu_items)));
        }

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "Sorting Direction",
            "Alphabetical ascending (A-Z) or descending (Z-A) ordering",
            sort_order_btn_wrap.into_any_element(),
        ));

        // 4. Auto Reveal Active File Toggle
        let ctrl_reveal = Switch::new("switch-exp-reveal")
            .checked(props.auto_reveal)
            .on_click(props.on_toggle_auto_reveal)
            .into_any_element();

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "Auto Reveal Active Document",
            "Automatically expand folders and select the active document in explorer",
            ctrl_reveal,
        ));
    }

    make_section(
        c,
        d,
        id,
        "File Tree & Workspace Explorer",
        expanded,
        toggle_fn,
        rows,
    )
}
