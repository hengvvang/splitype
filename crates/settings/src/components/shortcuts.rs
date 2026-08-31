//! Shortcuts overview section component.

use gpui::*;

use crate::components::shortcuts_data::ShortcutItem;
use crate::ui_helpers::{SettingsClickHandler, make_row, make_section};
use theme::{ThemeColors, ThemeDimensions};

pub(crate) fn render_shortcuts_section(
    c: &ThemeColors,
    d: &ThemeDimensions,
    id: impl Into<ElementId>,
    title: &'static str,
    expanded: bool,
    toggle_fn: SettingsClickHandler,
    items: &[ShortcutItem],
) -> AnyElement {
    let mut inner_border_color = c.dialog_border;
    inner_border_color.a *= 0.4;

    let mut rows = Vec::new();
    if expanded {
        for item in items {
            let ctrl_sc = div()
                .px(px(8.0))
                .py(px(2.0))
                .rounded(px(d.badge_radius))
                .bg(c.dialog_secondary_button_hover)
                .text_size(px(11.0))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(c.text_default)
                .child(item.shortcut)
                .into_any_element();

            rows.push(make_row(
                inner_border_color,
                c,
                d,
                item.name,
                item.description,
                ctrl_sc,
            ));
        }
    }

    make_section(c, d, id, title, expanded, toggle_fn, rows)
}
