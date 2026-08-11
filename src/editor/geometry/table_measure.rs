//! Table column width measurement — preferred-width estimation using the
//! window's text system.
//!
//! Text shaping requires a live `Window` and theme dimensions, so this
//! presentation logic lives in `editor::geometry` instead of the pure
//! `splitype_model::block::table` data model. The layout *math*
//! (`from_preferred_widths`) stays in the model, keeping the model testable
//! without a runtime. It is a free function (not an `impl` on
//! `TableColumnLayout`) because the type is defined in `splitype-model`.

use gpui::{FontStyle, FontWeight, Pixels, SharedString, TextRun, Window, px};

use crate::infra::theme::Theme;
use crate::model::inline::render_cache::InlineRenderCache;
use crate::model::inline::text::BlockText;
use crate::model::block::table::{TableColumnLayout, TableData};

/// Measure preferred column widths with the window's text system and
/// normalize them to fractions of the available table width.
pub(crate) fn measure_table_column_layout(
    table: &TableData,
    table_width: f32,
    window: &mut Window,
    theme: &Theme,
) -> TableColumnLayout {
    let preferred_widths = measure_preferred_column_widths(table, window, theme)
        .into_iter()
        .map(f32::from)
        .collect::<Vec<_>>();
    TableColumnLayout::from_preferred_widths(
        &preferred_widths,
        table_width,
        minimum_column_width(theme),
    )
}

fn measure_preferred_column_widths(
    table: &TableData,
    window: &mut Window,
    theme: &Theme,
) -> Vec<Pixels> {
    let column_count = table.header.len().max(1);
    let mut preferred_widths = vec![Pixels::ZERO; column_count];

    for (column, cell) in table.header.iter().enumerate() {
        preferred_widths[column] =
            preferred_widths[column].max(measure_cell_preferred_width(cell, true, window, theme));
    }

    for row in &table.rows {
        for (column, cell) in row.iter().enumerate().take(column_count) {
            preferred_widths[column] = preferred_widths[column]
                .max(measure_cell_preferred_width(cell, false, window, theme));
        }
    }

    preferred_widths
}

fn measure_cell_preferred_width(
    cell: &BlockText,
    is_header: bool,
    window: &mut Window,
    theme: &Theme,
) -> Pixels {
    let cache = cell.render_cache();
    let text = cache.text();
    let cell_chrome_width = cell_chrome_width(theme);
    if text.is_empty() {
        return cell_chrome_width;
    }

    let display_text = SharedString::from(text.to_string());
    let mut font = window.text_style().font();
    if is_header && font.weight < FontWeight::MEDIUM {
        font.weight = FontWeight::MEDIUM;
    }
    let base_run = TextRun {
        len: display_text.len(),
        font,
        color: theme.colors.text_default,
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let runs = measurement_runs(&cache, &base_run);
    let font_size = px(theme.typography.text_size);

    let text_width = window
        .text_system()
        .shape_text(display_text, font_size, &runs, None, None)
        .ok()
        .map(|lines| {
            lines
                .iter()
                .map(|line| line.width())
                .max()
                .unwrap_or(Pixels::ZERO)
        })
        .unwrap_or(Pixels::ZERO);

    text_width + cell_chrome_width
}

fn measurement_runs(cache: &InlineRenderCache, base_run: &TextRun) -> Vec<TextRun> {
    let mut boundaries = vec![0, cache.text().len()];
    for span in cache.spans() {
        boundaries.push(span.range.start);
        boundaries.push(span.range.end);
    }
    boundaries.sort_unstable();
    boundaries.dedup();

    let mut runs = Vec::new();
    for boundary_pair in boundaries.windows(2) {
        let start = boundary_pair[0];
        let end = boundary_pair[1];
        if start >= end {
            continue;
        }

        let inline_style = cache.style_at(start);
        let mut font = base_run.font.clone();
        if inline_style.bold && font.weight < FontWeight::BOLD {
            font.weight = FontWeight::BOLD;
        }
        if inline_style.italic {
            font.style = FontStyle::Italic;
        }

        runs.push(TextRun {
            len: end - start,
            font,
            color: base_run.color,
            background_color: None,
            underline: None,
            strikethrough: None,
        });
    }

    if runs.is_empty() {
        vec![base_run.clone()]
    } else {
        runs
    }
}

fn cell_chrome_width(theme: &Theme) -> Pixels {
    px(theme.dimensions.table_cell_padding_x * 2.0 + 2.0)
}

fn minimum_column_width(theme: &Theme) -> f32 {
    theme.dimensions.table_cell_padding_x * 2.0 + theme.typography.text_size * 4.0 + 2.0
}
