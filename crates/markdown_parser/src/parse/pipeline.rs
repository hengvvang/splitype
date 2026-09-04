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

/// Line and flat-list span of one top-level parse region.
///
/// One region is one top-level construct (fence, list, quote, table,
/// paragraph, ...) — a maximal contiguous run of source lines consumed by a
/// single dispatch step of the pipeline. Its blocks are contiguous in the
/// flattened block list, and the regions partition the document in line
/// order, which is what lets the incremental re-parse splice one region
/// range without touching the rest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RegionSpan {
    /// First source line of the region, inclusive.
    pub(crate) line_start: usize,
    /// One past the region's last source line.
    pub(crate) line_end: usize,
    /// Index of the region's first block in the flattened block list.
    pub(crate) flat_start: usize,
    /// One past the region's last block in the flattened block list.
    pub(crate) flat_end: usize,
}

/// Parsing mode for Markdown documents.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ParseMode {
    /// Linewise mode: 1 source line maps 1:1 to 1 block, empty lines are
    /// independent empty blocks (editing-first).
    #[default]
    Linewise,
    /// CommonMark mode: merges consecutive lines into single <p> paragraphs
    /// adhering 100% to CommonMark.
    CommonMark,
}

pub fn parse_preview_document(markdown: &str) -> Vec<BlockData> {
    parse_document_with_mode(markdown, ParseMode::CommonMark)
}

pub(crate) fn parse_document_with_mode(markdown: &str, mode: ParseMode) -> Vec<BlockData> {
    if markdown.is_empty() {
        return Vec::new();
    }
    let lines = markdown.lines().collect::<Vec<_>>();
    build_blocks_from_lines_internal(&lines, mode, true)
}

/// Internal dispatch: walk every line and emit native blocks or raw
/// fallbacks, recording the span of every top-level region alongside.
pub(crate) fn build_blocks_from_lines_with_regions<S: AsRef<str>>(
    lines: &[S],
    mode: ParseMode,
    allow_root_footnote_definitions: bool,
) -> (Vec<BlockData>, Vec<RegionSpan>) {
    let mut roots = Vec::new();
    let mut regions = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index].as_ref();
        if line.trim().is_empty() {
            if mode == ParseMode::Linewise {
                push_region(
                    &mut roots,
                    &mut regions,
                    vec![native_block(BlockKind::Paragraph, "")],
                    index,
                    index + 1,
                );
            }
            index += 1;
            continue;
        }

        if parse_opening_fence(line).is_some() {
            if let Some((block, next_index)) = collect_fenced_code_block(lines, index) {
                push_region(&mut roots, &mut regions, vec![block], index, next_index);
                index = next_index;
                continue;
            }
        }

        if let Some((block, end)) = collect_comment_block(lines, index) {
            push_region(&mut roots, &mut regions, vec![block], index, end);
            index = end;
            continue;
        }

        if is_block_html_start(line) {
            let end = collect_block_html_region(lines, index);
            push_region(
                &mut roots,
                &mut regions,
                vec![html_or_raw_block(
                    lines[index..end]
                        .iter()
                        .map(|line| line.as_ref())
                        .collect::<Vec<_>>()
                        .join("\n"),
                )],
                index,
                end,
            );
            index = end;
            continue;
        }

        if is_footnote_definition_start(line) {
            let end = collect_footnote_definition_region(lines, index);
            let blocks = if allow_root_footnote_definitions {
                if let Some(blocks) =
                    build_native_footnote_definition_block(&lines[index..end], mode)
                {
                    blocks
                } else {
                    vec![raw_block(
                        lines[index..end]
                            .iter()
                            .map(|line| line.as_ref())
                            .collect::<Vec<_>>()
                            .join("\n"),
                    )]
                }
            } else {
                vec![raw_block(
                    lines[index..end]
                        .iter()
                        .map(|line| line.as_ref())
                        .collect::<Vec<_>>()
                        .join("\n"),
                )]
            };
            push_region(&mut roots, &mut regions, blocks, index, end);
            index = end;
            continue;
        }

        if is_reference_definition_start(line) {
            let end = collect_reference_definition_region(lines, index);
            push_region(
                &mut roots,
                &mut regions,
                vec![raw_block(
                    lines[index..end]
                        .iter()
                        .map(|line| line.as_ref())
                        .collect::<Vec<_>>()
                        .join("\n"),
                )],
                index,
                end,
            );
            index = end;
            continue;
        }

        if parse_standalone_image(line).is_some() {
            push_region(
                &mut roots,
                &mut regions,
                vec![standalone_image_block(line.to_string())],
                index,
                index + 1,
            );
            index += 1;
            continue;
        }

        if strip_indented_code_prefix(line).is_some() {
            let Some((block, next_index)) = collect_indented_code_block(lines, index) else {
                unreachable!("indented code prefix disappeared after detection");
            };
            push_region(&mut roots, &mut regions, vec![block], index, next_index);
            index = next_index;
            continue;
        }

        if parse_list_marker(line).is_some() {
            let (blocks, next_index) = collect_list_blocks(lines, index);
            push_region(&mut roots, &mut regions, blocks, index, next_index);
            index = next_index;
            continue;
        }

        if is_quote_start(line) {
            let (blocks, next_index) = collect_quote_block(lines, index);
            push_region(&mut roots, &mut regions, blocks, index, next_index);
            index = next_index;
            continue;
        }

        if let Some((level, content)) = BlockKind::parse_atx_heading_line(line) {
            push_region(
                &mut roots,
                &mut regions,
                vec![native_block(BlockKind::Heading { level }, &content)],
                index,
                index + 1,
            );
            index += 1;
            continue;
        }

        if BlockKind::parse_thematic_break_line(line) {
            push_region(
                &mut roots,
                &mut regions,
                vec![BlockData::with_plain_text(
                    BlockKind::ThematicBreak,
                    line.to_string(),
                )],
                index,
                index + 1,
            );
            index += 1;
            continue;
        }

        if is_table_candidate_line(line) {
            let end = collect_table_candidate_region(lines, index);
            let region = &lines[index..end];
            if let Some(table) = parse_table_region(region) {
                push_region(
                    &mut roots,
                    &mut regions,
                    vec![BlockData::table(table)],
                    index,
                    end,
                );
            } else {
                push_region(
                    &mut roots,
                    &mut regions,
                    region
                        .iter()
                        .map(|line| plain_text_paragraph_block(line.as_ref().to_string()))
                        .collect(),
                    index,
                    end,
                );
            }
            index = end;
            continue;
        }

        if let Some(end) = collect_pipeless_table_region(lines, index)
            && let Some(table) = parse_table_region(&lines[index..end])
        {
            push_region(
                &mut roots,
                &mut regions,
                vec![BlockData::table(table)],
                index,
                end,
            );
            index = end;
            continue;
        }

        if is_display_math_start(line) {
            let end = collect_display_math_region(lines, index);
            push_region(
                &mut roots,
                &mut regions,
                vec![math_or_raw_block(
                    lines[index..end]
                        .iter()
                        .map(|line| line.as_ref())
                        .collect::<Vec<_>>()
                        .join("\n"),
                )],
                index,
                end,
            );
            index = end;
            continue;
        }

        if let Some(next) = lines.get(index + 1).map(|next| next.as_ref())
            && parse_list_marker(next).is_none()
            && let Some(level) = BlockKind::parse_setext_underline(next)
        {
            push_region(
                &mut roots,
                &mut regions,
                vec![native_block(BlockKind::Heading { level }, line.trim_end())],
                index,
                index + 2,
            );
            index += 2;
            continue;
        }

        match mode {
            ParseMode::Linewise => {
                push_region(
                    &mut roots,
                    &mut regions,
                    vec![native_block(BlockKind::Paragraph, line)],
                    index,
                    index + 1,
                );
                index += 1;
            }
            ParseMode::CommonMark => {
                let paragraph = collect_paragraph_block(lines, index);
                push_region(
                    &mut roots,
                    &mut regions,
                    vec![paragraph.0],
                    index,
                    paragraph.1,
                );
                index = paragraph.1;
            }
        }
    }

    (roots, regions)
}

/// Block-only variant for callers that don't need region spans.
pub(crate) fn build_blocks_from_lines_internal<S: AsRef<str>>(
    lines: &[S],
    mode: ParseMode,
    allow_root_footnote_definitions: bool,
) -> Vec<BlockData> {
    build_blocks_from_lines_with_regions(lines, mode, allow_root_footnote_definitions).0
}

/// Appends one top-level region's blocks and records its span.
fn push_region(
    roots: &mut Vec<BlockData>,
    regions: &mut Vec<RegionSpan>,
    blocks: Vec<BlockData>,
    line_start: usize,
    line_end: usize,
) {
    let flat_start = roots.len();
    roots.extend(blocks);
    regions.push(RegionSpan {
        line_start,
        line_end,
        flat_start,
        flat_end: roots.len(),
    });
}
