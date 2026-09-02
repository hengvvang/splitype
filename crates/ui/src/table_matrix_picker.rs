//! Reusable interactive table dimension matrix picker component.
//!
//! Provides a configurable matrix grid (e.g. 8x8 for insertion, 8x6 for resizing),
//! live dimension badges (`[ N ] Row x [ M ] Column`), and theme-aware cell highlight states.

use gpui::*;

use theme::Theme;

/// Renders the top indicator badge: `[ Row count ] Row  x  [ Col count ] Column`.
pub fn render_matrix_dimension_indicator(
    rows: usize,
    cols: usize,
    row_label: impl Into<SharedString>,
    col_label: impl Into<SharedString>,
    theme: &Theme,
) -> Div {
    let c = &theme.colors;
    div()
        .w_full()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(8.0))
        .pb(px(8.0))
        .border_b(px(1.0))
        .border_color(c.dialog_border)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .child(
                    div()
                        .px(px(8.0))
                        .py(px(2.0))
                        .min_w(px(28.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .border(px(1.0))
                        .border_color(c.dialog_border)
                        .rounded(px(3.0))
                        .bg(c.dialog_surface)
                        .text_size(px(12.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(c.text_default)
                        .child(format!("{}", rows)),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(c.dialog_muted)
                        .child(row_label.into()),
                ),
        )
        .child(
            div()
                .text_size(px(12.0))
                .text_color(c.dialog_muted)
                .child("x"),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .child(
                    div()
                        .px(px(8.0))
                        .py(px(2.0))
                        .min_w(px(28.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .border(px(1.0))
                        .border_color(c.dialog_border)
                        .rounded(px(3.0))
                        .bg(c.dialog_surface)
                        .text_size(px(12.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(c.text_default)
                        .child(format!("{}", cols)),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(c.dialog_muted)
                        .child(col_label.into()),
                ),
        )
}
