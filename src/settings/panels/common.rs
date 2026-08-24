//! Common UI helpers and building blocks for in-editor settings panel.

use gpui::*;

use crate::infra::theme::Theme;

pub(crate) use crate::settings::common::SettingsClickHandler;

pub(crate) fn make_row(
    title: &'static str,
    desc: &'static str,
    control: AnyElement,
    theme: &Theme,
    border_col: Hsla,
) -> AnyElement {
    crate::settings::common::make_row(
        border_col,
        &theme.colors,
        &theme.dimensions,
        title,
        desc,
        control,
    )
}

pub(crate) fn render_zed_stepper(
    id_dec: &'static str,
    id_inc: &'static str,
    val_num: String,
    unit_str: &'static str,
    is_editing: bool,
    on_dec: SettingsClickHandler,
    on_inc: SettingsClickHandler,
    on_click_center: SettingsClickHandler,
    theme: &Theme,
    panel_id: usize,
) -> AnyElement {
    crate::settings::common::render_zed_stepper(
        &theme.colors,
        &theme.dimensions,
        (id_dec, panel_id),
        (id_inc, panel_id),
        format!("{}-center-{}", id_dec, panel_id),
        val_num,
        unit_str,
        is_editing,
        on_dec,
        on_inc,
        on_click_center,
    )
}

pub(crate) fn make_section(
    sec_id: &'static str,
    title: &'static str,
    is_expanded: bool,
    toggle_fn: SettingsClickHandler,
    items: Vec<AnyElement>,
    theme: &Theme,
    panel_id: usize,
) -> AnyElement {
    crate::settings::common::make_section(
        &theme.colors,
        &theme.dimensions,
        (sec_id, panel_id),
        ElementId::Name(format!("{}-card-{}", sec_id, panel_id).into()),
        title,
        is_expanded,
        toggle_fn,
        items,
    )
}
