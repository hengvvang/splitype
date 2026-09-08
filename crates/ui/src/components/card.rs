//! Windows 11 Fluent Design card components: SettingsCard and SettingsGroup.
//!
//! Provides standardized card rows and section groups adhering to WinUI 3
//! geometry (8px section card radius, 4px settings row radius).

use gpui::*;
use theme::{ThemeColors, ThemeDimensions};

/// Click handler signature for card action buttons.
pub type CardClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// Highlights matching query text in a label string.
pub fn highlight_search_text(
    text: &str,
    query: &str,
    base_color: Hsla,
    highlight_bg: Hsla,
) -> AnyElement {
    let q = query.trim();
    if q.is_empty() || text.is_empty() {
        return div()
            .w_full()
            .min_w(px(0.0))
            .text_color(base_color)
            .child(text.to_string())
            .into_any_element();
    }

    let text_lower = text.to_lowercase();
    let words: Vec<String> = q.split_whitespace().map(|w| w.to_lowercase()).collect();
    if words.is_empty() {
        return div()
            .w_full()
            .min_w(px(0.0))
            .text_color(base_color)
            .child(text.to_string())
            .into_any_element();
    }

    let mut ranges = Vec::new();
    for word in &words {
        let mut start = 0;
        while let Some(idx) = text_lower[start..].find(word.as_str()) {
            let match_start = start + idx;
            let match_end = match_start + word.len();
            if text.is_char_boundary(match_start) && text.is_char_boundary(match_end) {
                ranges.push((match_start, match_end));
            }
            start = match_end.max(start + 1);
            if start >= text.len() {
                break;
            }
        }
    }

    if ranges.is_empty() {
        return div()
            .w_full()
            .min_w(px(0.0))
            .text_color(base_color)
            .child(text.to_string())
            .into_any_element();
    }

    ranges.sort_by_key(|r| r.0);
    let mut merged: Vec<std::ops::Range<usize>> = Vec::new();
    for (start, end) in ranges {
        if let Some(last) = merged.last_mut() {
            if start <= last.end {
                last.end = last.end.max(end);
                continue;
            }
        }
        merged.push(start..end);
    }

    let highlight_style = HighlightStyle {
        background_color: Some(highlight_bg),
        color: Some(base_color),
        font_weight: Some(FontWeight::SEMIBOLD),
        ..Default::default()
    };

    let styled = StyledText::new(text.to_string())
        .with_highlights(merged.into_iter().map(|range| (range, highlight_style)));

    div()
        .w_full()
        .min_w(px(0.0))
        .text_color(base_color)
        .child(styled)
        .into_any_element()
}

//

/// A settings item row inside a unified WinUI 3 group container.
/// Items inside the container are connected seamlessly with subtle divider lines.
pub fn settings_group_item_row(
    c: &ThemeColors,
    d: &ThemeDimensions,
    icon: Option<&str>,
    title: &str,
    desc: &str,
    query: &str,
    on_reset: Option<CardClickHandler>,
    control: AnyElement,
    has_top_border: bool,
) -> AnyElement {
    let has_query = !query.trim().is_empty();

    let title_element = if has_query {
        div()
            .min_w(px(0.0))
            .text_size(px(13.0))
            .font_weight(FontWeight::MEDIUM)
            .child(highlight_search_text(title, query, c.text_default, c.text_highlight_bg))
            .into_any_element()
    } else {
        div()
            .min_w(px(0.0))
            .text_size(px(13.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(c.text_default)
            .child(title.to_string())
            .into_any_element()
    };

    let mut title_row = div()
        .min_w(px(0.0))
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(title_element);

    if let Some(reset_fn) = on_reset {
        let reset_id = ElementId::Name(format!("reset-group-{title}").into());
        title_row = title_row.child(
            div()
                .id(reset_id)
                .flex_shrink_0()
                .cursor_pointer()
                .size(px(20.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(d.icon_button_radius))
                .hover(|s| s.bg(c.panel_row_hover))
                .child(
                    svg()
                        .path("plugin://splitype.settings/undo.svg")
                        .size(px(12.0))
                        .text_color(c.dialog_muted),
                )
                .on_click(reset_fn),
        );
    }

    let has_desc = !desc.is_empty();
    let label_column = if has_desc {
        let desc_element = if has_query {
            div()
                .w_full()
                .min_w(px(0.0))
                .text_size(px(11.5))
                .child(highlight_search_text(desc, query, c.dialog_muted, c.text_highlight_bg))
                .into_any_element()
        } else {
            div()
                .w_full()
                .min_w(px(0.0))
                .text_size(px(11.5))
                .text_color(c.dialog_muted)
                .child(desc.to_string())
                .into_any_element()
        };
        div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(title_row)
            .child(desc_element)
    } else {
        div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .justify_center()
            .child(title_row)
    };

    let effective_icon = icon.filter(|s| !s.trim().is_empty());
    let left_side = if let Some(icon_path) = effective_icon {
        div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .items_center()
            .gap(px(14.0))
            .child(
                div()
                    .size(px(24.0))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        svg()
                            .path(icon_path.to_string())
                            .size(px(18.0))
                            .text_color(c.text_default),
                    ),
            )
            .child(label_column)
            .into_any_element()
    } else {
        label_column.into_any_element()
    };

    let control_area = div()
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(control);

    let mut row = div()
        .w_full()
        .min_w(px(0.0))
        .min_h(px(56.0))
        .py(px(10.0))
        .px(px(16.0))
        .bg(c.dialog_surface)
        .hover(|this| this.bg(c.panel_row_hover))
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(16.0));

    if has_top_border {
        row = row.border_t_1().border_color(c.dialog_border);
    }

    row.child(left_side)
        .child(control_area)
        .into_any_element()
}

/// A complete Windows 11 Fluent Design SettingsExpander container:
/// header card with icon, title, description, and chevron, enclosing seamlessly
/// connected child rows separated by subtle dividers.
pub fn settings_expander_group(
    id: ElementId,
    icon: Option<&str>,
    title: &str,
    description: Option<&str>,
    query: &str,
    is_collapsed: bool,
    on_toggle: Option<CardClickHandler>,
    items: Vec<AnyElement>,
    c: &ThemeColors,
    d: &ThemeDimensions,
) -> AnyElement {
    let has_query = !query.trim().is_empty();

    let title_elem = if has_query {
        div()
            .min_w(px(0.0))
            .text_size(px(13.5))
            .font_weight(FontWeight::SEMIBOLD)
            .child(highlight_search_text(title, query, c.text_default, c.text_highlight_bg))
            .into_any_element()
    } else {
        div()
            .min_w(px(0.0))
            .text_size(px(13.5))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(c.text_default)
            .child(title.to_string())
            .into_any_element()
    };

    let desc_str = description.unwrap_or_default();
    let has_desc = !desc_str.is_empty();

    let text_column = if has_desc {
        let desc_elem = if has_query {
            div()
                .w_full()
                .min_w(px(0.0))
                .text_size(px(11.5))
                .child(highlight_search_text(desc_str, query, c.dialog_muted, c.text_highlight_bg))
                .into_any_element()
        } else {
            div()
                .w_full()
                .min_w(px(0.0))
                .text_size(px(11.5))
                .text_color(c.dialog_muted)
                .child(desc_str.to_string())
                .into_any_element()
        };
        div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(title_elem)
            .child(desc_elem)
    } else {
        div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .justify_center()
            .child(title_elem)
    };

    let effective_icon = icon.filter(|s| !s.trim().is_empty());
    let left_side = if let Some(icon_path) = effective_icon {
        div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .items_center()
            .gap(px(14.0))
            .child(
                div()
                    .size(px(26.0))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        svg()
                            .path(icon_path.to_string())
                            .size(px(20.0))
                            .text_color(c.text_default),
                    ),
            )
            .child(text_column)
            .into_any_element()
    } else {
        text_column.into_any_element()
    };

    let chevron_path = if is_collapsed {
        "plugin://splitype.settings/chevron-right.svg"
    } else {
        "plugin://splitype.settings/chevron-down.svg"
    };

    let right_chevron = div()
        .flex_shrink_0()
        .size(px(20.0))
        .flex()
        .items_center()
        .justify_center()
        .child(
            svg()
                .path(chevron_path)
                .size(px(14.0))
                .text_color(c.dialog_muted),
        );

    let mut header = div()
        .id(id)
        .w_full()
        .min_w(px(0.0))
        .min_h(px(56.0))
        .py(px(12.0))
        .px(px(16.0))
        .bg(c.dialog_surface)
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(16.0))
        .child(left_side)
        .child(right_chevron);

    if let Some(toggle) = on_toggle {
        header = header
            .cursor_pointer()
            .hover(|this| this.bg(c.panel_row_hover))
            .on_click(toggle);
    }

    let mut container = div()
        .w_full()
        .min_w(px(0.0))
        .rounded(px(d.section_card_radius))
        .border_1()
        .border_color(c.dialog_border)
        .bg(c.dialog_surface)
        .overflow_hidden()
        .flex()
        .flex_col()
        .child(header);

    if !is_collapsed && !items.is_empty() {
        container = container.children(items);
    }

    container.into_any_element()
}
