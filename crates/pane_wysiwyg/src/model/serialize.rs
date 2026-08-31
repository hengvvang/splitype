//! Markdown and source text serialization engine for the document block tree.

use gpui::*;

use super::Document;
use crate::markdown::block::CalloutKind;
use crate::markdown::block::image::parse_standalone_image;
use crate::markdown::block::table::serialize_table_markdown_lines;
use crate::markdown::parse::BlockKind;
pub use crate::markdown::parse::fence::safe_code_fence_with_info;
use crate::model::block::Block;

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

    pub fn collect_single_block_markdown_lines(
        block_ref: &Block,
        list_depth: usize,
        cx: &App,
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
                let fence = crate::model::serialize::safe_code_fence_with_info(
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

                if !block_ref.children.is_empty() {
                    let mut child_lines = Vec::new();
                    Self::collect_markdown_lines(
                        &block_ref.children,
                        list_depth,
                        cx,
                        &mut child_lines,
                        false,
                    );
                    lines.extend(
                        child_lines
                            .into_iter()
                            .map(|line| format!("{indentation}> {line}")),
                    );
                }
            }
            BlockKind::Callout(variant) => {
                let indentation = "  ".repeat(list_depth);
                lines.push(format!(
                    "{indentation}> {}",
                    variant.header_markdown(&block_ref.data.text_markdown())
                ));
                if !block_ref.children.is_empty() {
                    let mut child_lines = Vec::new();
                    Self::collect_markdown_lines(
                        &block_ref.children,
                        list_depth,
                        cx,
                        &mut child_lines,
                        false,
                    );
                    lines.extend(
                        child_lines
                            .into_iter()
                            .map(|line| format!("{indentation}> {line}")),
                    );
                }
            }
            BlockKind::FootnoteDefinition => {
                let indentation = "  ".repeat(list_depth);
                let full_text = block_ref.data.text_markdown();
                let (id, first_line) =
                    crate::markdown::block::footnote::split_footnote_definition_text(&full_text);
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

                if !block_ref.children.is_empty() {
                    Self::collect_markdown_lines(&block_ref.children, 2, cx, lines, false);
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
            BlockKind::BulletListItem
            | BlockKind::TaskListItem { .. }
            | BlockKind::NumberedListItem => {
                lines.push(
                    block_ref
                        .data
                        .serialize_markdown_line(list_depth, block_ref.list_ordinal),
                );
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
                lines.push(
                    block_ref
                        .data
                        .serialize_markdown_line(list_depth, block_ref.list_ordinal),
                );
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
