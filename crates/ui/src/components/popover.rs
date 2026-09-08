//! Popover style builders — floating menu panel containers.
//!
//! Positioning (`.absolute().left().top().w()`) and event handlers stay at
//! call sites; this builder supplies the shared surface styling.

use gpui::*;

use theme::{ThemeColors, ThemeDimensions};

/// Floating menu panel container.
pub fn menu_panel(c: &ThemeColors, d: &ThemeDimensions) -> Div {
    div()
        .occlude()
        .bg(c.dialog_surface)
        .border(px(d.dialog_border_width))
        .border_color(c.dialog_border)
        .rounded(px(d.menu_panel_radius))
        .shadow_lg()
        .p(px(d.menu_panel_padding))
        .flex()
        .flex_col()
        .gap(px(d.menu_panel_gap))
}

/// Full-window overlay layer — covers the editor viewport.
///
/// Event handling (`on_mouse_down`), occlude, and centering stay at call
/// sites; this builder supplies the four-corner full-screen geometry.
pub fn overlay() -> Div {
    div().absolute().top_0().left_0().right_0().bottom_0()
}

/// Maximum width cap for a floating menu panel to prevent runaway sizes.
pub const MENU_PANEL_MAX_WIDTH: f32 = 560.0;

/// Returns true if a character belongs to wide CJK / fullwidth Unicode ranges.
pub fn is_wide_menu_char(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1100..=0x11ff
            | 0x2e80..=0xa4cf
            | 0xac00..=0xd7a3
            | 0xf900..=0xfaff
            | 0xfe10..=0xfe6f
            | 0xff00..=0xff60
            | 0xffe0..=0xffe6
    )
}

/// Estimates the rendered pixel width of a menu label string given the font size.
pub fn estimated_menu_label_width(label: &str, text_size: f32) -> f32 {
    label
        .chars()
        .map(|ch| {
            if ch.is_ascii_whitespace() {
                text_size * 0.35
            } else if ch.is_ascii_punctuation() {
                text_size * 0.45
            } else if ch.is_ascii_uppercase() {
                text_size * 0.65
            } else if ch.is_ascii() {
                text_size * 0.55
            } else if is_wide_menu_char(ch) {
                text_size
            } else {
                text_size * 0.85
            }
        })
        .sum()
}

/// Calculates the ideal width for a menu panel containing the given labels with an explicit font size,
/// accounting for item horizontal padding, menu container padding, borders, and font variances.
pub fn menu_panel_width_for_labels_with_size<S: AsRef<str>>(
    labels: &[S],
    text_size: f32,
    dimensions: &ThemeDimensions,
) -> f32 {
    let widest_label = labels
        .iter()
        .map(|label| estimated_menu_label_width(label.as_ref(), text_size))
        .fold(0.0, f32::max);
    let content_width = widest_label
        + dimensions.menu_item_padding_x * 2.0
        + dimensions.menu_panel_padding * 2.0
        + dimensions.dialog_border_width * 2.0
        + 8.0;
    dimensions
        .menu_panel_width
        .max(content_width.ceil())
        .min(MENU_PANEL_MAX_WIDTH)
}

/// Calculates the ideal width for a menu panel using standard menu text size (`dimensions.menu_text_size`).
pub fn menu_panel_width_for_labels<S: AsRef<str>>(
    labels: &[S],
    dimensions: &ThemeDimensions,
) -> f32 {
    menu_panel_width_for_labels_with_size(labels, dimensions.menu_text_size, dimensions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test;

    #[test]
    fn test_menu_panel_width_calculation() {
        let theme = theme::Theme::default_theme();
        let d = &theme.dimensions;

        // 1. Short labels should not collapse below default menu_panel_width (180.0)
        let short_labels = ["Refresh", "Collapse All"];
        let width = menu_panel_width_for_labels(&short_labels, d);
        assert_eq!(width, d.menu_panel_width);

        // 2. Long label like "Add Folder to Explorer…" should expand comfortably
        let labels_with_long = ["Add Folder to Explorer…", "Refresh", "Collapse All"];
        let item_text_size = 17.0 * 0.8; // 13.6px
        let dynamic_width =
            menu_panel_width_for_labels_with_size(&labels_with_long, item_text_size, d);
        assert!(
            dynamic_width > 200.0,
            "Expected width > 200.0 for 'Add Folder to Explorer…', got {}",
            dynamic_width
        );

        // 3. Chinese label "添加文件夹到资源管理器" also expands properly
        let zh_labels = ["添加文件夹到资源管理器", "刷新", "全部折叠"];
        let zh_width = menu_panel_width_for_labels_with_size(&zh_labels, item_text_size, d);
        assert!(
            zh_width > 180.0,
            "Expected width > 180.0 for Chinese labels, got {}",
            zh_width
        );
    }
}


