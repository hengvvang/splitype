//! Markdown and source text serialization engine for the document block tree.

use std::collections::HashMap;

use gpui::*;

use super::Document;
use crate::model::block::Block;
use markdown_parser::block::CalloutKind;
use markdown_parser::block::image::parse_standalone_image;
use markdown_parser::block::table::serialize_table_markdown_lines;
use markdown_parser::parse::BlockKind;

/// Mapping from a block's entity id to its line range in the serialized Markdown document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockLineMapping {
    pub entity_id: EntityId,
    pub own_start_line: usize,
    pub own_end_line: usize,
    pub list_depth: usize,
}

impl Document {
    pub fn serialize_markdown(&self, cx: &App) -> String {
        let (lines, _) = self.serialize_markdown_lines_with_mapping(cx);
        lines.join("\n")
    }

    /// Serializes all document lines into a `Vec<String>` and records each
    /// block's own line range and list depth.
    pub fn serialize_markdown_lines_with_mapping(
        &self,
        cx: &App,
    ) -> (Vec<String>, Vec<BlockLineMapping>) {
        let mut lines = Vec::new();
        let mut mappings = Vec::new();
        Self::collect_root_markdown_lines_with_mapping(&self.roots, cx, &mut lines, &mut mappings);
        (lines, mappings)
    }

    pub fn serialize_source_text(&self, cx: &App) -> String {
        self.index
            .entries
            .iter()
            .map(|entries| entries.entity.read(cx).display_text().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn is_empty_root_paragraph(block: &Block) -> bool {
        block.kind() == BlockKind::Paragraph
            && block.data.text.plain_text().is_empty()
            && block.children.is_empty()
    }

    pub fn collect_root_markdown_lines_with_mapping(
        blocks: &[Entity<Block>],
        cx: &App,
        lines: &mut Vec<String>,
        mappings: &mut Vec<BlockLineMapping>,
    ) {
        for block in blocks {
            let block_ref = block.read(cx);
            Self::collect_single_block_markdown_lines_with_mapping(
                Some(block.entity_id()),
                block_ref,
                0,
                cx,
                lines,
                mappings,
            );
        }
    }

    pub fn collect_root_markdown_lines(
        blocks: &[Entity<Block>],
        cx: &App,
        lines: &mut Vec<String>,
    ) {
        let mut mappings = Vec::new();
        Self::collect_root_markdown_lines_with_mapping(blocks, cx, lines, &mut mappings);
    }

    /// Byte offset in the serialized Markdown at which each block's own
    /// lines begin. Matches [`Self::serialize_markdown`] byte-for-byte.
    pub fn markdown_offsets_of_blocks(&self, cx: &App) -> HashMap<EntityId, usize> {
        let (lines, mappings) = self.serialize_markdown_lines_with_mapping(cx);
        let mut line_offsets = Vec::with_capacity(lines.len() + 1);
        let mut curr = 0usize;
        for line in &lines {
            line_offsets.push(curr);
            curr += line.len() + 1;
        }
        line_offsets.push(curr);

        let mut offsets = HashMap::new();
        for m in mappings {
            let offset = line_offsets.get(m.own_start_line).copied().unwrap_or(curr);
            offsets.insert(m.entity_id, offset);
        }
        offsets
    }

    /// Byte offset at which `target` block's own lines begin, or `None`
    /// when the block is not part of the document.
    pub fn markdown_offset_of_block(&self, target: EntityId, cx: &App) -> Option<usize> {
        let (lines, mappings) = self.serialize_markdown_lines_with_mapping(cx);
        let mapping = mappings.into_iter().find(|m| m.entity_id == target)?;
        let mut offset = 0usize;
        for line in &lines[..mapping.own_start_line.min(lines.len())] {
            offset += line.len() + 1;
        }
        Some(offset)
    }

    /// Serializes only the lines this block itself contributes, without its
    /// children. Line contents (including list indentation and quote
    /// prefixes) match [`Self::serialize_markdown`] byte-for-byte.
    pub fn collect_block_own_markdown_lines(
        block_ref: &Block,
        list_depth: usize,
        lines: &mut Vec<String>,
    ) {
        match block_ref.kind() {
            BlockKind::Table => {
                if let Some(table) = block_ref.data.table.as_ref() {
                    lines.extend(serialize_table_markdown_lines(table));
                }
            }
            BlockKind::CodeBlock { language } => {
                let indentation = "  ".repeat(list_depth);
                let lang_str = language.as_ref().map(|s| s.as_ref()).unwrap_or("");
                let fence = markdown_parser::parse::safe_code_fence_with_info(
                    &block_ref.data.text.plain_text(),
                    language.as_ref().map(|language| language.as_ref()),
                );
                lines.push(format!("{indentation}{fence}{lang_str}"));
                let content = block_ref.data.text.plain_text();
                for code_line in content.split('\n') {
                    lines.push(format!("{indentation}{code_line}"));
                }
                lines.push(format!("{indentation}{fence}"));
            }
            BlockKind::Blockquote => {
                let text_markdown =
                    CalloutKind::escape_plain_quote_header(&block_ref.data.text_markdown());
                let indentation = "  ".repeat(list_depth);
                if !text_markdown.is_empty() || block_ref.children.is_empty() {
                    for line in text_markdown.split('\n') {
                        lines.push(format!("{indentation}> {line}"));
                    }
                }
            }
            BlockKind::Callout(variant) => {
                let indentation = "  ".repeat(list_depth);
                lines.push(format!(
                    "{indentation}> {}",
                    variant.header_markdown(&block_ref.data.text_markdown())
                ));
            }
            BlockKind::FootnoteDefinition => {
                let indentation = "  ".repeat(list_depth);
                let full_text = block_ref.data.text_markdown();
                let (id, first_line) =
                    markdown_parser::block::footnote::split_footnote_definition_text(&full_text);
                if first_line.is_empty() && block_ref.children.is_empty() {
                    lines.push(format!("{indentation}[^{id}]:"));
                    return;
                }

                let mut first_lines = first_line.split('\n');
                let first = first_lines.next().unwrap_or_default();
                if first.is_empty() {
                    lines.push(format!("{indentation}[^{id}]:"));
                } else {
                    lines.push(format!("{indentation}[^{id}]: {first}"));
                }
                for line in first_lines {
                    if line.is_empty() {
                        lines.push(String::new());
                    } else {
                        lines.push(format!("{indentation}    {line}"));
                    }
                }
            }
            BlockKind::RawMarkdown | BlockKind::HtmlComment | BlockKind::HtmlBlock => {
                let indentation = "  ".repeat(list_depth);
                let raw_markdown = block_ref
                    .data
                    .raw_source
                    .clone()
                    .unwrap_or_else(|| block_ref.data.text_markdown());
                for line in raw_markdown.split('\n') {
                    if indentation.is_empty() {
                        lines.push(line.to_string());
                    } else {
                        lines.push(format!("{indentation}{line}"));
                    }
                }
            }
            // Every remaining block kind — including list items —
            // serializes through its single-line Markdown form.
            _ => {
                lines.push(
                    block_ref
                        .data
                        .serialize_markdown_line(list_depth, block_ref.list_ordinal),
                );
            }
        }
    }

    pub fn collect_single_block_markdown_lines(
        block_ref: &Block,
        list_depth: usize,
        cx: &App,
        lines: &mut Vec<String>,
    ) {
        let mut mappings = Vec::new();
        Self::collect_single_block_markdown_lines_with_mapping(
            None,
            block_ref,
            list_depth,
            cx,
            lines,
            &mut mappings,
        );
    }

    pub fn collect_single_block_markdown_lines_with_mapping(
        entity_id: Option<EntityId>,
        block_ref: &Block,
        list_depth: usize,
        cx: &App,
        lines: &mut Vec<String>,
        mappings: &mut Vec<BlockLineMapping>,
    ) {
        let start_line = lines.len();
        Self::collect_block_own_markdown_lines(block_ref, list_depth, lines);
        let own_end_line = lines.len();
        if let Some(id) = entity_id {
            mappings.push(BlockLineMapping {
                entity_id: id,
                own_start_line: start_line,
                own_end_line,
                list_depth,
            });
        }

        match block_ref.kind() {
            BlockKind::Table
            | BlockKind::CodeBlock { .. }
            | BlockKind::RawMarkdown
            | BlockKind::HtmlComment
            | BlockKind::HtmlBlock => {}
            BlockKind::Blockquote | BlockKind::Callout(_) => {
                if !block_ref.children.is_empty() {
                    let mut child_lines = Vec::new();
                    let mut child_mappings = Vec::new();
                    Self::collect_markdown_lines_with_mapping(
                        &block_ref.children,
                        list_depth,
                        cx,
                        &mut child_lines,
                        &mut child_mappings,
                        false,
                    );
                    let base_line = lines.len();
                    let indentation = "  ".repeat(list_depth);
                    lines.extend(
                        child_lines
                            .into_iter()
                            .map(|line| format!("{indentation}> {line}")),
                    );
                    for mut cm in child_mappings {
                        cm.own_start_line += base_line;
                        cm.own_end_line += base_line;
                        mappings.push(cm);
                    }
                }
            }
            BlockKind::FootnoteDefinition => {
                if !block_ref.children.is_empty() {
                    Self::collect_markdown_lines_with_mapping(
                        &block_ref.children,
                        2,
                        cx,
                        lines,
                        mappings,
                        false,
                    );
                }
            }
            BlockKind::BulletListItem
            | BlockKind::TaskListItem { .. }
            | BlockKind::NumberedListItem => {
                let child_list_depth = list_depth + 1;
                for child in &block_ref.children {
                    let child_ref = child.read(cx);
                    if Self::list_child_requires_leading_blank_line(child_ref) {
                        lines.push(String::new());
                    }
                    Self::collect_single_block_markdown_lines_with_mapping(
                        Some(child.entity_id()),
                        child_ref,
                        child_list_depth,
                        cx,
                        lines,
                        mappings,
                    );
                }
            }
            _ => {
                let child_list_depth = list_depth + usize::from(block_ref.kind().is_list_item());
                Self::collect_markdown_lines_with_mapping(
                    &block_ref.children,
                    child_list_depth,
                    cx,
                    lines,
                    mappings,
                    false,
                );
            }
        }
    }

    pub fn list_child_requires_leading_blank_line(block_ref: &Block) -> bool {
        if block_ref.kind() != BlockKind::Paragraph || !block_ref.children.is_empty() {
            return false;
        }

        let markdown = block_ref.data.text_markdown();
        !markdown.is_empty() && parse_standalone_image(&markdown).is_none()
    }

    pub fn collect_markdown_lines_with_mapping(
        blocks: &[Entity<Block>],
        depth: usize,
        cx: &App,
        lines: &mut Vec<String>,
        mappings: &mut Vec<BlockLineMapping>,
        blank_line_between_siblings: bool,
    ) {
        let mut first = true;
        let mut previous_was_list_item = false;
        for block in blocks {
            let current_is_list_item = block.read(cx).kind().is_list_item();
            if !first
                && blank_line_between_siblings
                && !(previous_was_list_item && current_is_list_item)
            {
                lines.push(String::new());
            }
            first = false;

            let block_ref = block.read(cx);
            Self::collect_single_block_markdown_lines_with_mapping(
                Some(block.entity_id()),
                block_ref,
                depth,
                cx,
                lines,
                mappings,
            );
            previous_was_list_item = current_is_list_item;
        }
    }

    pub fn collect_markdown_lines(
        blocks: &[Entity<Block>],
        depth: usize,
        cx: &App,
        lines: &mut Vec<String>,
        blank_line_between_siblings: bool,
    ) {
        let mut mappings = Vec::new();
        Self::collect_markdown_lines_with_mapping(
            blocks,
            depth,
            cx,
            lines,
            &mut mappings,
            blank_line_between_siblings,
        );
    }
}
