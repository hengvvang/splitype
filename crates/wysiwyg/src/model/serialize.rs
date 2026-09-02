//! Markdown and source text serialization engine for the document block tree.

use std::collections::HashMap;

use gpui::*;

use super::Document;
use crate::model::block::Block;
use markdown_parser::block::CalloutKind;
use markdown_parser::block::image::parse_standalone_image;
use markdown_parser::block::table::serialize_table_markdown_lines;
use markdown_parser::parse::BlockKind;

impl Document {
    pub fn serialize_markdown(&self, cx: &App) -> String {
        let mut lines = Vec::new();
        Self::collect_root_markdown_lines(&self.roots, cx, &mut lines);
        lines.join("\n")
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

    pub fn collect_root_markdown_lines(
        blocks: &[Entity<Block>],
        cx: &App,
        lines: &mut Vec<String>,
    ) {
        for block in blocks {
            let block_ref = block.read(cx);
            Self::collect_single_block_markdown_lines(block_ref, 0, cx, lines);
        }
    }

    /// Byte offset in the serialized Markdown at which each block's own
    /// lines begin. The walker mirrors
    /// [`Self::collect_single_block_markdown_lines`] exactly, so offsets
    /// agree with [`Self::serialize_markdown`] byte-for-byte. Used to map
    /// block-local carets to document-level cursor hints.
    pub fn markdown_offsets_of_blocks(&self, cx: &App) -> HashMap<EntityId, usize> {
        let mut offsets = HashMap::new();
        self.walk_block_offsets(cx, &mut |entity_id, offset| {
            offsets.insert(entity_id, offset);
        });
        offsets
    }

    /// Byte offset at which `target` block's own lines begin, or `None`
    /// when the block is not part of the document.
    pub fn markdown_offset_of_block(&self, target: EntityId, cx: &App) -> Option<usize> {
        let mut found = None;
        self.walk_block_offsets(cx, &mut |entity_id, offset| {
            if found.is_none() && entity_id == target {
                found = Some(offset);
            }
        });
        found
    }

    /// Visits every block in serialization order, reporting the byte offset
    /// at which its own lines begin. The traversal applies the same line
    /// sequence the serializer produces, so reported offsets agree with
    /// [`Self::serialize_markdown`] byte-for-byte.
    fn walk_block_offsets(&self, cx: &App, visit: &mut impl FnMut(EntityId, usize)) {
        let mut offset = 0usize;
        Self::visit_block_offsets(&self.roots, 0, 0, false, cx, &mut offset, visit);
    }

    /// Recursive half of [`Self::walk_block_offsets`]. `line_prefix_len`
    /// accumulates the byte length of enclosing blockquote/callout prefixes
    /// (`{indentation}> `) prepended to every line of this subtree.
    #[allow(clippy::too_many_arguments)]
    fn visit_block_offsets(
        blocks: &[Entity<Block>],
        depth: usize,
        line_prefix_len: usize,
        blank_line_between: bool,
        cx: &App,
        offset: &mut usize,
        visit: &mut impl FnMut(EntityId, usize),
    ) {
        let mut first = true;
        let mut previous_was_list_item = false;
        for block in blocks {
            let block_ref = block.read(cx);
            let current_is_list_item = block_ref.kind().is_list_item();
            if !first && blank_line_between && !(previous_was_list_item && current_is_list_item) {
                *offset += 1;
            }
            first = false;
            visit(block.entity_id(), *offset);

            let mut own_lines = Vec::new();
            Self::collect_block_own_markdown_lines(block_ref, depth, &mut own_lines);
            for line in own_lines {
                *offset += line.len() + 1 + line_prefix_len;
            }

            match block_ref.kind() {
                BlockKind::Table
                | BlockKind::CodeBlock { .. }
                | BlockKind::RawMarkdown
                | BlockKind::HtmlComment
                | BlockKind::HtmlBlock => {}
                BlockKind::Blockquote | BlockKind::Callout(_) => {
                    let child_prefix_len = line_prefix_len + depth * 2 + 2;
                    Self::visit_block_offsets(
                        &block_ref.children,
                        depth,
                        child_prefix_len,
                        false,
                        cx,
                        offset,
                        visit,
                    );
                }
                BlockKind::FootnoteDefinition => {
                    Self::visit_block_offsets(
                        &block_ref.children,
                        2,
                        line_prefix_len,
                        false,
                        cx,
                        offset,
                        visit,
                    );
                }
                BlockKind::BulletListItem
                | BlockKind::TaskListItem { .. }
                | BlockKind::NumberedListItem => {
                    for child in &block_ref.children {
                        if Self::list_child_requires_leading_blank_line(child.read(cx)) {
                            *offset += 1;
                        }
                        Self::visit_block_offsets(
                            std::slice::from_ref(child),
                            depth + 1,
                            line_prefix_len,
                            false,
                            cx,
                            offset,
                            visit,
                        );
                    }
                }
                _ => {
                    let child_depth = depth + usize::from(current_is_list_item);
                    Self::visit_block_offsets(
                        &block_ref.children,
                        child_depth,
                        line_prefix_len,
                        false,
                        cx,
                        offset,
                        visit,
                    );
                }
            }
            previous_was_list_item = current_is_list_item;
        }
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
        Self::collect_block_own_markdown_lines(block_ref, list_depth, lines);
        match block_ref.kind() {
            BlockKind::Table
            | BlockKind::CodeBlock { .. }
            | BlockKind::RawMarkdown
            | BlockKind::HtmlComment
            | BlockKind::HtmlBlock => {}
            BlockKind::Blockquote | BlockKind::Callout(_) => {
                if !block_ref.children.is_empty() {
                    let mut child_lines = Vec::new();
                    Self::collect_markdown_lines(
                        &block_ref.children,
                        list_depth,
                        cx,
                        &mut child_lines,
                        false,
                    );
                    let indentation = "  ".repeat(list_depth);
                    lines.extend(
                        child_lines
                            .into_iter()
                            .map(|line| format!("{indentation}> {line}")),
                    );
                }
            }
            BlockKind::FootnoteDefinition => {
                if !block_ref.children.is_empty() {
                    Self::collect_markdown_lines(&block_ref.children, 2, cx, lines, false);
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
                    Self::collect_single_block_markdown_lines(
                        child_ref,
                        child_list_depth,
                        cx,
                        lines,
                    );
                }
            }
            _ => {
                let child_list_depth = list_depth + usize::from(block_ref.kind().is_list_item());
                Self::collect_markdown_lines(
                    &block_ref.children,
                    child_list_depth,
                    cx,
                    lines,
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

    pub fn collect_markdown_lines(
        blocks: &[Entity<Block>],
        depth: usize,
        cx: &App,
        lines: &mut Vec<String>,
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
            Self::collect_single_block_markdown_lines(block_ref, depth, cx, lines);
            previous_was_list_item = current_is_list_item;
        }
    }
}
