//! WYSIWYG pane — the rendered block view.
//!
//! Row layout and spacing helpers used by the main render pass; the render
//! orchestration itself lives in `crate::pane`.

use gpui::*;

use crate::model::RowSpacingInfo;

use ::theme::*;

// ── Spacing helpers ─────────────────────────────────────────────────────

/// Gap between two consecutive rendered rows.
///
/// Returns 0 for rows that share a quote group or border an empty paragraph
/// row; otherwise returns the default block gap from the theme.
pub fn row_top_gap(
    previous: Option<RowSpacingInfo>,
    current: RowSpacingInfo,
    default_gap: f32,
) -> f32 {
    let Some(previous) = previous else {
        return 0.0;
    };
    if (previous.quote_group_id.is_some() && previous.quote_group_id == current.quote_group_id)
        || previous.is_empty_paragraph
        || current.is_empty_paragraph
    {
        0.0
    } else {
        default_gap
    }
}

/// Gap for rows inside a callout group.
pub fn callout_row_top_gap(
    previous: Option<RowSpacingInfo>,
    current: RowSpacingInfo,
    dimensions: &ThemeDimensions,
) -> f32 {
    let Some(previous) = previous else {
        return 0.0;
    };
    if previous.visible_quote_group_id.is_some()
        && previous.visible_quote_group_id == current.visible_quote_group_id
    {
        return 0.0;
    }
    if previous.is_callout_header {
        dimensions.callout_header_margin_bottom
    } else {
        dimensions.callout_body_gap
    }
}

/// Gap for rows inside a footnote group.
pub fn footnote_row_top_gap(previous: Option<RowSpacingInfo>, default_gap: f32) -> f32 {
    let Some(previous) = previous else {
        return 0.0;
    };
    if previous.is_footnote_header {
        default_gap * 0.75
    } else {
        default_gap
    }
}

pub fn callout_colors(variant: markdown_parser::block::CalloutKind, theme: &Theme) -> (Hsla, Hsla) {
    syntax_highlighter::graphics::markup::callout_colors(variant, theme)
}

/// Linearly interpolates the editor content width ratio based on viewport
/// width. The column stays full-width until `centered_shrink_start`, then
/// shrinks to `centered_min_ratio` at `centered_shrink_end`.
pub fn centered_column_ratio(viewport_width: f32, dimensions: &ThemeDimensions) -> f32 {
    if viewport_width <= dimensions.centered_shrink_start {
        return 1.0;
    }

    let t = ((viewport_width - dimensions.centered_shrink_start)
        / (dimensions.centered_shrink_end - dimensions.centered_shrink_start))
        .clamp(0.0, 1.0);
    1.0 - t * (1.0 - dimensions.centered_min_ratio)
}

/// The centered content column width for `viewport_width`.
pub fn centered_column_width(viewport_width: f32, dimensions: &ThemeDimensions) -> f32 {
    let available_content_width = (viewport_width - dimensions.editor_padding * 2.0).max(1.0);
    let centered_ratio = centered_column_ratio(viewport_width, dimensions);
    (available_content_width * centered_ratio)
        .max(320.0)
        .min(available_content_width)
}
