//! Code fence, indented code, HTML comment, and paragraph collectors.

use super::helpers::*;
use super::lines::Lines;
use crate::block::mermaid::is_mermaid_info_string;
use crate::parse::data::BlockData;
use crate::parse::indent::strip_indented_code_prefix;
use crate::parse::kind::BlockKind;

pub(crate) fn collect_fenced_code_block<L: Lines + ?Sized>(
    lines: &L,
    start: usize,
) -> Option<(BlockData, usize)> {
    let fence = parse_opening_fence(lines.line(start))?;
    let (closing_index, is_closed) = match find_matching_closing_fence(lines, start, &fence) {
        Some(idx) => (idx, true),
        None => (lines.line_count().saturating_sub(1), false),
    };
    if is_mermaid_info_string(fence.language.as_ref().map(|language| language.as_ref())) {
        let raw = (start..=closing_index)
            .map(|i| lines.line(i))
            .collect::<Vec<_>>()
            .join("\n");
        return Some((BlockData::mermaid_block(raw), closing_index + 1));
    }

    let code_end = if is_closed {
        closing_index
    } else if start < lines.line_count() {
        lines.line_count()
    } else {
        start + 1
    };
    Some((
        build_code_block(
            fence.language.clone(),
            (start + 1..code_end)
                .map(|i| lines.line(i))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        closing_index + 1,
    ))
}

pub(crate) fn collect_indented_code_block<L: Lines + ?Sized>(
    lines: &L,
    start: usize,
) -> Option<(BlockData, usize)> {
    let stripped = strip_indented_code_prefix(lines.line(start))?;
    let mut code_lines = vec![stripped.to_string()];
    let mut code_index = start + 1;
    while code_index < lines.line_count() {
        if let Some(stripped) = strip_indented_code_prefix(lines.line(code_index)) {
            code_lines.push(stripped.to_string());
            code_index += 1;
        } else if lines.line(code_index).trim().is_empty() {
            code_lines.push(String::new());
            code_index += 1;
        } else {
            break;
        }
    }

    Some((build_code_block(None, code_lines.join("\n")), code_index))
}

pub(crate) fn collect_comment_block<L: Lines + ?Sized>(
    lines: &L,
    start: usize,
) -> Option<(BlockData, usize)> {
    let end = collect_closed_html_comment_region(lines, start)?;
    Some((
        comment_block(
            (start..end)
                .map(|i| lines.line(i))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        end,
    ))
}

pub(crate) fn collect_paragraph_block<L: Lines + ?Sized>(
    lines: &L,
    start: usize,
) -> (BlockData, usize) {
    let mut paragraph_lines = vec![lines.line(start)];
    let mut index = start + 1;
    while index < lines.line_count() {
        if (lines.line(index).trim().is_empty() || looks_like_root_block_start(lines, index))
            && !paragraph_can_continue_through_boundary(&paragraph_lines, lines, index)
        {
            break;
        }
        paragraph_lines.push(lines.line(index));
        index += 1;
    }

    (
        native_block(BlockKind::Paragraph, &paragraph_lines.join("\n")),
        index,
    )
}
