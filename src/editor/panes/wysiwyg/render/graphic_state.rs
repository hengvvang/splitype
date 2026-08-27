use gpui::*;

use crate::editor::panes::wysiwyg::render::layout::editor_text_font;
use crate::infra::theme::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GraphicKind {
    LatexMath,
    Mermaid,
}

impl GraphicKind {
    pub(crate) fn empty_title(&self) -> &'static str {
        match self {
            GraphicKind::LatexMath => "Empty LaTeX Math",
            GraphicKind::Mermaid => "Empty Mermaid Diagram",
        }
    }

    pub(crate) fn error_header(&self) -> &'static str {
        match self {
            GraphicKind::LatexMath => "LaTeX error(s):",
            GraphicKind::Mermaid => "Mermaid error(s):",
        }
    }
}

fn clean_error_message(err: &str) -> String {
    let trimmed = err.trim();
    let stripped = trimmed
        .strip_prefix("Error:")
        .or_else(|| trimmed.strip_prefix("error:"))
        .unwrap_or(trimmed)
        .trim();
    if stripped.is_empty() {
        "syntax parsing failed".to_string()
    } else {
        stripped.to_string()
    }
}

pub(crate) fn render_empty_graphic_placeholder(
    kind: GraphicKind,
    theme: &Theme,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    div()
        .w_full()
        .flex()
        .items_center()
        .justify_center()
        .py(px(d.block_padding_y.max(8.0)))
        .child(
            div()
                .text_size(px(t.text_size))
                .font_weight(FontWeight::MEDIUM)
                .text_color(c.image_placeholder_text)
                .child(SharedString::from(kind.empty_title())),
        )
        .into_any_element()
}

pub(crate) fn render_graphic_error_card(
    kind: GraphicKind,
    error: &str,
    _raw_source: &str,
    theme: &Theme,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    let cleaned_error = clean_error_message(error);
    let error_color = c.callout_caution_border;

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .py(px(d.block_padding_y.max(4.0)))
        .child(
            div()
                .text_size(px(t.text_size))
                .font_weight(FontWeight::MEDIUM)
                .text_color(error_color)
                .child(SharedString::from(kind.error_header())),
        )
        .child(
            div()
                .w_full()
                .pl(px(16.0))
                .text_size(px(t.code_size))
                .font_weight(FontWeight::NORMAL)
                .font_family(editor_text_font().family)
                .text_color(c.text_default)
                .child(SharedString::from(cleaned_error)),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graphic_kind_titles_and_headers() {
        assert_eq!(GraphicKind::LatexMath.empty_title(), "Empty LaTeX Math");
        assert_eq!(GraphicKind::Mermaid.empty_title(), "Empty Mermaid Diagram");
        assert_eq!(GraphicKind::LatexMath.error_header(), "LaTeX error(s):");
        assert_eq!(GraphicKind::Mermaid.error_header(), "Mermaid error(s):");
    }

    #[test]
    fn test_clean_error_message() {
        assert_eq!(clean_error_message("Error: invalid token"), "invalid token");
        assert_eq!(clean_error_message("  error: parse failed  "), "parse failed");
        assert_eq!(clean_error_message(""), "syntax parsing failed");
    }
}
