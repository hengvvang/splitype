//! Code fence, indented code, HTML comment, and paragraph collectors.

use super::helpers::*;
use crate::markdown::block::mermaid::is_mermaid_info_string;
use crate::markdown::parse::data::BlockData;
use crate::markdown::parse::indent::strip_indented_code_prefix;
use crate::markdown::parse::kind::BlockKind;

pub(crate) fn collect_fenced_code_block(
    lines: &[String],
    start: usize,
) -> Option<(BlockData, usize)> {
    let fence = parse_opening_fence(&lines[start])?;
    let (closing_index, is_closed) = match find_matching_closing_fence(lines, start, &fence) {
        Some(idx) => (idx, true),
        None => (lines.len().saturating_sub(1), false),
    };
    if is_mermaid_info_string(fence.language.as_ref().map(|language| language.as_ref())) {
        let raw = lines[start..=closing_index].join("\n");
        return Some((BlockData::mermaid_block(raw), closing_index + 1));
    }

    let code_lines = if is_closed {
        lines[start + 1..closing_index].to_vec()
    } else if start + 1 <= lines.len() {
        lines[start + 1..].to_vec()
    } else {
        Vec::new()
    };
    Some((
        build_code_block(fence.language.clone(), code_lines.join("\n")),
        closing_index + 1,
    ))
}

pub(crate) fn collect_indented_code_block(
    lines: &[String],
    start: usize,
) -> Option<(BlockData, usize)> {
    let stripped = strip_indented_code_prefix(&lines[start])?;
    let mut code_lines = vec![stripped.to_string()];
    let mut code_index = start + 1;
    while code_index < lines.len() {
        if let Some(stripped) = strip_indented_code_prefix(&lines[code_index]) {
            code_lines.push(stripped.to_string());
            code_index += 1;
        } else if lines[code_index].trim().is_empty() {
            code_lines.push(String::new());
            code_index += 1;
        } else {
            break;
        }
    }

    Some((build_code_block(None, code_lines.join("\n")), code_index))
}

pub(crate) fn collect_comment_block(lines: &[String], start: usize) -> Option<(BlockData, usize)> {
    let end = collect_closed_html_comment_region(lines, start)?;
    Some((comment_block(lines[start..end].join("\n")), end))
}

pub(crate) fn collect_paragraph_block(lines: &[String], start: usize) -> (BlockData, usize) {
    let mut paragraph_lines = vec![lines[start].to_string()];
    let mut index = start + 1;
    while index < lines.len() {
        if (lines[index].trim().is_empty() || looks_like_root_block_start(lines, index))
            && !paragraph_can_continue_through_boundary(&paragraph_lines, lines, index)
        {
            break;
        }
        paragraph_lines.push(lines[index].to_string());
        index += 1;
    }

    (
        native_block(BlockKind::Paragraph, &paragraph_lines.join("\n")),
        index,
    )
}
