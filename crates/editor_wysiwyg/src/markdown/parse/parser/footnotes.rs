//! Footnote definition and inline footnote head parser.

use super::helpers::*;
use super::pipeline::build_blocks_from_lines_internal;
use crate::markdown::block::footnote::{is_valid_footnote_id, parse_footnote_definition_head};
use crate::markdown::inline::text::BlockText;
use crate::markdown::parse::data::BlockData;
use crate::markdown::parse::indent::strip_leading_columns;
use crate::markdown::parse::kind::BlockKind;

pub(crate) fn build_native_footnote_definition_block(
    lines: &[String],
    mode: crate::markdown::parse::parser::ParseMode,
) -> Option<Vec<BlockData>> {
    let (id, first_line) = parse_footnote_definition_head(lines.first()?)?;
    // A definition line can carry several `[^id]:` heads on one line
    // (e.g. `[^a]: x [^b]: y`); split them into separate definitions. The
    // first content line of each lives in its block text as `id: content`,
    // so the rendered row shows the whole definition on a single editable
    // line; remaining lines become children of the last definition.
    let heads = split_inline_footnote_heads(id, &first_line);
    let mut body_lines = Vec::new();
    let mut seen_non_blank = false;
    for line in lines.iter().skip(1) {
        if line.trim().is_empty() {
            if seen_non_blank {
                body_lines.push(String::new());
            }
        } else {
            seen_non_blank = true;
            body_lines.push(
                strip_leading_columns(line, 4)
                    .unwrap_or(line.as_str())
                    .to_string(),
            );
        }
    }

    let mut children = build_blocks_from_lines_internal(&body_lines, mode, false);
    let head_count = heads.len();
    let mut result = Vec::new();
    for (index, (head_id, head_content)) in heads.into_iter().enumerate() {
        let mut text = BlockText::plain(format!("{head_id}: "));
        text.fragments
            .extend(BlockText::from_markdown(&head_content).fragments);
        let mut block = BlockData::new(BlockKind::FootnoteDefinition, text);
        if index + 1 == head_count {
            attach_child_blocks(&mut block, &mut children);
        }
        result.push(block);
    }
    result.extend(children);
    Some(result)
}

pub(crate) fn find_footnote_head_outside_code(text: &str) -> Option<(usize, usize, &str)> {
    let mut in_code = false;
    let mut backtick_count = 0usize;
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let start = i;
            while i < bytes.len() && bytes[i] == b'`' {
                i += 1;
            }
            let run_len = i - start;
            if in_code {
                if run_len == backtick_count {
                    in_code = false;
                    backtick_count = 0;
                }
            } else {
                in_code = true;
                backtick_count = run_len;
            }
            continue;
        }

        if !in_code && i + 2 <= bytes.len() && &bytes[i..i + 2] == b"[^" {
            let after_open = &text[i + 2..];
            if let Some(close) = after_open.find("]:") {
                let id = &after_open[..close];
                if is_valid_footnote_id(id) {
                    return Some((i, i + 2 + close + 2, id));
                }
            }
        }
        i += 1;
    }
    None
}

/// Splits a definition head line that contains additional inline `[^id]:`
/// heads (e.g. `[^a]: x [^b]: y`) into one `(id, content)` pair per head.
pub(crate) fn split_inline_footnote_heads(id: String, first_line: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut current_id = id;
    let mut rest = first_line;
    loop {
        let Some((open, close_end, next_id)) = find_footnote_head_outside_code(rest) else {
            result.push((current_id, rest.to_string()));
            break;
        };
        result.push((current_id, rest[..open].trim_end().to_string()));
        current_id = next_id.to_string();
        rest = rest[close_end..].trim_start();
    }
    result
}
