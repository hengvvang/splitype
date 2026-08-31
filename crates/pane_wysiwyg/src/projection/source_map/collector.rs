//! Recursive block-tree traversal and source-target mapping collector.

use std::collections::HashMap;
use std::ops::Range;

use gpui::*;

use crate::model::Document;
use crate::model::block::Block;
use crate::markdown::parse::BlockKind;
use crate::projection::source_map::block_kinds::{
    push_code_block_mapping, push_fenced_block_mapping, push_footnote_definition_full_mapping,
    push_inline_block_mapping, push_raw_block_mapping, wrap_source_mapping_with_quotes,
};
use crate::projection::source_map::table_cells::push_table_mappings;
use crate::state::SourceTargetMapping;

/// Collects source-target mappings for a single block and all its children recursively.
pub fn collect_single_block_source_mappings(
    block: &Entity<Block>,
    list_depth: usize,
    quote_depth: usize,
    absolute_start: usize,
    mappings: &mut Vec<SourceTargetMapping>,
    block_ranges: &mut HashMap<EntityId, Range<usize>>,
    cx: &App,
) -> usize {
    let (kind, list_ordinal, text, children) = {
        let block_ref = block.read(cx);
        let kind = block_ref.kind();
        let text = (!matches!(
            kind,
            BlockKind::Table
                | BlockKind::CodeBlock { .. }
                | BlockKind::HtmlComment
                | BlockKind::HtmlBlock
                | BlockKind::MathBlock
                | BlockKind::MermaidBlock
                | BlockKind::RawMarkdown
                | BlockKind::ThematicBreak
        ))
        .then(|| block_ref.data.text.source_offset_map());
        (
            kind,
            block_ref.list_ordinal,
            text,
            block_ref.children.clone(),
        )
    };

    let own_len = match kind {
        BlockKind::Table => {
            push_table_mappings(block, list_depth, quote_depth, absolute_start, mappings, cx)
        }
        BlockKind::CodeBlock { .. } => {
            push_code_block_mapping(block, quote_depth, absolute_start, mappings, cx)
        }
        BlockKind::RawMarkdown | BlockKind::HtmlComment | BlockKind::HtmlBlock => {
            push_raw_block_mapping(block, quote_depth, absolute_start, mappings, cx)
        }
        BlockKind::MathBlock | BlockKind::MermaidBlock => {
            push_fenced_block_mapping(block, quote_depth, absolute_start, mappings, cx)
        }
        BlockKind::ThematicBreak => {
            let line = block
                .read(cx)
                .data
                .serialize_markdown_line(list_depth, list_ordinal);
            if quote_depth == 0 {
                line.len()
            } else {
                wrap_source_mapping_with_quotes(
                    line.clone(),
                    (0..=line.len()).collect(),
                    (0..=line.len()).collect(),
                    quote_depth,
                )
                .0
                .len()
            }
        }
        BlockKind::Heading { level } => push_inline_block_mapping(
            block,
            text.expect("heading text").source().to_string(),
            format!("{}{} ", "  ".repeat(list_depth), "#".repeat(level as usize)),
            String::new(),
            quote_depth,
            absolute_start,
            mappings,
        ),
        BlockKind::Paragraph => {
            let indentation = "  ".repeat(list_depth);
            push_inline_block_mapping(
                block,
                text.expect("paragraph text").source().to_string(),
                indentation.clone(),
                indentation,
                quote_depth,
                absolute_start,
                mappings,
            )
        }
        BlockKind::BulletListItem => {
            let indentation = "  ".repeat(list_depth);
            push_inline_block_mapping(
                block,
                text.expect("bullet text").source().to_string(),
                format!("{indentation}- "),
                format!("{indentation}  "),
                quote_depth,
                absolute_start,
                mappings,
            )
        }
        BlockKind::TaskListItem { checked } => {
            let indentation = "  ".repeat(list_depth);
            push_inline_block_mapping(
                block,
                text.expect("task text").source().to_string(),
                format!("{indentation}- [{}] ", if checked { "x" } else { " " }),
                format!("{indentation}      "),
                quote_depth,
                absolute_start,
                mappings,
            )
        }
        BlockKind::NumberedListItem => {
            let indentation = "  ".repeat(list_depth);
            let ordinal = list_ordinal.unwrap_or(1);
            push_inline_block_mapping(
                block,
                text.expect("numbered text").source().to_string(),
                format!("{indentation}{ordinal}. "),
                format!("{indentation}   "),
                quote_depth,
                absolute_start,
                mappings,
            )
        }
        BlockKind::Blockquote => {
            let text = text.expect("quote text").source().to_string();
            if text.is_empty() && !children.is_empty() {
                0
            } else {
                push_inline_block_mapping(
                    block,
                    text,
                    String::new(),
                    String::new(),
                    quote_depth + 1,
                    absolute_start,
                    mappings,
                )
            }
        }
        BlockKind::Callout(variant) => {
            let text_markdown = text.expect("callout text").source().to_string();
            if text_markdown.is_empty() {
                let full_text = wrap_source_mapping_with_quotes(
                    format!("[!{}]", variant.marker()),
                    vec![0],
                    vec![0; format!("[!{}]", variant.marker()).len() + 1],
                    quote_depth + 1,
                )
                .0;
                mappings.push(SourceTargetMapping {
                    entity: block.clone(),
                    full_source_range: absolute_start..absolute_start + full_text.len(),
                    content_to_source: vec![full_text.len()],
                    source_to_content: vec![0; full_text.len() + 1],
                });
                full_text.len()
            } else {
                push_inline_block_mapping(
                    block,
                    text_markdown,
                    format!("[!{}] ", variant.marker()),
                    String::new(),
                    quote_depth + 1,
                    absolute_start,
                    mappings,
                )
            }
        }
        BlockKind::FootnoteDefinition => {
            let footnote_source = text.expect("footnote text").source().to_string();
            let (footnote_id, first_line) =
                crate::markdown::block::footnote::split_footnote_definition_text(&footnote_source);
            push_footnote_definition_full_mapping(
                block,
                footnote_id,
                first_line,
                quote_depth,
                absolute_start,
                mappings,
            )
        }
    };

    if kind == BlockKind::FootnoteDefinition {
        let mut total_len = own_len;
        for child in &children {
            if total_len > 0 {
                total_len += 1;
            }
            total_len += collect_single_block_source_mappings(
                child,
                2,
                quote_depth,
                absolute_start + total_len,
                mappings,
                block_ranges,
                cx,
            );
        }
        block_ranges.insert(
            block.entity_id(),
            absolute_start..absolute_start + total_len,
        );
        return total_len;
    }

    let child_list_depth = list_depth + usize::from(kind.is_list_item());
    let child_quote_depth = quote_depth + usize::from(kind.is_quote_container());
    let mut total_len = own_len;
    for child in &children {
        let child_ref = child.read(cx);
        if kind.is_list_item()
            && Document::list_child_requires_leading_blank_line(child_ref)
        {
            total_len += 1;
        }
        if total_len > 0 {
            total_len += 1;
        }
        total_len += collect_single_block_source_mappings(
            child,
            child_list_depth,
            child_quote_depth,
            absolute_start + total_len,
            mappings,
            block_ranges,
            cx,
        );
    }

    block_ranges.insert(
        block.entity_id(),
        absolute_start..absolute_start + total_len,
    );
    total_len
}

