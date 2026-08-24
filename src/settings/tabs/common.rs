//! Shared UI helpers and building blocks across settings tab views.

use gpui::*;

use crate::infra::theme::{ThemeColors, ThemeDimensions};
use crate::settings::window::SettingsWindow;

pub(crate) use crate::settings::common::{make_row, SettingsClickHandler};

pub(crate) fn render_zed_stepper(
    c: &ThemeColors,
    d: &ThemeDimensions,
    id_dec: &'static str,
    id_inc: &'static str,
    val_num: String,
    unit_str: &'static str,
    is_editing: bool,
    on_dec: SettingsClickHandler,
    on_inc: SettingsClickHandler,
    on_click_center: SettingsClickHandler,
) -> AnyElement {
    crate::settings::common::render_zed_stepper(
        c,
        d,
        id_dec,
        id_inc,
        format!("{}-center", id_dec),
        val_num,
        unit_str,
        is_editing,
        on_dec,
        on_inc,
        on_click_center,
    )
}

pub(crate) fn make_section(
    c: &ThemeColors,
    d: &ThemeDimensions,
    id: &'static str,
    key: &'static str,
    title: &'static str,
    expanded: bool,
    toggle_ed: WeakEntity<SettingsWindow>,
    items: Vec<AnyElement>,
) -> AnyElement {
    crate::settings::common::make_section(
        c,
        d,
        format!("{}-header", id),
        id,
        title,
        expanded,
        Box::new(move |_event, _window, cx| {
            let _ = toggle_ed.update(cx, |this, cx| {
                if this.expanded_sections.contains(key) {
                    this.expanded_sections.remove(key);
                } else {
                    this.expanded_sections.insert(key.to_string());
                }
                cx.notify();
            });
        }),
        items,
    )
}
