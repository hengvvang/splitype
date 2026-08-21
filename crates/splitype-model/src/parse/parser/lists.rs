//! Ordered, unordered, and task list item parser with nested child blocks.

use super::code_and_text::{
    collect_comment_block, collect_fenced_code_block, collect_indented_code_block,
    collect_paragraph_block,
};
use super::helpers::*;
use super::quotes::collect_quote_block;
use crate::block::image::parse_standalone_image;
use crate::block::table::{
    collect_table_candidate_region, is_table_candidate_line, parse_table_region,
};
use crate::parse::data::BlockData;
use crate::parse::indent::{
    dedent_lines, is_quote_start, leading_indent_columns_and_bytes, strip_indented_code_prefix,
};
use crate::parse::kind::BlockKind;

pub(crate) fn collect_list_blocks(lines: &[String], start: usize) -> (Vec<BlockData>, usize) {
    let mut roots = Vec::new();
    let mut index = start;

    while index < lines.len() {
        let Some(marker) = parse_list_marker(&lines[index]) else {
            break;
        };

        let item_end = collect_list_item_region(lines, index, marker.indent_columns);
        let mut block = native_block(marker.kind.clone(), &marker.text);
        let mut body_index = index + 1;
        let mut pending_blank_lines = 0usize;
        let mut fallback_raw = false;
        let mut saw_child = false;
        let mut item_children: Vec<BlockData> = Vec::new();

        while body_index < item_end {
            let line = &lines[body_index];
            if line.trim().is_empty() {
                pending_blank_lines += 1;
                body_index += 1;
                continue;
            }

            let (line_indent_columns, _) = leading_indent_columns_and_bytes(line);
            if line_indent_columns > marker.indent_columns {
                let anchor_dedented =
                    dedent_lines(&lines[body_index..item_end], line_indent_columns);

                if parse_list_marker(&anchor_dedented[0]).is_some() {
                    let (mut children, consumed) = collect_list_blocks(&anchor_dedented, 0);
                    attach_child_blocks(&mut block, &mut children);
                    item_children.append(&mut children);
                    body_index += consumed;
                    pending_blank_lines = 0;
                    saw_child = true;
                    continue;
                }

                if is_quote_start(&anchor_dedented[0]) {
                    let (mut quote_blocks, consumed) = collect_quote_block(&anchor_dedented, 0);
                    if !quote_blocks.is_empty() && quote_blocks[0].kind == BlockKind::RawMarkdown {
                        fallback_raw = true;
                        break;
                    }

                    attach_child_blocks(&mut block, &mut quote_blocks);
                    item_children.append(&mut quote_blocks);
                    body_index += consumed;
                    pending_blank_lines = 0;
                    saw_child = true;
                    continue;
                }

                if parse_opening_fence(&anchor_dedented[0]).is_some()
                    && let Some((mut code_block, consumed)) =
                        collect_fenced_code_block(&anchor_dedented, 0)
                {
                    attach_child_block(&mut block, &mut code_block);
                    item_children.push(code_block);
                    body_index += consumed;
                    pending_blank_lines = 0;
                    saw_child = true;
                    continue;
                }

                if is_table_candidate_line(&anchor_dedented[0]) {
                    let table_end = collect_table_candidate_region(&anchor_dedented, 0);
                    let table_region = &anchor_dedented[..table_end];
                    let mut child = if let Some(table) = parse_table_region(table_region) {
                        BlockData::table(table)
                    } else {
                        raw_block(table_region.join("\n"))
                    };
                    attach_child_block(&mut block, &mut child);
                    item_children.push(child);
                    body_index += table_end;
                    pending_blank_lines = 0;
                    saw_child = true;
                    continue;
                }

                if starts_with_standalone_image_child_paragraph(&anchor_dedented) {
                    let mut child = standalone_image_block(anchor_dedented[0].clone());
                    attach_child_block(&mut block, &mut child);
                    item_children.push(child);
                    body_index += 1;
                    pending_blank_lines = 0;
                    saw_child = true;
                    continue;
                }

                if line_indent_columns >= marker.content_indent_columns {
                    let content_dedented =
                        dedent_lines(&lines[body_index..item_end], marker.content_indent_columns);
                    if strip_indented_code_prefix(&content_dedented[0]).is_some() {
                        let Some((mut code_block, consumed)) =
                            collect_indented_code_block(&content_dedented, 0)
                        else {
                            unreachable!("indented code prefix disappeared after child detection");
                        };

                        attach_child_block(&mut block, &mut code_block);
                        item_children.push(code_block);
                        body_index += consumed;
                        pending_blank_lines = 0;
                        saw_child = true;
                        continue;
                    }
                }

                if is_reference_definition_start(&anchor_dedented[0]) {
                    let consumed = collect_reference_definition_region(&anchor_dedented, 0);
                    let mut child = raw_block(anchor_dedented[..consumed].join("\n"));
                    attach_child_block(&mut block, &mut child);
                    item_children.push(child);
                    body_index += consumed;
                    pending_blank_lines = 0;
                    saw_child = true;
                    continue;
                }

                if let Some((mut comment, consumed)) = collect_comment_block(&anchor_dedented, 0) {
                    attach_child_block(&mut block, &mut comment);
                    item_children.push(comment);
                    body_index += consumed;
                    pending_blank_lines = 0;
                    saw_child = true;
                    continue;
                }

                if is_block_html_start(&anchor_dedented[0]) {
                    let consumed = collect_block_html_region(&anchor_dedented, 0);
                    let mut child = html_or_raw_block(anchor_dedented[..consumed].join("\n"));
                    attach_child_block(&mut block, &mut child);
                    item_children.push(child);
                    body_index += consumed;
                    pending_blank_lines = 0;
                    saw_child = true;
                    continue;
                }

                if is_footnote_definition_start(&anchor_dedented[0]) {
                    let consumed = collect_footnote_definition_region(&anchor_dedented, 0);
                    let mut child = raw_block(anchor_dedented[..consumed].join("\n"));
                    attach_child_block(&mut block, &mut child);
                    item_children.push(child);
                    body_index += consumed;
                    pending_blank_lines = 0;
                    saw_child = true;
                    continue;
                }

                if is_display_math_start(&anchor_dedented[0]) {
                    let consumed = collect_display_math_region(&anchor_dedented, 0);
                    let mut child = math_or_raw_block(anchor_dedented[..consumed].join("\n"));
                    attach_child_block(&mut block, &mut child);
                    item_children.push(child);
                    body_index += consumed;
                    pending_blank_lines = 0;
                    saw_child = true;
                    continue;
                }

                let should_promote_plain_child = pending_blank_lines > 0
                    || saw_child
                    || block.text.plain_text().is_empty()
                    || parse_standalone_image(&block.text.serialize_markdown()).is_some();
                if should_promote_plain_child {
                    let (mut paragraph, consumed) = collect_paragraph_block(&anchor_dedented, 0);
                    attach_child_block(&mut block, &mut paragraph);
                    item_children.push(paragraph);
                    body_index += consumed;
                    pending_blank_lines = 0;
                    saw_child = true;
                    continue;
                }
            }

            if line_indent_columns >= marker.content_indent_columns {
                let content_dedented =
                    dedent_lines(&lines[body_index..item_end], marker.content_indent_columns);
                if strip_indented_code_prefix(&content_dedented[0]).is_some() {
                    let Some((mut code_block, consumed)) =
                        collect_indented_code_block(&content_dedented, 0)
                    else {
                        unreachable!("indented code prefix disappeared after detection");
                    };

                    attach_child_block(&mut block, &mut code_block);
                    item_children.push(code_block);
                    body_index += consumed;
                    pending_blank_lines = 0;
                    saw_child = true;
                    continue;
                }
            }

            let trimmed = line.trim_start_matches([' ', '\t']);
            append_markdown_to_block(
                &mut block,
                if pending_blank_lines > 0 {
                    "\n\n"
                } else {
                    "\n"
                },
                trimmed,
            );
            pending_blank_lines = 0;
            body_index += 1;
        }

        if fallback_raw {
            roots.push(raw_block(lines[index..item_end].join("\n")));
        } else {
            roots.push(block);
            roots.append(&mut item_children);
        }
        index = item_end;
    }

    (roots, index)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------
