//! WYSIWYG panel — the rendered block view.
//!
//! Row layout and spacing helpers used by the main render pass; the render
//! orchestration itself lives in `crate::editor::view`.

use gpui::*;

use crate::editor::controller::*;
use crate::infra::theme::*;

// ── Spacing helpers ─────────────────────────────────────────────────────

/// Row-level spacing metadata read from a [`Block`] entity once per frame.
///
/// Used to decide whether consecutive rows belong to the same visual group
/// (quote, callout, footnote) and should collapse their inter-row gap.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RowSpacingInfo {
    pub quote_group_anchor: Option<BlockId>,
    pub visible_quote_group_anchor: Option<BlockId>,
    pub callout_anchor: Option<BlockId>,
    pub callout_variant: Option<crate::model::block::CalloutKind>,
    pub is_callout_header: bool,
    pub footnote_anchor: Option<BlockId>,
    pub is_footnote_header: bool,
}

impl RowSpacingInfo {
    /// Read spacing metadata from a block entity.
    pub fn from_block(block: &crate::editor::tree::block::Block) -> Self {
        Self {
            quote_group_anchor: block.quote_group_anchor,
            visible_quote_group_anchor: block.visible_quote_group_anchor,
            callout_anchor: block.callout_anchor,
            callout_variant: block.callout_variant,
            is_callout_header: block.kind().is_callout(),
            footnote_anchor: block.footnote_anchor,
            is_footnote_header: block.kind().is_footnote_definition(),
        }
    }
}

/// Gap between two consecutive rendered rows.
///
/// Returns 0 for rows that share a quote group; otherwise returns the
/// default block gap from the theme.
pub fn row_top_gap(
    previous: Option<RowSpacingInfo>,
    current: RowSpacingInfo,
    default_gap: f32,
) -> f32 {
    let Some(previous) = previous else {
        return 0.0;
    };
    if previous.quote_group_anchor.is_some()
        && previous.quote_group_anchor == current.quote_group_anchor
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
    if previous.visible_quote_group_anchor.is_some()
        && previous.visible_quote_group_anchor == current.visible_quote_group_anchor
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
pub fn callout_colors(variant: crate::model::block::CalloutKind, theme: &Theme) -> (Hsla, Hsla) {
    let c = &theme.colors;
    match variant {
        crate::model::block::CalloutKind::Note => (c.callout_note_border, c.callout_note_bg),
        crate::model::block::CalloutKind::Tip => (c.callout_tip_border, c.callout_tip_bg),
        crate::model::block::CalloutKind::Important => {
            (c.callout_important_border, c.callout_important_bg)
        }
        crate::model::block::CalloutKind::Warning => {
            (c.callout_warning_border, c.callout_warning_bg)
        }
        crate::model::block::CalloutKind::Caution => {
            (c.callout_caution_border, c.callout_caution_bg)
        }
    }
}

// ── Font helpers ────────────────────────────────────────────────────────

/// The editor's text font with Tibetan fallbacks for the target OS.
pub fn editor_text_font() -> Font {
    static FALLBACKS: std::sync::OnceLock<FontFallbacks> = std::sync::OnceLock::new();
    let fallbacks = FALLBACKS
        .get_or_init(|| {
            FontFallbacks::from_fonts(tibetan_font_fallbacks_for_target_os(std::env::consts::OS))
        })
        .clone();
    let mut font = font(".SystemUIFont");
    font.fallbacks = Some(fallbacks);
    font
}

/// Return Tibetan-script font families for the given OS.
pub fn tibetan_font_fallbacks_for_target_os(target_os: &str) -> Vec<String> {
    let families: &[&str] = match target_os {
        "windows" => &[
            "Microsoft Himalaya",
            "Noto Serif Tibetan",
            "Noto Sans Tibetan",
            "BabelStone Tibetan",
        ],
        "macos" => &["Kailasa", "Noto Serif Tibetan", "Noto Sans Tibetan"],
        _ => &[
            "Noto Serif Tibetan",
            "Noto Sans Tibetan",
            "Microsoft Himalaya",
            "Kailasa",
            "BabelStone Tibetan",
        ],
    };
    families.iter().map(|f| (*f).to_string()).collect()
}

// ── Empty panel prompt ──────────────────────────────────────────────────

/// A centered muted-text prompt for empty tiled panels (explorer files,
/// outline).

#[cfg(test)]
mod tests {
    use super::{
        RowSpacingInfo, callout_row_top_gap, editor_text_font, row_top_gap,
        tibetan_font_fallbacks_for_target_os,
    };
    use crate::infra::theme::Theme;
    use crate::model::block::BlockId;
    use uuid::Uuid;

    #[test]
    fn contiguous_quote_rows_collapse_inter_row_gap() {
        let group = Uuid::new_v4();
        let gap = row_top_gap(
            Some(RowSpacingInfo {
                quote_group_anchor: Some(BlockId(group)),
                ..RowSpacingInfo::default()
            }),
            RowSpacingInfo {
                quote_group_anchor: Some(BlockId(group)),
                ..RowSpacingInfo::default()
            },
            4.0,
        );
        assert_eq!(gap, 0.0);
    }

    #[test]
    fn editor_text_font_keeps_system_ui_as_primary_family() {
        assert_eq!(editor_text_font().family.to_string(), ".SystemUIFont");
    }

    #[test]
    fn tibetan_font_fallbacks_prioritize_platform_defaults() {
        assert_eq!(
            tibetan_font_fallbacks_for_target_os("windows")
                .first()
                .map(String::as_str),
            Some("Microsoft Himalaya")
        );
        assert_eq!(
            tibetan_font_fallbacks_for_target_os("macos")
                .first()
                .map(String::as_str),
            Some("Kailasa")
        );
        assert_eq!(
            tibetan_font_fallbacks_for_target_os("linux")
                .first()
                .map(String::as_str),
            Some("Noto Serif Tibetan")
        );
        assert_eq!(
            tibetan_font_fallbacks_for_target_os("unknown")
                .first()
                .map(String::as_str),
            Some("Noto Serif Tibetan")
        );
    }

    #[test]
    fn nested_quote_separator_row_keeps_outer_group_gap_collapsed() {
        let group = Uuid::new_v4();
        let gap = row_top_gap(
            Some(RowSpacingInfo {
                quote_group_anchor: Some(BlockId(group)),
                ..RowSpacingInfo::default()
            }),
            RowSpacingInfo {
                quote_group_anchor: Some(BlockId(group)),
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
                quote_group_anchor: Some(BlockId(Uuid::new_v4())),
                ..RowSpacingInfo::default()
            }),
            RowSpacingInfo {
                quote_group_anchor: Some(BlockId(Uuid::new_v4())),
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
                quote_group_anchor: None,
                ..RowSpacingInfo::default()
            }),
            RowSpacingInfo {
                quote_group_anchor: Some(BlockId(Uuid::new_v4())),
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
                visible_quote_group_anchor: Some(BlockId(group)),
                ..RowSpacingInfo::default()
            }),
            RowSpacingInfo {
                visible_quote_group_anchor: Some(BlockId(group)),
                ..RowSpacingInfo::default()
            },
            dimensions,
        );

        assert_eq!(gap, 0.0);
    }
}
