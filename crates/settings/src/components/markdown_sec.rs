//! Markdown & Document Assets settings section component.

use gpui::*;

use config::settings::ImagePasteBehavior;
use theme::{ThemeColors, ThemeDimensions};
use crate::ui_helpers::{SettingsClickHandler, make_row, make_section};
use ui::select::{select_option, select_panel, select_trigger};
use ui::switch::Switch;

pub(crate) struct MarkdownProps {
    pub show_table_headers: bool,
    pub on_toggle_table_headers: SettingsClickHandler,

    pub image_paste_behavior: ImagePasteBehavior,
    pub is_image_paste_open: bool,
    pub on_toggle_image_paste: SettingsClickHandler,
    pub on_select_image_paste: Box<dyn Fn(ImagePasteBehavior) -> SettingsClickHandler>,

    pub render_math: bool,
    pub on_toggle_render_math: SettingsClickHandler,

    pub render_diagrams: bool,
    pub on_toggle_render_diagrams: SettingsClickHandler,
}

pub(crate) fn render_markdown_section(
    c: &ThemeColors,
    d: &ThemeDimensions,
    id: impl Into<ElementId>,
    expanded: bool,
    toggle_fn: SettingsClickHandler,
    props: MarkdownProps,
) -> AnyElement {
    let mut inner_border_color = c.dialog_border;
    inner_border_color.a *= 0.4;

    let mut rows = Vec::new();

    if expanded {
        // 1. Persistent Table Headers
        let ctrl_tbl = Switch::new("switch-tbl-headers")
            .checked(props.show_table_headers)
            .on_click(props.on_toggle_table_headers)
            .into_any_element();

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "Persistent Table Headers",
            "Keep table column headers visible when editing table blocks",
            ctrl_tbl,
        ));

        // 2. Clipboard Image Paste Behavior
        let paste_options = ImagePasteBehavior::all();
        let current_paste_label = props.image_paste_behavior.display_name();

        let mut paste_btn_wrap = div().relative().child(
            select_trigger("pref-btn-img-paste", c, d)
                .text_size(px(12.0))
                .text_color(c.text_default)
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .truncate()
                        .child(current_paste_label),
                )
                .child(
                    div().flex_shrink_0().pl(px(4.0)).child(
                        svg()
                            .path("icons/settings/select-chevron.svg")
                            .size(px(16.0))
                            .text_color(c.dialog_muted),
                    ),
                )
                .on_click(props.on_toggle_image_paste),
        );

        if props.is_image_paste_open {
            let mut menu_items = Vec::new();
            for behavior in paste_options {
                let is_selected = *behavior == props.image_paste_behavior;
                menu_items.push(
                    select_option(
                        ElementId::Name(format!("paste-item-{}", behavior.as_str()).into()),
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
                    .child(behavior.display_name())
                    .child(if is_selected {
                        svg()
                            .path("icons/settings/checkmark.svg")
                            .size(px(15.0))
                            .text_color(c.dialog_primary_button_bg)
                            .into_any_element()
                    } else {
                        div().w(px(13.0)).into_any_element()
                    })
                    .on_click((props.on_select_image_paste)(*behavior))
                    .into_any_element(),
                );
            }

            paste_btn_wrap =
                paste_btn_wrap.child(gpui::deferred(select_panel(c, d).children(menu_items)));
        }

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "Clipboard Image Paste Action",
            "Storage destination and path strategy when pasting images from clipboard",
            paste_btn_wrap.into_any_element(),
        ));

        // 3. LaTeX Math Rendering
        let ctrl_math = Switch::new("switch-render-math")
            .checked(props.render_math)
            .on_click(props.on_toggle_render_math)
            .into_any_element();

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "LaTeX Math Formula Rendering",
            "Enable live KaTeX math formula rendering in preview and rich blocks",
            ctrl_math,
        ));

        // 4. Mermaid Diagrams Rendering
        let ctrl_diagrams = Switch::new("switch-render-diagrams")
            .checked(props.render_diagrams)
            .on_click(props.on_toggle_render_diagrams)
            .into_any_element();

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "Mermaid Diagram Rendering",
            "Enable live Mermaid diagram visualization for flowchart and chart blocks",
            ctrl_diagrams,
        ));
    }

    make_section(
        c,
        d,
        id,
        "Markdown & Document Elements",
        expanded,
        toggle_fn,
        rows,
    )
}

