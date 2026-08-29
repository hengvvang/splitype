//! Shared UI helpers and building blocks across settings panels and window tabs.

use gpui::*;

use splitype_infra::theme::{ThemeColors, ThemeDimensions};
use splitype_ui::section::{section_card, section_header, settings_row};

pub(crate) type SettingsClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

pub(crate) fn make_row(
    inner_border_color: Hsla,
    c: &ThemeColors,
    d: &ThemeDimensions,
    title: &'static str,
    desc: &'static str,
    control: AnyElement,
) -> AnyElement {
    make_row_with_reset(inner_border_color, c, d, title, desc, None, control)
}

pub(crate) fn make_row_with_reset(
    inner_border_color: Hsla,
    c: &ThemeColors,
    d: &ThemeDimensions,
    title: &'static str,
    desc: &'static str,
    on_reset: Option<SettingsClickHandler>,
    control: AnyElement,
) -> AnyElement {
    let mut title_row = div().flex().items_center().gap(px(6.0)).child(
        div()
            .text_size(px(12.5))
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(c.text_default)
            .child(title),
    );

    if let Some(reset_fn) = on_reset {
        let reset_id = ElementId::Name(format!("reset-{}", title).into());
        title_row = title_row.child(
            div()
                .id(reset_id)
                .cursor_pointer()
                .p(px(2.0))
                .rounded(px(3.0))
                .hover(|s| s.bg(c.dialog_secondary_button_hover))
                .child(
                    svg()
                        .path("icons/settings/undo.svg")
                        .size(px(12.0))
                        .text_color(c.dialog_muted)
                        .hover(|s| s.text_color(c.text_default)),
                )
                .on_click(reset_fn),
        );
    }

    settings_row(inner_border_color, c, d)
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(title_row)
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(c.dialog_muted)
                        .child(desc),
                ),
        )
        .child(control)
        .into_any_element()
}

pub(crate) fn make_section(
    c: &ThemeColors,
    d: &ThemeDimensions,
    id: impl Into<ElementId>,
    title: &'static str,
    expanded: bool,
    toggle_fn: SettingsClickHandler,
    items: Vec<AnyElement>,
) -> AnyElement {
    let base_id = id.into();
    let header_id = ElementId::from((base_id.clone(), "header"));
    let card_id = ElementId::from((base_id, "card"));
    let header = section_header()
        .id(header_id)
        .child(
            svg()
                .path(if expanded {
                    "icons/settings/chevron-down.svg"
                } else {
                    "icons/settings/chevron-right.svg"
                })
                .size(px(16.0))
                .text_color(c.text_default),
        )
        .child(
            div()
                .text_size(px(13.0))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(c.text_default)
                .child(title),
        )
        .on_click(move |event, window, cx| toggle_fn(event, window, cx));

    let mut card = section_card(c, d).id(card_id).child(header);

    if expanded && !items.is_empty() {
        let body = div()
            .w_full()
            .px(px(10.0))
            .pb(px(10.0))
            .pt(px(2.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .children(items);

        card = card.child(body);
    }

    card.into_any_element()
}
