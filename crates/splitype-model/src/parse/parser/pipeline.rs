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

/// Parsing mode for Markdown documents.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ParseMode {
    /// WYSIWYG mode (editing-first): 1 source line maps 1:1 to 1 block, empty lines are independent empty blocks.
    #[default]
    Wysiwyg,
    /// Preview mode (standard-first): merges consecutive lines into single <p> paragraphs adhering 100% to CommonMark.
    Preview,
}

pub fn parse_document(markdown: &str) -> Vec<BlockData> {
    parse_document_with_mode(markdown, ParseMode::Wysiwyg)
}

pub fn parse_wysiwyg_document(markdown: &str) -> Vec<BlockData> {
    parse_document_with_mode(markdown, ParseMode::Wysiwyg)
}

pub fn parse_preview_document(markdown: &str) -> Vec<BlockData> {
    parse_document_with_mode(markdown, ParseMode::Preview)
}

pub fn parse_document_with_mode(markdown: &str, mode: ParseMode) -> Vec<BlockData> {
    if markdown.is_empty() {
        return Vec::new();
    }
    let lines = markdown
        .lines()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    build_blocks_from_lines_internal(&lines, mode, true)
}

/// Build blocks from pre-split Markdown lines.
///
/// Equivalent to the editor's `build_blocks_from_lines`.
pub fn build_blocks_from_lines(lines: &[String]) -> Vec<BlockData> {
    build_wysiwyg_blocks_from_lines(lines)
}

pub fn build_wysiwyg_blocks_from_lines(lines: &[String]) -> Vec<BlockData> {
    build_blocks_from_lines_internal(lines, ParseMode::Wysiwyg, true)
}

pub fn build_preview_blocks_from_lines(lines: &[String]) -> Vec<BlockData> {
    build_blocks_from_lines_internal(lines, ParseMode::Preview, true)
}

/// Internal dispatch: walk every line and emit native blocks or raw fallbacks.
pub(crate) fn build_blocks_from_lines_internal(
    lines: &[String],
    mode: ParseMode,
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
            if let Some((block, next_index)) = collect_fenced_code_block(lines, index) {
                roots.push(block);
                index = next_index;
                continue;
            }
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
                if let Some(mut blocks) =
                    build_native_footnote_definition_block(&lines[index..end], mode)
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

        if let Some(next) = lines.get(index + 1)
            && parse_list_marker(next).is_none()
            && let Some(level) = BlockKind::parse_setext_underline(next)
        {
            roots.push(native_block(BlockKind::Heading { level }, line.trim_end()));
            index += 2;
            continue;
        }

        match mode {
            ParseMode::Wysiwyg => {
                roots.push(native_block(BlockKind::Paragraph, line));
                index += 1;
            }
            ParseMode::Preview => {
                let paragraph = collect_paragraph_block(lines, index);
                roots.push(paragraph.0);
                index = paragraph.1;
            }
        }
    }

    roots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wysiwyg_does_not_merge_consecutive_lines_and_keeps_empty_blocks() {
        let markdown = "Line 1\nLine 2\n\nLine 3";
        let blocks = parse_wysiwyg_document(markdown);
        assert_eq!(blocks.len(), 4);
        assert_eq!(blocks[0].text.plain_text(), "Line 1");
        assert_eq!(blocks[1].text.plain_text(), "Line 2");
        assert_eq!(blocks[2].text.plain_text(), "");
        assert_eq!(blocks[3].text.plain_text(), "Line 3");
    }

    #[test]
    fn test_preview_merges_consecutive_lines_per_commonmark() {
        let markdown = "Line 1\nLine 2\n\nLine 3";
        let blocks = parse_preview_document(markdown);
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].text.plain_text(), "Line 1\nLine 2");
        assert_eq!(blocks[1].text.plain_text(), "");
        assert_eq!(blocks[2].text.plain_text(), "Line 3");
    }
}
