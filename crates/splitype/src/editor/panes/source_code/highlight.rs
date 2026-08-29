use std::ops::Range;

use gpui::{Font, TextRun};

use syntax::highlight::{
    CodeHighlightSpan, code_highlight_color,
};
use theme::ThemeColors;

/// Builds a sequence of `TextRun`s for a single line using Tree-sitter highlight spans.
pub(crate) fn build_line_text_runs(
    line_text: &str,
    line_range: Range<usize>,
    spans: &[CodeHighlightSpan],
    font: Font,
    theme_colors: &ThemeColors,
) -> Vec<TextRun> {
    if line_text.is_empty() {
        return Vec::new();
    }

    if spans.is_empty() {
        return vec![TextRun {
            len: line_text.len(),
            font,
            color: theme_colors.text_default,
            ..Default::default()
        }];
    }

    let l_start = line_range.start;
    let l_end = line_range.end;
    let mut runs = Vec::new();
    let mut current_offset = 0; // relative to line start (0..line_text.len())

    for span in spans {
        if span.range.end <= l_start || span.range.start >= l_end {
            continue;
        }

        let span_local_start = span.range.start.saturating_sub(l_start).min(line_text.len());
        let span_local_end = span.range.end.saturating_sub(l_start).min(line_text.len());

        if span_local_start > current_offset {
            let gap_len = span_local_start - current_offset;
            runs.push(TextRun {
                len: gap_len,
                font: font.clone(),
                color: theme_colors.text_default,
                ..Default::default()
            });
            current_offset = span_local_start;
        }

        if span_local_end > current_offset {
            let seg_len = span_local_end - current_offset;
            let color = code_highlight_color(theme_colors, span.class);
            runs.push(TextRun {
                len: seg_len,
                font: font.clone(),
                color,
                ..Default::default()
            });
            current_offset = span_local_end;
        }
    }

    if current_offset < line_text.len() {
        runs.push(TextRun {
            len: line_text.len() - current_offset,
            font: font.clone(),
            color: theme_colors.text_default,
            ..Default::default()
        });
    }

    if runs.is_empty() {
        runs.push(TextRun {
            len: line_text.len(),
            font,
            color: theme_colors.text_default,
            ..Default::default()
        });
    }

    runs
}
