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

/// Settings row container — title/description label and a control on the right (4px radius).
pub fn settings_row(border: Hsla, c: &ThemeColors, d: &ThemeDimensions) -> Div {
    div()
        .w_full()
        .min_w(px(0.0))
        .min_h(px(56.0))
        .py(px(10.0))
        .px(px(16.0))
        .rounded(px(d.settings_row_radius))
        .bg(c.dialog_surface)
        .border_1()
        .border_color(border)
        .hover(|this| this.bg(c.panel_row_hover))
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(16.0))
}

/// Settings card group container (WinUI 3 8px section_card_radius).
pub fn settings_group(c: &ThemeColors, d: &ThemeDimensions) -> Div {
    div()
        .w_full()
        .min_w(px(0.0))
        .rounded(px(d.section_card_radius))
        .border_1()
        .border_color(c.dialog_border)
        .bg(c.dialog_surface)
        .overflow_hidden()
        .flex()
        .flex_col()
}

/// A unified settings row designed after Windows 11 SettingsCard with dynamic multi-line
/// description wrapping, optional icon, optional chevron, and non-shrinking right control.
pub fn settings_card_row(
    c: &ThemeColors,
    d: &ThemeDimensions,
    icon: Option<&'static str>,
    title: &str,
    desc: &str,
    query: &str,
    on_reset: Option<CardClickHandler>,
    control: AnyElement,
    has_chevron: bool,
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
        let reset_id = ElementId::Name(format!("reset-{title}").into());
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

    let left_side = if let Some(icon_path) = icon {
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
                            .path(icon_path)
                            .size(px(18.0))
                            .text_color(c.text_default),
                    ),
            )
            .child(label_column)
            .into_any_element()
    } else {
        label_column.into_any_element()
    };

    let mut control_area = div()
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(control);

    if has_chevron {
        control_area = control_area.child(
            svg()
                .path("plugin://splitype.settings/chevron-right.svg")
                .size(px(14.0))
                .text_color(c.dialog_muted)
                .flex_shrink_0(),
        );
    }

    settings_row(c.dialog_border, c, d)
        .child(left_side)
        .child(control_area)
        .into_any_element()
}
