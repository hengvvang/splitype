//! Startup and general lifecycle settings section component.

use gpui::*;

use crate::ui_helpers::{SettingsClickHandler, SettingsOptionHandler, make_row, make_section};
use config::settings::StartupOpenSetting;
use theme::{ThemeColors, ThemeDimensions};
use ui::select::{select_option, select_panel, select_trigger};
use ui::switch::Switch;

pub(crate) struct StartupProps {
    pub startup_open: StartupOpenSetting,
    pub is_startup_open: bool,
    pub on_toggle_startup: SettingsClickHandler,
    pub on_select_startup: SettingsOptionHandler<StartupOpenSetting>,

    pub restore_window_state: bool,
    pub on_toggle_restore_window_state: SettingsClickHandler,
}

pub(crate) fn render_startup_section(
    c: &ThemeColors,
    d: &ThemeDimensions,
    id: impl Into<ElementId>,
    expanded: bool,
    toggle_fn: SettingsClickHandler,
    props: StartupProps,
) -> AnyElement {
    let mut inner_border_color = c.dialog_border;
    inner_border_color.a *= 0.4;

    let mut rows = Vec::new();

    if expanded {
        // 1. Startup Document Selection Mode
        let startup_options = StartupOpenSetting::all();
        let current_startup_label = props.startup_open.display_name();

        let mut startup_btn_wrap = div().relative().child(
            select_trigger("pref-btn-startup", c, d)
                .text_size(px(12.0))
                .text_color(c.text_default)
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .truncate()
                        .child(current_startup_label),
                )
                .child(
                    div().flex_shrink_0().pl(px(4.0)).child(
                        svg()
                            .path("icons/settings/select-chevron.svg")
                            .size(px(16.0))
                            .text_color(c.dialog_muted),
                    ),
                )
                .on_click(props.on_toggle_startup),
        );

        if props.is_startup_open {
            let mut menu_items = Vec::new();
            for opt in startup_options {
                let is_selected = *opt == props.startup_open;
                menu_items.push(
                    select_option(
                        ElementId::Name(format!("startup-item-{}", opt.as_str()).into()),
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
                    .child(opt.display_name())
                    .child(if is_selected {
                        svg()
                            .path("icons/settings/checkmark.svg")
                            .size(px(15.0))
                            .text_color(c.dialog_primary_button_bg)
                            .into_any_element()
                    } else {
                        div().w(px(13.0)).into_any_element()
                    })
                    .on_click((props.on_select_startup)(*opt))
                    .into_any_element(),
                );
            }

            startup_btn_wrap =
                startup_btn_wrap.child(gpui::deferred(select_panel(c, d).children(menu_items)));
        }

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "App Startup Behavior",
            "Initial document or workspace opened upon launching Splitype",
            startup_btn_wrap.into_any_element(),
        ));

        // 2. Restore Window State Toggle
        let ctrl_window_state = Switch::new("switch-startup-win-state")
            .checked(props.restore_window_state)
            .on_click(props.on_toggle_restore_window_state)
            .into_any_element();

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "Restore Window Geometry & State",
            "Remember previous window dimensions, position, and layout splits on startup",
            ctrl_window_state,
        ));
    }

    make_section(
        c,
        d,
        id,
        "Application Startup & General",
        expanded,
        toggle_fn,
        rows,
    )
}
