//! Markdown block parsing pipeline orchestrator.

use super::code_and_text::{
    collect_comment_block, collect_fenced_code_block, collect_indented_code_block,
    collect_paragraph_block,
};
use super::footnotes::build_native_footnote_definition_block;
use super::helpers::*;
use super::lists::collect_list_blocks;
use super::quotes::collect_quote_block;
use crate::block::image::parse_standalone_image;
use crate::block::table::{
    collect_pipeless_table_region, collect_table_candidate_region, is_table_candidate_line,
    parse_table_region,
};
use crate::parse::data::BlockData;
use crate::parse::indent::{is_quote_start, strip_indented_code_prefix};
use crate::parse::kind::BlockKind;

pub fn parse_document(markdown: &str) -> Vec<BlockData> {
    if markdown.is_empty() {
        return Vec::new();
    }
    let lines = markdown
        .lines()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    build_blocks_from_lines_internal(&lines, true)
}

/// Build blocks from pre-split Markdown lines.
///
/// Equivalent to the editor's `build_blocks_from_lines`.
pub fn build_blocks_from_lines(lines: &[String]) -> Vec<BlockData> {
    build_blocks_from_lines_internal(lines, true)
}

/// Internal dispatch: walk every line and emit native blocks or raw fallbacks.
pub(crate) fn build_blocks_from_lines_internal(
    lines: &[String],
    allow_root_footnote_definitions: bool,
) -> Vec<BlockData> {
    let mut roots = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let line = &lines[index];
        if line.trim().is_empty() {
            roots.push(native_block(BlockKind::Paragraph, ""));
            index += 1;
            continue;
        }

        if parse_opening_fence(line).is_some() {
            let Some((block, next_index)) = collect_fenced_code_block(lines, index) else {
                let paragraph = collect_paragraph_block(lines, index);
                roots.push(paragraph.0);
                index = paragraph.1;
                continue;
            };

            roots.push(block);
            index = next_index;
            continue;
        }

        if let Some((block, end)) = collect_comment_block(lines, index) {
            roots.push(block);
            index = end;
            continue;
        }

        if is_block_html_start(line) {
            let end = collect_block_html_region(lines, index);
            roots.push(html_or_raw_block(lines[index..end].join("\n")));
            index = end;
            continue;
        }

        if is_footnote_definition_start(line) {
            let end = collect_footnote_definition_region(lines, index);
            if allow_root_footnote_definitions {
                if let Some(mut blocks) = build_native_footnote_definition_block(&lines[index..end])
                {
                    roots.append(&mut blocks);
                } else {
                    roots.push(raw_block(lines[index..end].join("\n")));
                }
            } else {
                roots.push(raw_block(lines[index..end].join("\n")));
            }
            index = end;
            continue;
        }

        if is_reference_definition_start(line) {
            let end = collect_reference_definition_region(lines, index);
            roots.push(raw_block(lines[index..end].join("\n")));
            index = end;
            continue;
        }

        if let Some(level) = lines
            .get(index + 1)
            .and_then(|next| BlockKind::parse_setext_underline(next))
        {
            roots.push(native_block(BlockKind::Heading { level }, line.trim_end()));
            index += 2;
            continue;
        }

        if parse_standalone_image(line).is_some() {
            roots.push(standalone_image_block(line.to_string()));
            index += 1;
            continue;
        }

        if strip_indented_code_prefix(line).is_some() {
            let Some((block, next_index)) = collect_indented_code_block(lines, index) else {
                unreachable!("indented code prefix disappeared after detection");
            };

            roots.push(block);
            index = next_index;
            continue;
        }

        if parse_list_marker(line).is_some() {
            let (mut blocks, next_index) = collect_list_blocks(lines, index);
            roots.append(&mut blocks);
            index = next_index;
            continue;
        }

        if is_quote_start(line) {
            let (mut blocks, next_index) = collect_quote_block(lines, index);
            roots.append(&mut blocks);
            index = next_index;
            continue;
        }

        if let Some((level, content)) = BlockKind::parse_atx_heading_line(line) {
            roots.push(native_block(BlockKind::Heading { level }, &content));
            index += 1;
            continue;
        }

        if BlockKind::parse_thematic_break_line(line) {
            roots.push(BlockData::with_plain_text(
                BlockKind::ThematicBreak,
                line.to_string(),
            ));
            index += 1;
            continue;
        }

        if is_table_candidate_line(line) {
            let end = collect_table_candidate_region(lines, index);
            let region = &lines[index..end];
            if let Some(table) = parse_table_region(region) {
                roots.push(BlockData::table(table));
            } else {
                roots.extend(region.iter().cloned().map(plain_text_paragraph_block));
            }
            index = end;
            continue;
        }

        if let Some(end) = collect_pipeless_table_region(lines, index)
            && let Some(table) = parse_table_region(&lines[index..end])
        {
            roots.push(BlockData::table(table));
            index = end;
            continue;
        }

        if is_display_math_start(line) {
            let end = collect_display_math_region(lines, index);
            roots.push(math_or_raw_block(lines[index..end].join("\n")));
            index = end;
            continue;
        }

        let paragraph = collect_paragraph_block(lines, index);
        roots.push(paragraph.0);
        index = paragraph.1;
    }

    roots
}
