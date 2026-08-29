//! WYSIWYG panel — the rendered block view.
//!
//! Row layout and spacing helpers used by the main render pass; the render
//! orchestration itself lives in `crate::view`.

use gpui::*;

use crate::editor::engine::controller::*;
use ::theme::*;

// ── Spacing helpers ─────────────────────────────────────────────────────

/// Row-level spacing metadata read from a [`Block`] entity once per frame.
///
/// Used to decide whether consecutive rows belong to the same visual group
/// (quote, callout, footnote) and should collapse their inter-row gap.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RowSpacingInfo {
    pub quote_group_id: Option<BlockId>,
    pub visible_quote_group_id: Option<BlockId>,
    pub callout_group_id: Option<BlockId>,
    pub callout_variant: Option<markdown::block::CalloutKind>,
    pub is_callout_header: bool,
    pub footnote_group_id: Option<BlockId>,
    pub is_footnote_header: bool,
    pub is_empty_paragraph: bool,
}

impl RowSpacingInfo {
    /// Read spacing metadata from a block entity.
    pub fn from_block(block: &crate::editor::document::block::Block) -> Self {
        Self {
            quote_group_id: block.quote_group_id,
            visible_quote_group_id: block.visible_quote_group_id,
            callout_group_id: block.callout_group_id,
            callout_variant: block.callout_variant,
            is_callout_header: block.kind().is_callout(),
            footnote_group_id: block.footnote_group_id,
            is_footnote_header: block.kind().is_footnote_definition(),
            is_empty_paragraph: block.kind() == markdown::parse::BlockKind::Paragraph
                && block.data.text.plain_text().is_empty()
                && block.children.is_empty(),
        }
    }
}

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

/// Callout accent border + background colours from the theme.
pub fn callout_colors(variant: markdown::block::CalloutKind, theme: &Theme) -> (Hsla, Hsla) {
    let style = theme.callout_style(variant);
    (style.border_color, style.background_color)
}



#[cfg(test)]
mod tests {
    use super::{
        RowSpacingInfo, callout_row_top_gap,
        row_top_gap,
    };
    use theme::{Theme, TypographyScope, TypographyStore};
    use markdown::parse::BlockId;
    use uuid::Uuid;

    #[test]
    fn contiguous_quote_rows_collapse_inter_row_gap() {
        let group = Uuid::new_v4();
        let gap = row_top_gap(
            Some(RowSpacingInfo {
                quote_group_id: Some(BlockId(group)),
                ..RowSpacingInfo::default()
            }),
            RowSpacingInfo {
                quote_group_id: Some(BlockId(group)),
                ..RowSpacingInfo::default()
            },
            4.0,
        );
        assert_eq!(gap, 0.0);
    }

    #[test]
    fn default_prose_font_keeps_lexend_as_primary_family() {
        let f = TypographyStore::default_font(TypographyScope::Prose);
        assert_eq!(f.family.to_string(), "Lexend");
        assert!(f.fallbacks.is_none(), "fallbacks should be None to rely on native OS glyph cascading");
    }

    #[test]
    fn nested_quote_separator_row_keeps_outer_group_gap_collapsed() {
        let group = Uuid::new_v4();
        let gap = row_top_gap(
            Some(RowSpacingInfo {
                quote_group_id: Some(BlockId(group)),
                ..RowSpacingInfo::default()
            }),
            RowSpacingInfo {
                quote_group_id: Some(BlockId(group)),
                ..RowSpacingInfo::default()
            },
            4.0,
        );
        assert_eq!(gap, 0.0);
    }

    #[test]
    fn distinct_quote_groups_keep_default_gap() {
        let gap = row_top_gap(
            Some(RowSpacingInfo {
                quote_group_id: Some(BlockId(Uuid::new_v4())),
                ..RowSpacingInfo::default()
            }),
            RowSpacingInfo {
                quote_group_id: Some(BlockId(Uuid::new_v4())),
                ..RowSpacingInfo::default()
            },
            4.0,
        );
        assert_eq!(gap, 4.0);
    }

    #[test]
    fn non_quote_rows_keep_default_gap() {
        let gap = row_top_gap(
            Some(RowSpacingInfo {
                quote_group_id: None,
                ..RowSpacingInfo::default()
            }),
            RowSpacingInfo {
                quote_group_id: Some(BlockId(Uuid::new_v4())),
                ..RowSpacingInfo::default()
            },
            4.0,
        );
        assert_eq!(gap, 4.0);
    }

    #[test]
    fn callout_inner_spacing_uses_header_and_body_tokens() {
        let theme = Theme::default_theme();
        let dimensions = &theme.dimensions;

        let header_gap = callout_row_top_gap(
            Some(RowSpacingInfo {
                is_callout_header: true,
                ..RowSpacingInfo::default()
            }),
            RowSpacingInfo::default(),
            dimensions,
        );
        let body_gap = callout_row_top_gap(
            Some(RowSpacingInfo {
                is_callout_header: false,
                ..RowSpacingInfo::default()
            }),
            RowSpacingInfo::default(),
            dimensions,
        );

        assert_eq!(header_gap, dimensions.callout_header_margin_bottom);
        assert_eq!(body_gap, dimensions.callout_body_gap);
    }

    #[test]
    fn nested_quote_rows_inside_callout_collapse_body_gap() {
        let theme = Theme::default_theme();
        let dimensions = &theme.dimensions;
        let group = Uuid::new_v4();

        let gap = callout_row_top_gap(
            Some(RowSpacingInfo {
                is_callout_header: false,
                visible_quote_group_id: Some(BlockId(group)),
                ..RowSpacingInfo::default()
            }),
            RowSpacingInfo {
                visible_quote_group_id: Some(BlockId(group)),
                ..RowSpacingInfo::default()
            },
            dimensions,
        );

        assert_eq!(gap, 0.0);
    }
}
