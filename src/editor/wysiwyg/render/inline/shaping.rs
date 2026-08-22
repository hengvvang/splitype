//! Inline text shaping and TextRun builder logic for wysiwyg rendering.

use gpui::*;

use crate::editor::render::code_highlight::highlight::code_highlight_color;
use crate::editor::tree::block::Block;
use crate::editor::wysiwyg::render::html_document::html_css_color_to_hsla;
use crate::infra::theme::ThemeColors;

/// The block's text-style line height without gpui's internal .round().
pub fn unrounded_line_height(window: &Window) -> Pixels {
    let style = window.text_style();
    style
        .line_height
        .to_pixels(style.font_size, window.rem_size())
}

pub fn build_text_runs(
    input: &Block,
    display_text: &SharedString,
    base_run: &TextRun,
    underline_thickness: Pixels,
    link_color: Hsla,
    marker_color: Hsla,
    footnote_color: Hsla,
    code_bg: Hsla,
    show_inline_code_backgrounds: bool,
) -> Vec<TextRun> {
    let spans = input.inline_spans();
    let delimiter_ranges = input.projected_delimiter_ranges();
    let mut boundaries = vec![0, display_text.len()];
    for span in spans {
        boundaries.push(span.range.start);
        boundaries.push(span.range.end);
    }
    for range in &delimiter_ranges {
        boundaries.push(range.start);
        boundaries.push(range.end);
    }
    if let Some(marked_range) = input.marked_range.as_ref() {
        boundaries.push(marked_range.start);
        boundaries.push(marked_range.end);
    }

    let footnote_def_id_range = if input.kind().is_footnote_definition() {
        if display_text.starts_with("[^") {
            display_text.find("]:").map(|end| 2..end)
        } else {
            display_text.find(':').map(|end| 0..end)
        }
    } else {
        None
    };
    if let Some(range) = footnote_def_id_range.as_ref() {
        boundaries.push(range.start);
        boundaries.push(range.end);
    }

    boundaries.sort_unstable();
    boundaries.dedup();

    let marked_range = input.marked_range.as_ref();
    let mut runs = Vec::new();
    let mut span_idx = 0usize;
    for boundary_pair in boundaries.windows(2) {
        let start = boundary_pair[0];
        let end = boundary_pair[1];
        if start >= end {
            continue;
        }

        while span_idx < spans.len() && spans[span_idx].range.end <= start {
            span_idx += 1;
        }
        let active_span = spans
            .get(span_idx)
            .filter(|span| span.range.start <= start && start < span.range.end);

        let inline_style = active_span.map(|s| s.style).unwrap_or_default();
        let html_style = active_span.and_then(|s| s.html_style);
        let is_link = active_span.map(|s| s.link.is_some()).unwrap_or(false);
        let is_footnote = active_span.map(|s| s.footnote.is_some()).unwrap_or(false);
        let is_footnote_id = footnote_def_id_range
            .as_ref()
            .map(|range| start >= range.start && end <= range.end)
            .unwrap_or(false);
        let is_delimiter = delimiter_ranges
            .iter()
            .any(|range| range.start <= start && end <= range.end);
        let is_marked = marked_range
            .map(|range| start < range.end && range.start < end)
            .unwrap_or(false);

        let mut font = base_run.font.clone();
        if inline_style.bold && font.weight < FontWeight::BOLD {
            font.weight = FontWeight::BOLD;
        }
        if inline_style.italic {
            font.style = FontStyle::Italic;
        }

        let mut run_color = if is_delimiter {
            marker_color
        } else if is_footnote || is_footnote_id {
            footnote_color
        } else if is_link {
            link_color
        } else {
            base_run.color
        };
        if let Some(style) = html_style
            && let Some(color) = style.color
        {
            run_color = html_css_color_to_hsla(color, run_color);
        }
        let underline =
            (inline_style.underline || is_marked || is_link).then_some(UnderlineStyle {
                color: Some(run_color),
                thickness: underline_thickness,
                wavy: false,
            });
        let strikethrough = inline_style.strikethrough.then_some(StrikethroughStyle {
            color: Some(run_color),
            thickness: underline_thickness,
        });

        let mut background_color = if show_inline_code_backgrounds && inline_style.code {
            Some(code_bg)
        } else {
            base_run.background_color
        };
        if let Some(style) = html_style
            && let Some(color) = style.background_color
        {
            background_color = Some(html_css_color_to_hsla(color, run_color));
        }

        runs.push(TextRun {
            len: end - start,
            font,
            color: run_color,
            background_color,
            underline,
            strikethrough,
        });
    }

    if runs.is_empty() {
        vec![base_run.clone()]
    } else {
        runs
    }
}

pub fn build_code_text_runs(
    input: &Block,
    display_text: &SharedString,
    base_run: &TextRun,
    underline_thickness: Pixels,
    colors: &ThemeColors,
) -> Vec<TextRun> {
    let highlight_spans = input
        .code_highlight_result()
        .map(|r| r.spans.as_slice())
        .unwrap_or(&[]);
    let mut boundaries = vec![0, display_text.len()];
    for span in highlight_spans {
        boundaries.push(span.range.start);
        boundaries.push(span.range.end);
    }
    if let Some(marked_range) = input.marked_range.as_ref() {
        boundaries.push(marked_range.start);
        boundaries.push(marked_range.end);
    }
    boundaries.sort_unstable();
    boundaries.dedup();

    let marked_range = input.marked_range.as_ref();
    let mut runs = Vec::new();
    let mut span_idx = 0usize;
    for boundary_pair in boundaries.windows(2) {
        let start = boundary_pair[0];
        let end = boundary_pair[1];
        if start >= end {
            continue;
        }

        let is_marked = marked_range
            .map(|range| start < range.end && range.start < end)
            .unwrap_or(false);
        while span_idx < highlight_spans.len() && highlight_spans[span_idx].range.end <= start {
            span_idx += 1;
        }
        let run_color = highlight_spans
            .get(span_idx)
            .filter(|span| span.range.start <= start && start < span.range.end)
            .map(|span| code_highlight_color(colors, span.class))
            .unwrap_or(base_run.color);

        runs.push(TextRun {
            len: end - start,
            font: base_run.font.clone(),
            color: run_color,
            background_color: base_run.background_color,
            underline: is_marked.then_some(UnderlineStyle {
                color: Some(run_color),
                thickness: underline_thickness,
                wavy: false,
            }),
            strikethrough: None,
        });
    }

    if runs.is_empty() {
        vec![base_run.clone()]
    } else {
        runs
    }
}
