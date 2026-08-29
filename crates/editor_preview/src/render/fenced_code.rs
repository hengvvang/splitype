//! Preview fenced code rendering — highlighted code with line numbers,
//! mirroring the WYSIWYG code block styles.

use gpui::*;

use editor_wysiwyg::highlight::{
    code_highlight_color, highlight_code_block,
};
use crate::node::PreviewBlock;
use theme::Theme;

/// Extracts the fence language tag from the raw source (e.g. `rust` from
/// ```rust ... ```), mirroring the WYSIWYG language resolution.
fn fence_language(block: &PreviewBlock) -> Option<String> {
    let raw = block.data.raw_source.as_deref().unwrap_or_default();
    let trimmed = raw.trim_start();
    let rest = trimmed.strip_prefix("```")?;
    let first_line = rest.split('\n').next().unwrap_or("");
    let language = first_line.trim();
    if language.is_empty() {
        None
    } else {
        Some(language.to_string())
    }
}

/// Renders a fenced code block read-only with syntax highlighting.
pub(crate) fn render_preview_fenced_code(block: &PreviewBlock, base: Div, theme: &Theme) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    let source = block.display_text();
    let language = fence_language(block);
    let highlighted = highlight_code_block(language.as_deref(), &source);

    let line_count = source.split('\n').count().max(1);
    let line_numbers_text = (1..=line_count)
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join("\n");

    let code_body = if let Some(result) = highlighted {
        render_highlighted_lines(&source, &result, c.code_text, theme)
    } else {
        div()
            .text_size(px(t.code_size))
            .text_color(c.code_text)
            .line_height(rems(t.text_line_height))
            .child(SharedString::from(source.to_string()))
            .into_any_element()
    };

    base.w_full()
        .child(
            div()
                .w_full()
                .font(theme::TypographyStore::default_font(
                    theme::TypographyScope::Code,
                ))
                .bg(c.code_bg)
                .rounded(px(d.code_block_radius))
                .px(px(d.code_block_padding_x))
                .py(px(d.code_block_padding_y))
                .text_size(px(t.code_size))
                .text_color(c.code_text)
                .line_height(rems(t.text_line_height))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .child(
                            div()
                                .flex_none()
                                .pr(px(10.0))
                                .mr(px(8.0))
                                .border_r_1()
                                .border_color(c.table_border)
                                .text_align(TextAlign::Right)
                                .text_size(px(t.code_size))
                                .line_height(rems(t.text_line_height))
                                .text_color(c.dialog_muted)
                                .child(SharedString::from(line_numbers_text)),
                        )
                        .child(div().min_w(px(0.0)).flex_1().child(code_body)),
                ),
        )
        .into_any_element()
}

/// Renders each source line with per-span highlight colors.
fn render_highlighted_lines(
    source: &str,
    result: &editor_wysiwyg::highlight::CodeHighlightResult,
    default_color: Hsla,
    theme: &Theme,
) -> AnyElement {
    let c = &theme.colors;
    let t = &theme.typography;

    let lines: Vec<AnyElement> = source
        .split('\n')
        .enumerate()
        .map(|(line_index, line)| {
            let line_start = source
                .split('\n')
                .take(line_index)
                .map(|l| l.len() + 1)
                .sum::<usize>();
            let line_end = line_start + line.len();

            let mut segments: Vec<AnyElement> = Vec::new();
            let mut cursor = 0usize;
            for span in result
                .spans
                .iter()
                .filter(|span| span.range.start < line_end && span.range.end > line_start)
            {
                let local_start = span.range.start.saturating_sub(line_start);
                let local_end = (span.range.end - line_start).min(line.len());
                if local_start > cursor {
                    segments.push(
                        div()
                            .child(SharedString::from(line[cursor..local_start].to_string()))
                            .into_any_element(),
                    );
                }
                if local_end > local_start {
                    segments.push(
                        div()
                            .text_color(code_highlight_color(c, span.class))
                            .child(SharedString::from(line[local_start..local_end].to_string()))
                            .into_any_element(),
                    );
                }
                cursor = local_end.max(cursor);
            }
            if cursor < line.len() {
                segments.push(
                    div()
                        .child(SharedString::from(line[cursor..].to_string()))
                        .into_any_element(),
                );
            }

            div()
                .flex()
                .flex_row()
                .text_size(px(t.code_size))
                .text_color(default_color)
                .line_height(rems(t.text_line_height))
                .children(segments)
                .into_any_element()
        })
        .collect();

    div().flex().flex_col().children(lines).into_any_element()
}
