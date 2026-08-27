//! Blockquote and Callout parser.

use super::code_and_text::{
    collect_comment_block, collect_fenced_code_block, collect_indented_code_block,
};
use super::footnotes::build_native_footnote_definition_block;
use super::helpers::*;
use super::lists::collect_list_blocks;
use crate::block::callout::CalloutKind;
use crate::block::table::{
    collect_table_candidate_region, is_table_candidate_line, parse_table_region,
};
use crate::inline::text::BlockText;
use crate::parse::data::BlockData;
use crate::parse::indent::{is_quote_start, strip_indented_code_prefix, strip_one_quote_level};
use crate::parse::kind::BlockKind;

pub(crate) fn collect_quote_block(lines: &[String], start: usize) -> (Vec<BlockData>, usize) {
    let end = collect_quote_raw_region(lines, start);
    let region = &lines[start..end];
    let mut dequoted = Vec::with_capacity(region.len());
    for line in region {
        if line.trim().is_empty() {
            dequoted.push(String::new());
            continue;
        }

        let Some(content) = strip_one_quote_level(line) else {
            let raw_block = raw_block(region.join("\n"));
            return (vec![raw_block], end);
        };
        dequoted.push(content);
    }

    let Some(result) = build_native_quote_block(&dequoted) else {
        let raw_block = raw_block(region.join("\n"));
        return (vec![raw_block], end);
    };

    (result, end)
}

pub(crate) fn build_native_quote_block(lines: &[String]) -> Option<Vec<BlockData>> {
    if let Some(header_index) = lines.iter().position(|line| !line.trim().is_empty())
        && let Some((variant, text)) = CalloutKind::parse_header_line(&lines[header_index])
    {
        return build_native_callout_block(&lines[header_index + 1..], variant, text);
    }

    let mut own_text = String::new();
    let mut child_blocks: Vec<BlockData> = Vec::new();
    let mut index = 0usize;
    let mut pending_blank_lines = 0usize;
    let mut saw_child = false;

    while index < lines.len() {
        let line = &lines[index];
        if line.trim().is_empty() {
            pending_blank_lines += 1;
            index += 1;
            continue;
        }

        if is_table_candidate_line(line) {
            if pending_blank_lines > 0 && (!own_text.is_empty() || !child_blocks.is_empty()) {
                append_separator_children(&mut child_blocks, pending_blank_lines);
            }
            let table_end = collect_table_candidate_region(lines, index);
            let table_region = &lines[index..table_end];
            if let Some(table) = parse_table_region(table_region) {
                child_blocks.push(BlockData::table(table));
            } else {
                child_blocks.push(raw_block(table_region.join("\n")));
            }
            saw_child = true;
            pending_blank_lines = 0;
            index = table_end;
            continue;
        }

        if is_footnote_definition_start(line) {
            if pending_blank_lines > 0 && (!own_text.is_empty() || !child_blocks.is_empty()) {
                append_separator_children(&mut child_blocks, pending_blank_lines);
            }
            let footnote_end = collect_footnote_definition_region(lines, index);
            if let Some(mut footnote_blocks) = build_native_footnote_definition_block(
                &lines[index..footnote_end],
                crate::parse::parser::ParseMode::Wysiwyg,
            ) {
                child_blocks.append(&mut footnote_blocks);
                saw_child = true;
                pending_blank_lines = 0;
                index = footnote_end;
                continue;
            }
        }

        if let Some((comment, consumed)) = collect_comment_block(lines, index) {
            if pending_blank_lines > 0 && (!own_text.is_empty() || !child_blocks.is_empty()) {
                append_separator_children(&mut child_blocks, pending_blank_lines);
            }
            child_blocks.push(comment);
            saw_child = true;
            pending_blank_lines = 0;
            index = consumed;
            continue;
        }

        if is_block_html_start(line) {
            if pending_blank_lines > 0 && (!own_text.is_empty() || !child_blocks.is_empty()) {
                append_separator_children(&mut child_blocks, pending_blank_lines);
            }
            let html_end = collect_block_html_region(lines, index);
            child_blocks.push(html_or_raw_block(lines[index..html_end].join("\n")));
            saw_child = true;
            pending_blank_lines = 0;
            index = html_end;
            continue;
        }

        if is_display_math_start(line) {
            if pending_blank_lines > 0 && (!own_text.is_empty() || !child_blocks.is_empty()) {
                append_separator_children(&mut child_blocks, pending_blank_lines);
            }
            let math_end = collect_display_math_region(lines, index);
            child_blocks.push(math_or_raw_block(lines[index..math_end].join("\n")));
            saw_child = true;
            pending_blank_lines = 0;
            index = math_end;
            continue;
        }

        if let Some(unsupported_end) = collect_unsupported_quote_region(lines, index) {
            if pending_blank_lines > 0 && (!own_text.is_empty() || !child_blocks.is_empty()) {
                append_separator_children(&mut child_blocks, pending_blank_lines);
            }
            child_blocks.push(raw_block(lines[index..unsupported_end].join("\n")));
            saw_child = true;
            pending_blank_lines = 0;
            index = unsupported_end;
            continue;
        }

        if is_quote_start(line) {
            if pending_blank_lines > 0 && (!own_text.is_empty() || !child_blocks.is_empty()) {
                append_separator_children(&mut child_blocks, pending_blank_lines);
            }
            let (mut nested_quote_blocks, consumed) = collect_quote_block(lines, index);
            if !nested_quote_blocks.is_empty()
                && nested_quote_blocks[0].kind == BlockKind::RawMarkdown
            {
                return None;
            }
            child_blocks.append(&mut nested_quote_blocks);
            saw_child = true;
            pending_blank_lines = 0;
            index = consumed;
            continue;
        }

        if parse_list_marker(line).is_some() {
            if pending_blank_lines > 0 && (!own_text.is_empty() || !child_blocks.is_empty()) {
                append_separator_children(&mut child_blocks, pending_blank_lines);
            }
            let (mut list_blocks, consumed) = collect_list_blocks(lines, index);
            if list_blocks
                .iter()
                .any(|block| block.kind == BlockKind::RawMarkdown)
            {
                return None;
            }
            child_blocks.append(&mut list_blocks);
            saw_child = true;
            pending_blank_lines = 0;
            index = consumed;
            continue;
        }

        if parse_opening_fence(line).is_some()
            && let Some((code_block, consumed)) = collect_fenced_code_block(lines, index)
        {
            if pending_blank_lines > 0 && (!own_text.is_empty() || !child_blocks.is_empty()) {
                append_separator_children(&mut child_blocks, pending_blank_lines);
            }
            child_blocks.push(code_block);
            saw_child = true;
            pending_blank_lines = 0;
            index = consumed;
            continue;
        }

        if starts_with_standalone_image_child_paragraph(&lines[index..]) {
            if pending_blank_lines > 0 && (!own_text.is_empty() || !child_blocks.is_empty()) {
                append_separator_children(&mut child_blocks, pending_blank_lines);
            }
            child_blocks.push(standalone_image_block(line.to_string()));
            saw_child = true;
            pending_blank_lines = 0;
            index += 1;
            continue;
        }

        if strip_indented_code_prefix(line).is_some()
            && let Some((code_block, consumed)) = collect_indented_code_block(lines, index)
        {
            if pending_blank_lines > 0 && (!own_text.is_empty() || !child_blocks.is_empty()) {
                append_separator_children(&mut child_blocks, pending_blank_lines);
            }
            child_blocks.push(code_block);
            saw_child = true;
            pending_blank_lines = 0;
            index = consumed;
            continue;
        }

        let mut paragraph_lines = vec![line.clone()];
        index += 1;
        while index < lines.len() {
            let next = &lines[index];
            if next.trim().is_empty()
                || is_quote_start(next)
                || parse_list_marker(next).is_some()
                || parse_opening_fence(next).is_some()
                || strip_indented_code_prefix(next).is_some()
                || quote_content_starts_unsupported(lines, index)
            {
                break;
            }

            paragraph_lines.push(next.clone());
            index += 1;
        }

        if is_standalone_image_paragraph(&paragraph_lines) {
            if pending_blank_lines > 0 && (!own_text.is_empty() || !child_blocks.is_empty()) {
                append_separator_children(&mut child_blocks, pending_blank_lines);
            }
            child_blocks.push(standalone_image_block(paragraph_lines.join("\n")));
            saw_child = true;
            pending_blank_lines = 0;
            continue;
        }

        if saw_child {
            if pending_blank_lines > 0 && (!own_text.is_empty() || !child_blocks.is_empty()) {
                append_separator_children(&mut child_blocks, pending_blank_lines);
            }
            child_blocks.push(native_block(
                BlockKind::Paragraph,
                &paragraph_lines.join("\n"),
            ));
            pending_blank_lines = 0;
            continue;
        }

        if !own_text.is_empty() {
            own_text.push_str(if pending_blank_lines > 0 {
                "\n\n"
            } else {
                "\n"
            });
        }
        own_text.push_str(&paragraph_lines.join("\n"));
        pending_blank_lines = 0;
    }

    if pending_blank_lines > 0 && (!own_text.is_empty() || !child_blocks.is_empty()) {
        append_separator_children(&mut child_blocks, pending_blank_lines);
    }

    let mut block = native_block(BlockKind::Blockquote, &own_text);
    attach_child_blocks(&mut block, &mut child_blocks);

    let mut result = vec![block];
    result.extend(child_blocks);
    Some(result)
}

pub(crate) fn build_native_callout_block(
    lines: &[String],
    variant: CalloutKind,
    text: String,
) -> Option<Vec<BlockData>> {
    let mut child_blocks = Vec::new();
    let mut index = 0usize;
    let mut pending_blank_lines = 0usize;

    while index < lines.len() {
        let line = &lines[index];
        if line.trim().is_empty() {
            pending_blank_lines += 1;
            index += 1;
            continue;
        }

        if pending_blank_lines > 0 {
            append_separator_children(&mut child_blocks, pending_blank_lines);
            pending_blank_lines = 0;
        }

        if is_table_candidate_line(line) {
            let table_end = collect_table_candidate_region(lines, index);
            let table_region = &lines[index..table_end];
            if let Some(table) = parse_table_region(table_region) {
                child_blocks.push(BlockData::table(table));
            } else {
                child_blocks.push(raw_block(table_region.join("\n")));
            }
            index = table_end;
            continue;
        }

        if is_footnote_definition_start(line) {
            let footnote_end = collect_footnote_definition_region(lines, index);
            if let Some(mut footnote_blocks) = build_native_footnote_definition_block(
                &lines[index..footnote_end],
                crate::parse::parser::ParseMode::Wysiwyg,
            ) {
                child_blocks.append(&mut footnote_blocks);
                index = footnote_end;
                continue;
            }
        }

        if let Some((comment, consumed)) = collect_comment_block(lines, index) {
            child_blocks.push(comment);
            index = consumed;
            continue;
        }

        if is_block_html_start(line) {
            let html_end = collect_block_html_region(lines, index);
            child_blocks.push(html_or_raw_block(lines[index..html_end].join("\n")));
            index = html_end;
            continue;
        }

        if is_display_math_start(line) {
            let math_end = collect_display_math_region(lines, index);
            child_blocks.push(math_or_raw_block(lines[index..math_end].join("\n")));
            index = math_end;
            continue;
        }

        if let Some(unsupported_end) = collect_unsupported_quote_region(lines, index) {
            child_blocks.push(raw_block(lines[index..unsupported_end].join("\n")));
            index = unsupported_end;
            continue;
        }

        if is_quote_start(line) {
            let (mut nested_quote_blocks, consumed) = collect_quote_block(lines, index);
            if !nested_quote_blocks.is_empty()
                && nested_quote_blocks[0].kind == BlockKind::RawMarkdown
            {
                return None;
            }
            child_blocks.append(&mut nested_quote_blocks);
            index = consumed;
            continue;
        }

        if parse_list_marker(line).is_some() {
            let (mut list_blocks, consumed) = collect_list_blocks(lines, index);
            if list_blocks
                .iter()
                .any(|block| block.kind == BlockKind::RawMarkdown)
            {
                return None;
            }
            child_blocks.append(&mut list_blocks);
            index = consumed;
            continue;
        }

        if parse_opening_fence(line).is_some()
            && let Some((code_block, consumed)) = collect_fenced_code_block(lines, index)
        {
            child_blocks.push(code_block);
            index = consumed;
            continue;
        }

        if starts_with_standalone_image_child_paragraph(&lines[index..]) {
            child_blocks.push(standalone_image_block(line.to_string()));
            index += 1;
            continue;
        }

        if strip_indented_code_prefix(line).is_some()
            && let Some((code_block, consumed)) = collect_indented_code_block(lines, index)
        {
            child_blocks.push(code_block);
            index = consumed;
            continue;
        }

        let mut paragraph_lines = vec![line.clone()];
        index += 1;
        while index < lines.len() {
            let next = &lines[index];
            if next.trim().is_empty()
                || is_quote_start(next)
                || parse_list_marker(next).is_some()
                || parse_opening_fence(next).is_some()
                || strip_indented_code_prefix(next).is_some()
                || quote_content_starts_unsupported(lines, index)
            {
                break;
            }

            paragraph_lines.push(next.clone());
            index += 1;
        }

        child_blocks.push(native_block(
            BlockKind::Paragraph,
            &paragraph_lines.join("\n"),
        ));
    }

    if pending_blank_lines > 0 {
        append_separator_children(&mut child_blocks, pending_blank_lines);
    }

    let mut block = BlockData::new(BlockKind::Callout(variant), BlockText::from_markdown(&text));
    attach_child_blocks(&mut block, &mut child_blocks);

    let mut result = vec![block];
    result.extend(child_blocks);
    Some(result)
}
